# Metadata Cleaner / Privacy Scanner

A cross-platform desktop application built with Rust and Iced to scan and clean metadata from various file types to protect your privacy.

## Features

-   **Deep Scanning**: Recursively scans folders for files with metadata.
-   **Privacy Protection**: Removes metadata (EXIF, XMP, IPTC, etc.) using ExifTool.
-   **Wide Format Support**: Supports Images, PDFs, Office Documents, and Audio/Video files.
-   **Safety First**: Optional backup of original files before cleaning.
-   **Modern UI**: Clean, dark-mode enabled interface with drag-and-drop support.
-   **History Tracking**: Keeps a local history of scans and cleaned files.
-   **Reporting**: Export scan results to JSON or CSV.

## Prerequisites

This application requires **ExifTool** to be installed on your system.

-   **Windows**: Download the executable from [exiftool.org](https://exiftool.org/), rename it to `exiftool.exe`, and add it to your system `PATH`.
-   **macOS**: `brew install exiftool`
-   **Linux**: `sudo apt install libimage-exiftool-perl`

## Installation & Running

1.  **Clone the repository**:
    ```bash
    git clone https://github.com/yourusername/metadata-cleaner.git
    cd metadata-cleaner
    ```

2.  **Build and Run**:
    ```bash
    cargo run --release
    ```

## Usage

1.  **Scan**: Drag and drop a folder onto the window or use the "Scan Folder" button (if configured).
2.  **Review**: See the list of files and their status.
3.  **Clean**: Click "Clean Metadata" to remove sensitive information.
4.  **Export**: Save a report of the scan for your records.

## Technologies

-   **Rust**: Core language.
-   **Iced**: GUI framework.
-   **Tokio**: Async runtime.
-   **Rusqlite**: SQLite database storage.
-   **ExifTool**: Metadata engine.

## License

MIT
