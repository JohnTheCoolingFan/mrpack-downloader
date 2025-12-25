"""
Copyright (C) 2024 Fern Lane, curseforge-modpack-downloader

Licensed under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License.
You may obtain a copy of the License at

    http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
See the License for the specific language governing permissions and
limitations under the License.

IN NO EVENT SHALL THE AUTHOR BE LIABLE FOR ANY CLAIM, DAMAGES OR
OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
OTHER DEALINGS IN THE SOFTWARE.
"""

import glob
import hashlib
import io
import json
import logging
import os
import queue
import shutil
import ssl
import tempfile
import threading
import time
import tkinter as tk
import zipfile
from tkinter import filedialog, messagebox, scrolledtext, ttk
from urllib import request

from _version import __version__

# Constants from main.py
USER_AGENT_DEFAULT = (
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
    "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36"
)

INFO_URL = "https://api.cfwidget.com/{project_id}"
DOWNLOAD_URL = "https://www.curseforge.com/api/v1/mods/{project_id}/files/{file_id}/download"
FORGE_URL = "https://maven.minecraftforge.net/net/minecraftforge/forge/{game_version}-{forge_version}/forge-{game_version}-{forge_version}-installer.jar"
FORGE_URL_OLD = "https://maven.minecraftforge.net/net/minecraftforge/forge/{game_version}-{forge_version}-{game_version}/forge-{game_version}-{forge_version}-{game_version}-installer.jar"
FABRIC_URL = "https://maven.fabricmc.net/net/fabricmc/fabric-installer/1.0.1/fabric-installer-1.0.1.jar"
FABRIC_FILE_NAME = "fabric-installer-1.0.1.jar"

MANIFEST_FILE = "manifest.json"
MODLIST_FILE = "modlist.html"

CHECKSUM_BUFFER_SIZE = 2048
FILE_DOWNLOAD_MAX_ATTEMPTS = 3

# Colors for modern UI
COLORS = {
    "bg": "#1e1e2e",
    "bg_secondary": "#313244",
    "bg_hover": "#45475a",
    "accent": "#89b4fa",
    "accent_hover": "#b4befe",
    "text": "#cdd6f4",
    "text_secondary": "#a6adc8",
    "success": "#a6e3a1",
    "success_hover": "#94e2d5",
    "warning": "#f9e2af",
    "error": "#f38ba8",
    "error_hover": "#eba0ac",
    "border": "#45475a",
}


class QueueHandler(logging.Handler):
    """Custom logging handler that puts log records into a queue"""

    def __init__(self, log_queue):
        super().__init__()
        self.log_queue = log_queue

    def emit(self, record):
        self.log_queue.put(self.format(record))


class ModernButton(tk.Button):
    """Custom styled button"""

    def __init__(self, master, **kwargs):
        self.default_bg = kwargs.pop("bg", COLORS["accent"])
        self.hover_bg = kwargs.pop("hover_bg", COLORS["accent_hover"])

        super().__init__(
            master,
            bg=self.default_bg,
            fg=COLORS["bg"],
            activebackground=self.hover_bg,
            activeforeground=COLORS["bg"],
            relief="flat",
            font=("Segoe UI", 10, "bold"),
            cursor="hand2",
            padx=15,
            pady=8,
            **kwargs,
        )
        self.bind("<Enter>", self._on_enter)
        self.bind("<Leave>", self._on_leave)

    def _on_enter(self, e):
        if self["state"] != "disabled":
            self["bg"] = self.hover_bg

    def _on_leave(self, e):
        if self["state"] != "disabled":
            self["bg"] = self.default_bg


class ModernEntry(tk.Entry):
    """Custom styled entry"""

    def __init__(self, master, **kwargs):
        super().__init__(
            master,
            bg=COLORS["bg_secondary"],
            fg=COLORS["text"],
            insertbackground=COLORS["text"],
            relief="flat",
            font=("Segoe UI", 10),
            highlightthickness=2,
            highlightbackground=COLORS["border"],
            highlightcolor=COLORS["accent"],
            **kwargs,
        )


