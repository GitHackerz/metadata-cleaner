# Metadata Cleaner

A cross-platform desktop application built with Rust and Iced that scans directories for files containing metadata (EXIF, XMP, IPTC, GPS, etc.) and strips that metadata using ExifTool — protecting your privacy before you share files.

## Features

- **Directory Scanning** — Recursively scan folders; progress updates in real time
- **Drag-and-Drop** — Drop a folder directly onto the window to start scanning
- **Folder Picker** — Click *Select Folder* to browse for a directory
- **Metadata Cleaning** — Removes all metadata tags via ExifTool; runs in background
- **Backup Before Clean** — Optionally keep `.original` copies of files before stripping
- **Cancellable Scans** — Abort an in-progress scan at any time
- **Scan Persistence** — Every scan is saved to a local SQLite database
- **History Page** — Browse all past scans; load any previous scan's file list
- **Export Reports** — Save scan results to JSON or CSV with timestamped filenames
- **Dark / Light Theme** — Toggle via the Settings panel; preference is persisted
- **Wide Format Support** — JPG, PNG, TIFF, WebP, GIF, PDF, DOCX, XLSX, PPTX, MP3, WAV, FLAC, MP4, MOV, AVI, MKV

## Prerequisites

**ExifTool must be installed and on your system `PATH`.**

| Platform | Command |
| --- | --- |
| macOS | `brew install exiftool` |
| Ubuntu / Debian | `sudo apt install libimage-exiftool-perl` |
| Fedora / RHEL | `sudo dnf install perl-Image-ExifTool` |
| Windows | Download from [exiftool.org](https://exiftool.org/), rename to `exiftool.exe`, add to `PATH` |

If ExifTool is not found, the app starts normally but the *Clean Metadata* button is disabled and a warning banner is shown.

## Building & Running

You need the [Rust toolchain](https://rustup.rs/) (stable, 1.75+).

```bash
# Clone
git clone https://github.com/YOUR_USERNAME/metadata-cleaner.git
cd metadata-cleaner

# Development build
cargo run

# Optimised release build
cargo run --release
```

## Running Tests

```bash
cargo test
```

Tests cover: domain model serialization, SQLite CRUD operations, scanner extension filtering, and JSON/CSV export correctness.

## Usage

1. **Open** — Launch the app. The Scanner page opens with a prompt.
2. **Select** — Drag a folder onto the window, or click **Select Folder**.
3. **Scan** — The app walks the directory tree and lists all supported files.
4. **Review** — Inspect file paths, types, and current statuses in the list.
5. **Clean** — Click **Clean Metadata**. Each file is processed in the background; the list updates as files are cleaned.
6. **Export** — Click **Export Report** to save a JSON or CSV report.
7. **History** — Switch to the **History** tab to see all previous scans and load their file lists.

## Architecture

```text
src/
├── main.rs                 Entry point — logger init, Iced app launch
├── backend/
│   ├── error.rs            Unified AppError enum (thiserror)
│   ├── models.rs           Domain types: ScanRecord, FileRecord, UserPreferences
│   ├── db.rs               SQLite persistence (WAL mode, indexed)
│   ├── scanner.rs          Async directory walker with cancellation
│   ├── exiftool.rs         ExifTool CLI wrapper (read, clean, backup)
│   ├── export.rs           JSON / CSV report generation
│   └── logger.rs           env_logger initialisation
└── frontend/
    ├── main.rs             Iced Application state machine
    ├── ui.rs               Shared primitives (header, primary_button)
    ├── components/
    │   ├── file_list.rs    Scrollable file list with status colours
    │   ├── progress_bar.rs Dual progress bars (scan + clean)
    │   └── settings_panel.rs Recursive / backup / theme toggles
    └── pages/
        ├── scanner_page.rs Main scanning view
        └── history_page.rs Historical scans browser
```

**Communication pattern**: All backend calls are made directly from Iced's `update()` method (single-threaded event loop). Heavy work (ExifTool subprocess calls, directory walks) runs in `tokio::task::spawn_blocking`. Results are streamed back to the UI via `tokio::sync::mpsc` channels, polled at 16ms intervals.

## Technology Stack

| Crate | Purpose |
| --- | --- |
| `iced` 0.12 | GUI framework (Elm-architecture) |
| `tokio` | Async runtime |
| `rusqlite` (bundled) | SQLite database — no system dependency |
| `walkdir` | Directory traversal |
| `rfd` | Native async file dialogs |
| `serde` + `serde_json` + `csv` | Serialisation and export |
| `chrono` | Timestamps |
| `uuid` | Unique record IDs |
| `thiserror` | Structured error types |
| `log` + `env_logger` | Logging |

## Privacy

This app removes metadata tags (EXIF, XMP, IPTC, GPS, etc.) from supported file types using ExifTool. It does not alter image pixels, remove embedded text in images, or handle steganographically hidden data. Always review files manually for sensitive content before sharing.