class CurseForgeDownloaderGUI:
    """Main GUI application"""

    def __init__(self, root):
        self.root = root
        self.root.title(f"CurseForge Modpack Downloader v{__version__}")
        self.root.geometry("800x700")
        self.root.minsize(700, 650)
        self.root.configure(bg=COLORS["bg"])

        # Set icon if available (Windows only)
        if os.name == "nt":
            try:
                self.root.iconbitmap(default="")
            except tk.TclError:
                pass  # Icon not available, continue without it

        # Initialize variables
        self.download_thread = None
        self.is_downloading = False
        self.log_queue = queue.Queue()
        self.progress_queue = queue.Queue()

        # Setup logging
        self.setup_logging()

        # Create UI
        self.create_widgets()

        # Start log and progress updater
        self.update_log()
        self.update_progress()

    def setup_logging(self):
        """Setup logging to use queue handler"""
        self.logger = logging.getLogger("curseforge_gui")
        self.logger.setLevel(logging.INFO)

        # Clear existing handlers
        self.logger.handlers = []

        # Add queue handler
        handler = QueueHandler(self.log_queue)
        handler.setFormatter(logging.Formatter("[%(asctime)s] [%(levelname)s] %(message)s", datefmt="%H:%M:%S"))
        self.logger.addHandler(handler)

    def create_widgets(self):
        """Create all UI widgets"""
        # Main container with padding
        main_frame = tk.Frame(self.root, bg=COLORS["bg"])
        main_frame.pack(fill="both", expand=True, padx=20, pady=20)

        # Title
        title_frame = tk.Frame(main_frame, bg=COLORS["bg"])
        title_frame.pack(fill="x", pady=(0, 5))

        title_label = tk.Label(
            title_frame,
            text="🟧 CurseForge Modpack Downloader",
            font=("Segoe UI", 18, "bold"),
            fg=COLORS["accent"],
            bg=COLORS["bg"],
        )
        title_label.pack(side="left")

        version_label = tk.Label(
            title_frame,
            text=f"v{__version__}",
            font=("Segoe UI", 10),
            fg=COLORS["text_secondary"],
            bg=COLORS["bg"],
        )
        version_label.pack(side="left", padx=(10, 0), pady=(8, 0))

        # Subtitle/Description
        subtitle_label = tk.Label(
            main_frame,
            text="Download and update your favorite Minecraft modpacks easily",
            font=("Segoe UI", 10),
            fg=COLORS["text_secondary"],
            bg=COLORS["bg"],
        )
        subtitle_label.pack(anchor="w", pady=(0, 15))

        # Input section
        input_frame = tk.LabelFrame(
            main_frame,
            text=" Input ",
            font=("Segoe UI", 11, "bold"),
            fg=COLORS["text"],
            bg=COLORS["bg"],
            relief="flat",
            highlightthickness=1,
            highlightbackground=COLORS["border"],
        )
        input_frame.pack(fill="x", pady=(0, 15))

        # Modpack path/URL
        source_frame = tk.Frame(input_frame, bg=COLORS["bg"])
        source_frame.pack(fill="x", padx=15, pady=(15, 10))

        source_label = tk.Label(
            source_frame,
            text="Modpack ZIP or URL:",
            font=("Segoe UI", 10),
            fg=COLORS["text"],
            bg=COLORS["bg"],
        )
        source_label.pack(anchor="w")

        source_entry_frame = tk.Frame(source_frame, bg=COLORS["bg"])
        source_entry_frame.pack(fill="x", pady=(5, 0))

        self.source_var = tk.StringVar()
        self.source_entry = ModernEntry(source_entry_frame, textvariable=self.source_var)
        self.source_entry.pack(side="left", fill="x", expand=True, ipady=5)

        browse_source_btn = ModernButton(
            source_entry_frame,
            text="Browse",
            command=self.browse_source,
            bg=COLORS["bg_secondary"],
            hover_bg=COLORS["bg_hover"],
        )
        browse_source_btn.pack(side="right", padx=(10, 0))

        source_hint = tk.Label(
            source_frame,
            text="💡 Enter path to downloaded ZIP file or direct download URL from CurseForge",
            font=("Segoe UI", 9),
            fg=COLORS["text_secondary"],
            bg=COLORS["bg"],
        )
        source_hint.pack(anchor="w", pady=(5, 0))

        # Destination path
        dest_frame = tk.Frame(input_frame, bg=COLORS["bg"])
        dest_frame.pack(fill="x", padx=15, pady=(10, 15))

        dest_label = tk.Label(
            dest_frame,
            text="Destination Folder:",
            font=("Segoe UI", 10),
            fg=COLORS["text"],
            bg=COLORS["bg"],
        )
        dest_label.pack(anchor="w")

        dest_entry_frame = tk.Frame(dest_frame, bg=COLORS["bg"])
        dest_entry_frame.pack(fill="x", pady=(5, 0))

        self.dest_var = tk.StringVar()
        self.dest_entry = ModernEntry(dest_entry_frame, textvariable=self.dest_var)
        self.dest_entry.pack(side="left", fill="x", expand=True, ipady=5)

        browse_dest_btn = ModernButton(
            dest_entry_frame,
            text="Browse",
            command=self.browse_destination,
            bg=COLORS["bg_secondary"],
            hover_bg=COLORS["bg_hover"],
        )
        browse_dest_btn.pack(side="right", padx=(10, 0))

        dest_hint = tk.Label(
            dest_frame,
            text="💡 Select your Minecraft version folder (e.g., .minecraft/versions/MyModPack/)",
            font=("Segoe UI", 9),
            fg=COLORS["text_secondary"],
            bg=COLORS["bg"],
        )
        dest_hint.pack(anchor="w", pady=(5, 0))

        # Options section
        options_frame = tk.LabelFrame(
            main_frame,
            text=" Options (for existing config files) ",
            font=("Segoe UI", 11, "bold"),
            fg=COLORS["text"],
            bg=COLORS["bg"],
            relief="flat",
            highlightthickness=1,
            highlightbackground=COLORS["border"],
        )
        options_frame.pack(fill="x", pady=(0, 15))

        options_inner = tk.Frame(options_frame, bg=COLORS["bg"])
        options_inner.pack(fill="x", padx=15, pady=15)

        self.conflict_var = tk.StringVar(value="rename")

        options_data = [
            ("skip", "Skip existing", "Keep your current config files unchanged"),
            ("rename", "Backup existing (recommended)", "Create .old backup before replacing"),
            ("overwrite", "Overwrite all", "Replace all config files with new versions"),
        ]

        for i, (value, text, desc) in enumerate(options_data):
            opt_frame = tk.Frame(options_inner, bg=COLORS["bg"])
            opt_frame.grid(row=i // 2, column=i % 2, sticky="w", padx=10, pady=5)

            rb = tk.Radiobutton(
                opt_frame,
                text=text,
                variable=self.conflict_var,
                value=value,
                font=("Segoe UI", 10),
                fg=COLORS["text"],
                bg=COLORS["bg"],
                selectcolor=COLORS["bg_secondary"],
                activebackground=COLORS["bg"],
                activeforeground=COLORS["accent"],
                highlightthickness=0,
            )
            rb.pack(anchor="w")

            desc_label = tk.Label(
                opt_frame,
                text=desc,
                font=("Segoe UI", 8),
                fg=COLORS["text_secondary"],
                bg=COLORS["bg"],
            )
            desc_label.pack(anchor="w", padx=(20, 0))

        # Progress section
        progress_frame = tk.Frame(main_frame, bg=COLORS["bg"])
        progress_frame.pack(fill="x", pady=(0, 15))

        # Progress bar style
        style = ttk.Style()
        style.theme_use("clam")
        style.configure(
            "Custom.Horizontal.TProgressbar",
            troughcolor=COLORS["bg_secondary"],
            background=COLORS["accent"],
            borderwidth=0,
            lightcolor=COLORS["accent"],
            darkcolor=COLORS["accent"],
        )

        self.progress_var = tk.DoubleVar(value=0)
        self.progress_bar = ttk.Progressbar(
            progress_frame,
            variable=self.progress_var,
            maximum=100,
            style="Custom.Horizontal.TProgressbar",
            length=300,
        )
        self.progress_bar.pack(fill="x")

        self.progress_label = tk.Label(
            progress_frame,
            text="🟢 Ready - Click START DOWNLOAD to begin",
            font=("Segoe UI", 9),
            fg=COLORS["text_secondary"],
            bg=COLORS["bg"],
        )
        self.progress_label.pack(anchor="w", pady=(5, 0))

        # Buttons section (placed before log so always visible)
        button_frame = tk.Frame(main_frame, bg=COLORS["bg"])
        button_frame.pack(fill="x", pady=(15, 15))

        # Create a grid layout for better button placement
        button_frame.columnconfigure(0, weight=1)
        button_frame.columnconfigure(1, weight=2)
        button_frame.columnconfigure(2, weight=1)

        # Left side - Clear Log button
        clear_btn = ModernButton(
            button_frame,
            text="🗑️  Clear Log",
            command=self.clear_log,
            bg=COLORS["bg_secondary"],
            hover_bg=COLORS["bg_hover"],
        )
        clear_btn.grid(row=0, column=0, sticky="w")

        # Center - Main Start/Download Button (prominent)
        self.download_btn = ModernButton(
            button_frame,
            text="▶  START DOWNLOAD",
            command=self.start_download,
            bg=COLORS["success"],
            hover_bg=COLORS["success_hover"],
        )
        self.download_btn.config(font=("Segoe UI", 12, "bold"), padx=30, pady=12)
        self.download_btn.grid(row=0, column=1)

        # Right side - Cancel button
        self.cancel_btn = ModernButton(
            button_frame,
            text="⏹️  Cancel",
            command=self.cancel_download,
            bg=COLORS["error"],
            hover_bg=COLORS["error_hover"],
        )
        self.cancel_btn.grid(row=0, column=2, sticky="e")
        self.cancel_btn.config(state="disabled")

        # Log section (at the bottom, expands to fill remaining space)
        log_frame = tk.LabelFrame(
            main_frame,
            text=" Log ",
            font=("Segoe UI", 11, "bold"),
            fg=COLORS["text"],
            bg=COLORS["bg"],
            relief="flat",
            highlightthickness=1,
            highlightbackground=COLORS["border"],
        )
        log_frame.pack(fill="both", expand=True)

        self.log_text = scrolledtext.ScrolledText(
            log_frame,
            bg=COLORS["bg_secondary"],
            fg=COLORS["text"],
            font=("Consolas", 9),
            relief="flat",
            insertbackground=COLORS["text"],
            highlightthickness=0,
            state="disabled",
        )
        self.log_text.pack(fill="both", expand=True, padx=10, pady=10)

        # Configure log tags for colored output
        self.log_text.tag_configure("INFO", foreground=COLORS["text"])
        self.log_text.tag_configure("WARNING", foreground=COLORS["warning"])
        self.log_text.tag_configure("ERROR", foreground=COLORS["error"])
        self.log_text.tag_configure("SUCCESS", foreground=COLORS["success"])

    def browse_source(self):
        """Open file dialog to select source ZIP"""
        filepath = filedialog.askopenfilename(
            title="Select Modpack ZIP File",
            filetypes=[("ZIP files", "*.zip"), ("All files", "*.*")],
        )
        if filepath:
            self.source_var.set(filepath)

    def browse_destination(self):
        """Open folder dialog to select destination"""
        folderpath = filedialog.askdirectory(title="Select Destination Folder")
        if folderpath:
            self.dest_var.set(folderpath)

    def clear_log(self):
        """Clear the log text widget"""
        self.log_text.config(state="normal")
        self.log_text.delete(1.0, tk.END)
        self.log_text.config(state="disabled")

    def log_message(self, message, level="INFO"):
        """Add a message to the log queue"""
        self.log_queue.put(f"[{level}] {message}")

    def update_log(self):
        """Update log text widget from queue"""
        try:
            while True:
                message = self.log_queue.get_nowait()
                self.log_text.config(state="normal")

                # Determine tag based on message content
                tag = "INFO"
                if "[WARNING]" in message:
                    tag = "WARNING"
                elif "[ERROR]" in message:
                    tag = "ERROR"
                elif "downloaded" in message.lower() or "finished" in message.lower():
                    tag = "SUCCESS"

                self.log_text.insert(tk.END, message + "\n", tag)
                self.log_text.see(tk.END)
                self.log_text.config(state="disabled")
        except queue.Empty:
            pass
        finally:
            self.root.after(100, self.update_log)

    def update_progress(self):
        """Update progress bar from queue"""
        try:
            while True:
                progress_data = self.progress_queue.get_nowait()
                if isinstance(progress_data, tuple):
                    value, text = progress_data
                    self.progress_var.set(value)
                    self.progress_label.config(text=text)
                elif isinstance(progress_data, str):
                    self.progress_label.config(text=progress_data)
        except queue.Empty:
            pass
        finally:
            self.root.after(100, self.update_progress)

    def start_download(self):
        """Start the download process in a separate thread"""
        source = self.source_var.get().strip()
        destination = self.dest_var.get().strip()

        if not source:
            messagebox.showerror("Error", "Please enter a modpack ZIP path or URL")
            return

        if not destination:
            messagebox.showerror("Error", "Please select a destination folder")
            return

        self.is_downloading = True
        self.download_btn.config(state="disabled", text="⏳  DOWNLOADING...")
        self.cancel_btn.config(state="normal")
        self.progress_var.set(0)

        self.download_thread = threading.Thread(target=self.download_worker, args=(source, destination), daemon=True)
        self.download_thread.start()

    def cancel_download(self):
        """Cancel the current download"""
        self.is_downloading = False
        self.logger.warning("Download cancelled by user")
        self.progress_queue.put((0, "⚠️ Cancelled"))
        self.download_btn.config(state="normal", text="▶  START DOWNLOAD")
        self.cancel_btn.config(state="disabled")

    def download_finished(self, success=True, message=""):
        """Called when download is complete"""
        self.is_downloading = False
        self.download_btn.config(state="normal", text="▶  START DOWNLOAD")
        self.cancel_btn.config(state="disabled")

        if success:
            self.progress_queue.put((100, "✅ Complete!"))
            messagebox.showinfo("Success", message or "Modpack downloaded successfully!")
        else:
            self.progress_queue.put((0, "❌ Failed"))
            if message:
                messagebox.showerror("Error", message)

    def download_worker(self, source, destination):
        """Worker thread for downloading"""
        try:
            # Fix SSL
            try:
                self.logger.info("Applying SSL certificates fix")
                ssl._create_default_https_context = ssl._create_unverified_context
            except Exception as e:
                self.logger.warning(f"Unable to apply SSL certificates fix: {e}")

            conflict_mode = self.conflict_var.get()

            with tempfile.TemporaryDirectory() as temp_dir:
                self.logger.info(f"Working directory: {temp_dir}")
                self.progress_queue.put((5, "Downloading/Opening modpack..."))

                if not self.is_downloading:
                    return

                # Download or open archive
                archive = self._open_or_download(source, temp_dir)

                try:
                    if not self.is_downloading:
                        return

                    # Extract and parse manifest
                    self.progress_queue.put((10, "Extracting archive..."))
                    manifest = self._unzip_modpack(archive, temp_dir)
                finally:
                    archive.close()

                if not self.is_downloading:
                    return

                # Create destination if needed
                if not os.path.exists(destination):
                    self.logger.info(f"Creating directory {destination}")
                    os.makedirs(destination)

                # Copy manifest and modlist
                self.logger.info(f"Copying {MANIFEST_FILE}")
                shutil.copyfile(os.path.join(temp_dir, MANIFEST_FILE), os.path.join(destination, MANIFEST_FILE))
                self.logger.info(f"Copying {MODLIST_FILE}")
                shutil.copyfile(os.path.join(temp_dir, MODLIST_FILE), os.path.join(destination, MODLIST_FILE))

                # Download files
                files_total = len(manifest["files"])
                for i, file in enumerate(manifest["files"]):
                    if not self.is_downloading:
                        return

                    progress = 15 + (i / files_total) * 70
                    self.progress_queue.put((progress, f"Downloading file {i + 1}/{files_total}..."))

                    self._download_file(file, destination, i, files_total)

                if not self.is_downloading:
                    return

                # Handle overrides
                self.progress_queue.put((90, "Processing overrides..."))
                self._handle_overrides(manifest, temp_dir, destination, conflict_mode)

                if not self.is_downloading:
                    return

                # Download mod loader
                self.progress_queue.put((95, "Downloading mod loader..."))
                self._download_mod_loader(manifest, destination)

                # Done
                self.logger.info(f"{manifest['name']} v{manifest['version']} downloaded")
                self.logger.info("curseforge-modpack-downloader finished")

                self.root.after(0, lambda: self.download_finished(True, f"{manifest['name']} v{manifest['version']} downloaded successfully!"))

        except Exception as e:
            self.logger.error(f"Error: {str(e)}")
            self.root.after(0, lambda: self.download_finished(False, str(e)))

    def _open_or_download(self, zip_or_url, temp_dir):
        """Opens provided zip file or downloads as temp file"""
        if os.path.exists(zip_or_url):
            return open(zip_or_url, "rb")

        out_file = tempfile.TemporaryFile(dir=temp_dir)
        self.logger.info(f"Downloading {zip_or_url}")
        with request.urlopen(zip_or_url) as response:
            shutil.copyfileobj(response, out_file)
        return out_file

    def _unzip_modpack(self, archive, temp_dir):
        """Unzips archive and parses manifest"""
        self.logger.info("Extracting archive")
        with zipfile.ZipFile(archive, "r") as zip_ref:
            zip_ref.extractall(temp_dir)

        self.logger.info("Trying to read and parse manifest.json")
        with open(os.path.join(temp_dir, MANIFEST_FILE), "r", encoding="utf-8") as file:
            manifest = json.loads(file.read())

        if not manifest:
            raise Exception("Manifest read error")
        if "name" not in manifest:
            raise Exception('No "name" key in manifest')
        if "version" not in manifest:
            raise Exception('No "version" key in manifest')
        if "files" not in manifest:
            raise Exception('No "files" key in manifest')

        self.logger.info(f"Found {len(manifest['files'])} files")
        return manifest

    def _download_file(self, file, destination, index, total):
        """Download a single mod file"""
        project_id = file["projectID"]
        file_id = file["fileID"]

        url = INFO_URL.format(project_id=project_id)
        headers = {"content-type": "application/json", "User-Agent": USER_AGENT_DEFAULT}
        self.logger.info(f"[{index + 1}/{total}] Retrieving project information from {url}")

        try:
            request_ = request.Request(url, headers=headers)
            project_info = json.loads(request.urlopen(request_).read().decode("utf-8"))
        except Exception as e:
            self.logger.warning(f"[{index + 1}/{total}] Unable to get project info: {e}. Skipping project {project_id}")
            return

        if not project_info or "title" not in project_info or "type" not in project_info or "files" not in project_info:
            self.logger.warning(f"[{index + 1}/{total}] Unable to parse project info. Skipping project {project_id}")
            return

        # Convert type to directory
        type_ = project_info["type"]
        if type_ == "Mods":
            root_dir = "mods"
        elif type_ == "Resource Packs":
            root_dir = "resourcepacks"
        elif type_ == "Shaders":
            root_dir = "shaderpacks"
        else:
            root_dir = "mods"
            self.logger.warning(f"[{index + 1}/{total}] Unknown project type {type_}. Considering as mod...")

        destination_dir = os.path.join(destination, root_dir)

        # Find file info
        file_size = 0
        file_to_download_name = None
        files_to_delete = []

        for file_ in project_info["files"]:
            if file_["id"] == file_id:
                if not file_to_download_name:
                    file_to_download_name = file_["name"]
                    file_size = file_["filesize"]

                    dest_file = os.path.join(destination_dir, file_to_download_name)
                    if os.path.exists(dest_file) and os.path.getsize(dest_file) == file_size:
                        break
            else:
                file_to_delete = os.path.join(destination_dir, file_["name"])
                if os.path.exists(file_to_delete):
                    files_to_delete.append(file_to_delete)

        if not file_to_download_name:
            self.logger.warning(f"[{index + 1}/{total}] No file {file_id}. Skipping project {project_id}")
            return

        dest_path = os.path.join(destination_dir, file_to_download_name)

        if os.path.exists(dest_path):
            self.logger.info(f"[{index + 1}/{total}] File {file_to_download_name} already exists")
            return

        if not os.path.exists(destination_dir):
            self.logger.info(f"[{index + 1}/{total}] Creating directory {destination_dir}")
            os.makedirs(destination_dir)

        download_url = DOWNLOAD_URL.format(project_id=project_id, file_id=file_id)

        attempts = 0
        while True:
            try:
                attempts += 1
                self.logger.info(f"[{index + 1}/{total}] Downloading {download_url} as {file_to_download_name}")
                with request.urlopen(download_url) as response, open(dest_path, "w+b") as out_file:
                    shutil.copyfileobj(response, out_file)
                    out_file.flush()
                    os.fsync(out_file.fileno())
                if not os.path.exists(dest_path) or os.path.getsize(dest_path) != file_size:
                    raise Exception("File doesn't exist or has wrong size")
                break
            except Exception as e:
                self.logger.error(f"Unable to download {download_url}: {e}")

            if attempts >= FILE_DOWNLOAD_MAX_ATTEMPTS:
                raise Exception(f"Unable to download file in {FILE_DOWNLOAD_MAX_ATTEMPTS} attempts")

            time.sleep(1)
            self.logger.warning("Retrying...")

        # Delete previous versions
        for file_to_delete in files_to_delete:
            self.logger.warning(f"[{index + 1}/{total}] Deleting previous version {os.path.basename(file_to_delete)}")
            os.remove(file_to_delete)

    def _file_md5(self, file_path):
        """Calculate MD5 checksum"""
        md5 = hashlib.md5()
        with open(file_path, "rb") as file:
            data = file.read(CHECKSUM_BUFFER_SIZE)
            while len(data) > 0:
                md5.update(data)
                data = file.read(CHECKSUM_BUFFER_SIZE)
        return md5.hexdigest()

    def _file_copy(self, source, dest, rewrite):
        """Copy file with optional backup"""
        try:
            dest_dir = os.path.dirname(dest)
            if not os.path.exists(dest_dir):
                self.logger.info(f"Creating directory {dest_dir}")
                os.makedirs(dest_dir)

            if not rewrite and os.path.exists(dest):
                very_old_files = sorted(glob.glob(f"{dest}.old*"), reverse=True)
                for very_old_file in very_old_files:
                    self.logger.info(f"Renaming {very_old_file} into {very_old_file}.old")
                    shutil.move(very_old_file, f"{very_old_file}.old")

                self.logger.info(f"Renaming {dest} into {dest}.old")
                shutil.move(dest, f"{dest}.old")

            self.logger.info(f"Copying {source} into {dest}")
            shutil.copyfile(source, dest)
        except Exception as e:
            self.logger.error(f"Unable to copy {source} into {dest}. Please copy files manually: {e}")

    def _handle_overrides(self, manifest, temp_dir, destination, conflict_mode):
        """Handle override files"""
        overrides = manifest.get("overrides")
        if not overrides:
            return

        overrides_dir = os.path.join(temp_dir, overrides)

        override_file_paths = []
        for root, _, f_names in os.walk(overrides_dir):
            for override_file_path in f_names:
                override_file_paths.append(os.path.join(root, override_file_path))

        for override_file in override_file_paths:
            if not self.is_downloading:
                return

            override_file_rel = os.path.relpath(override_file, overrides_dir)
            override_file_target = os.path.join(destination, override_file_rel)

            if not os.path.exists(override_file_target):
                self._file_copy(override_file, override_file_target, rewrite=True)
                continue

            checksum_src = self._file_md5(override_file)
            checksum_target = self._file_md5(override_file_target)

            if checksum_src == checksum_target:
                self.logger.info(f"Skipping file {override_file_rel}. Already exists")
                continue

            # Apply conflict mode
            if conflict_mode == "skip":
                self.logger.info(f"Skipping file {override_file_rel}")
            elif conflict_mode == "overwrite":
                self._file_copy(override_file, override_file_target, rewrite=True)
            else:
                # Default: rename (backup)
                self._file_copy(override_file, override_file_target, rewrite=False)

    def _download_mod_loader(self, manifest, destination):
        """Download Forge or Fabric mod loader"""
        if (
            "minecraft" not in manifest
            or "version" not in manifest["minecraft"]
            or "modLoaders" not in manifest["minecraft"]
            or len(manifest["minecraft"]["modLoaders"]) == 0
        ):
            return

        mod_loader_id = manifest["minecraft"]["modLoaders"][0]["id"]

        if mod_loader_id.startswith("forge-"):
            game_version = manifest["minecraft"]["version"]
            if len(game_version.split(".")) > 1 and int(game_version.split(".")[1].strip()) < 8:
                download_url = FORGE_URL_OLD.format(game_version=game_version, forge_version=mod_loader_id[6:])
            else:
                download_url = FORGE_URL.format(game_version=game_version, forge_version=mod_loader_id[6:])

            dest_name = download_url.split("/")[-1].strip()
            if not dest_name:
                dest_name = download_url.split("/")[-2].strip()
            dest_path = os.path.join(destination, dest_name)
            self.logger.info(f"Downloading {download_url} as {dest_path}")

            request_ = request.Request(download_url, headers={"User-Agent": USER_AGENT_DEFAULT})
            with request.urlopen(request_) as response, open(dest_path, "w+b") as out_file:
                shutil.copyfileobj(response, out_file)

            self.logger.info(f'Forge {mod_loader_id[6:]} downloaded. Please run java -jar "{dest_path}" to install')

        elif mod_loader_id.startswith("fabric"):
            dest_path = os.path.join(destination, FABRIC_FILE_NAME)
            self.logger.info(f"Downloading {FABRIC_URL} as {dest_path}")

            request_ = request.Request(FABRIC_URL, headers={"User-Agent": USER_AGENT_DEFAULT})
            with request.urlopen(request_) as response, open(dest_path, "w+b") as out_file:
                shutil.copyfileobj(response, out_file)

            self.logger.info(f'Fabric installer downloaded. Please run java -jar "{dest_path}" to install')

        else:
            self.logger.warning(f"Please download {mod_loader_id} mod loader manually")


def main():
    """Main entry point for GUI"""
    root = tk.Tk()
    app = CurseForgeDownloaderGUI(root)
    root.mainloop()


if __name__ == "__main__":
    main()
