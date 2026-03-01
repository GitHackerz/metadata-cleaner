use chrono::Local;
use iced::executor;
use iced::widget::{button, column, row};
use iced::{Application, Command, Element, Subscription, Theme};
use log::{error, info, warn};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{mpsc, watch};
use uuid::Uuid;

use crate::backend::db::Database;
use crate::backend::exiftool::ExifTool;
use crate::backend::models::{FileRecord, FileStatus, ScanRecord, ScanStatus, UserPreferences};
use crate::backend::scanner::{Scanner, ScannerMessage};
use crate::frontend::components::{file_list, settings_panel};
use crate::frontend::pages::{history_page, scanner_page};

#[derive(Debug, Clone)]
pub struct CleanResult {
    pub file_id: String,
    pub success: bool,
    pub error_msg: Option<String>,
    pub total_cleaned: i32,
    pub total_remaining: i32,
}

pub struct MetadataCleanerApp {
    db: Arc<Database>,
    current_page: Page,
    files: Vec<FileRecord>,
    history: Vec<ScanRecord>,
    preferences: UserPreferences,
    scanning: bool,
    cleaning: bool,
    progress: f32,
    clean_progress: f32,
    status_text: String,
    selected_file_id: Option<String>,
    exiftool_available: bool,
    is_wsl: bool,
    current_scan_id: Option<String>,
    path_input: String,
    scan_rx: Option<mpsc::Receiver<ScannerMessage>>,
    cancel_tx: Option<watch::Sender<bool>>,
    clean_rx: Option<mpsc::Receiver<CleanResult>>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Page {
    Scanner,
    History,
}

#[derive(Debug, Clone)]
pub enum Message {
    ScannerPage(scanner_page::ScannerPageMessage),
    HistoryPage(history_page::HistoryPageMessage),
    SwitchPage(Page),
    FileDropped(PathBuf),
    ScanProgress(ScannerMessageWrapper),
    ScanFinished,
    ScanTick,
    CleanTick,
    ExportComplete(String),
    DirectorySelected(Option<PathBuf>),
    CancelScan,
}

#[derive(Debug, Clone)]
pub enum ScannerMessageWrapper {
    FoundFile(FileRecord),
    Completed(i32),
    Error(String),
}

impl Application for MetadataCleanerApp {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let db_path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("metadata_cleaner.db");

        #[allow(clippy::arc_with_non_send_sync)]
        let db = Arc::new(Database::new(db_path).expect("Failed to initialize database"));
        let preferences = db.get_preferences().unwrap_or_default();
        let exiftool_available = ExifTool::check_availability();

        // Detect WSL more robustly: only consider WSL when running on Linux
        // and either the common WSL env vars are present or /proc/version
        // contains "microsoft". This prevents false-positive detection
        // on Windows where those env vars might be set.
        let is_wsl = if cfg!(target_os = "linux") {
            if std::env::var("WSL_DISTRO_NAME").is_ok() || std::env::var("WSLENV").is_ok() {
                true
            } else {
                match std::fs::read_to_string("/proc/version") {
                    Ok(s) => s.to_lowercase().contains("microsoft"),
                    Err(_) => false,
                }
            }
        } else {
            false
        };

        if is_wsl {
            info!("Running inside WSL - native file picker unavailable, use path text box.");
        }
        if exiftool_available {
            info!("ExifTool is available.");
        } else {
            warn!("ExifTool not found on PATH. Cleaning will be unavailable.");
        }

        let status_text = if is_wsl {
            "WSL detected - type a folder path in the box and press Enter or click Scan."
                .to_string()
        } else if exiftool_available {
            "Ready - drag a folder here or click Browse.".to_string()
        } else {
            "ExifTool not found - install it from https://exiftool.org to enable cleaning."
                .to_string()
        };

        let initial_path = preferences.last_scan_path.clone().unwrap_or_default();

        (
            Self {
                db,
                current_page: Page::Scanner,
                files: Vec::new(),
                history: Vec::new(),
                preferences,
                scanning: false,
                cleaning: false,
                progress: 0.0,
                clean_progress: 0.0,
                status_text,
                selected_file_id: None,
                exiftool_available,
                is_wsl,
                current_scan_id: None,
                path_input: initial_path,
                scan_rx: None,
                cancel_tx: None,
                clean_rx: None,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Metadata Cleaner".to_string()
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SwitchPage(page) => {
                self.current_page = page;
                if self.current_page == Page::History {
                    match self.db.get_recent_scans(50) {
                        Ok(history) => self.history = history,
                        Err(e) => error!("Failed to load history: {}", e),
                    }
                }
                Command::none()
            }

            Message::DirectorySelected(Some(path)) => {
                let path_str = path.to_string_lossy().to_string();
                self.path_input = path_str.clone();
                self.preferences.last_scan_path = Some(path_str.clone());
                let _ = self.db.save_preferences(&self.preferences);
                self.start_scan(path_str)
            }
            Message::DirectorySelected(None) => Command::none(),

            Message::FileDropped(path) => {
                let path_str = path.to_string_lossy().to_string();
                self.preferences.last_scan_path = Some(path_str.clone());
                let _ = self.db.save_preferences(&self.preferences);
                self.start_scan(path_str)
            }

            Message::CancelScan => {
                if let Some(tx) = &self.cancel_tx {
                    let _ = tx.send(true);
                }
                self.scanning = false;
                self.status_text = "Scan cancelled.".to_string();
                if let Some(scan_id) = self.current_scan_id.clone() {
                    let _ = self.db.update_scan_status(
                        &scan_id,
                        &ScanStatus::Failed("Cancelled by user".into()),
                    );
                    let _ = self
                        .db
                        .update_scan_totals(&scan_id, self.files.len() as i32, 0);
                }
                Command::none()
            }

            Message::ScannerPage(msg) => match msg {
                scanner_page::ScannerPageMessage::SelectDirectory => {
                    if self.is_wsl {
                        self.status_text =
                            "File picker unavailable in WSL - type the folder path in the box above and press Enter."
                                .to_string();
                        return Command::none();
                    }
                    Command::perform(
                        async {
                            rfd::AsyncFileDialog::new()
                                .set_title("Select Folder to Scan")
                                .pick_folder()
                                .await
                                .map(|h| h.path().to_path_buf())
                        },
                        Message::DirectorySelected,
                    )
                }

                scanner_page::ScannerPageMessage::CancelScan => self.update(Message::CancelScan),

                scanner_page::ScannerPageMessage::PathInputChanged(val) => {
                    self.path_input = val;
                    Command::none()
                }

                scanner_page::ScannerPageMessage::ScanPath => {
                    let path = self.path_input.trim().to_string();
                    if path.is_empty() {
                        self.update(Message::ScannerPage(
                            scanner_page::ScannerPageMessage::SelectDirectory,
                        ))
                    } else {
                        self.preferences.last_scan_path = Some(path.clone());
                        let _ = self.db.save_preferences(&self.preferences);
                        self.start_scan(path)
                    }
                }

                scanner_page::ScannerPageMessage::CleanMetadata => {
                    if !self.exiftool_available {
                        self.status_text =
                            "ExifTool is not installed. Please install it from https://exiftool.org"
                                .to_string();
                        return Command::none();
                    }
                    self.start_clean()
                }

                scanner_page::ScannerPageMessage::ExportReport => {
                    let files = self.files.clone();
                    Command::perform(
                        async move {
                            let default_name =
                                crate::backend::export::generate_default_filename("json");
                            let handle = rfd::AsyncFileDialog::new()
                                .set_file_name(&default_name)
                                .add_filter("JSON", &["json"])
                                .add_filter("CSV", &["csv"])
                                .save_file()
                                .await;

                            match handle {
                                None => None,
                                Some(path) => {
                                    let path_str = path.path().to_string_lossy().to_string();
                                    let res = if path_str.ends_with(".csv") {
                                        crate::backend::export::export_csv(&files, &path_str)
                                    } else {
                                        crate::backend::export::export_json(&files, &path_str)
                                    };
                                    Some(match res {
                                        Ok(()) => format!("Report exported to {}", path_str),
                                        Err(e) => format!("Export failed: {}", e),
                                    })
                                }
                            }
                        },
                        |result| match result {
                            Some(msg) => Message::ExportComplete(msg),
                            None => Message::ExportComplete(String::new()),
                        },
                    )
                }

                scanner_page::ScannerPageMessage::FileList(
                    file_list::FileListMessage::SelectFile(id),
                ) => {
                    self.selected_file_id = Some(id);
                    Command::none()
                }

                scanner_page::ScannerPageMessage::Settings(settings_msg) => {
                    match settings_msg {
                        settings_panel::SettingsMessage::RecursiveToggled(val) => {
                            self.preferences.recursive_default = val;
                        }
                        settings_panel::SettingsMessage::BackupToggled(val) => {
                            self.preferences.backup_enabled = val;
                        }
                        settings_panel::SettingsMessage::ThemeSelected(val) => {
                            self.preferences.theme = val;
                        }
                    }
                    let _ = self.db.save_preferences(&self.preferences);
                    Command::none()
                }
            },

            Message::HistoryPage(msg) => match msg {
                history_page::HistoryPageMessage::LoadHistory => {
                    match self.db.get_recent_scans(50) {
                        Ok(history) => self.history = history,
                        Err(e) => error!("Failed to load history: {}", e),
                    }
                    Command::none()
                }
                history_page::HistoryPageMessage::ViewScan(scan_id) => {
                    match self.db.get_files_for_scan(&scan_id) {
                        Ok(files) => {
                            self.files = files;
                            self.current_page = Page::Scanner;
                            self.current_scan_id = Some(scan_id);
                            self.status_text = "Loaded scan from history.".to_string();
                            self.progress = 100.0;
                        }
                        Err(e) => error!("Failed to load files for scan: {}", e),
                    }
                    Command::none()
                }
            },

            Message::ScanTick => {
                if let Some(rx) = &mut self.scan_rx {
                    match rx.try_recv() {
                        Ok(msg) => match msg {
                            ScannerMessage::FoundFile(f) => {
                                if let Err(e) = self.db.save_file(&f) {
                                    warn!("DB save_file failed: {}", e);
                                }
                                return Command::perform(
                                    async move { ScannerMessageWrapper::FoundFile(f) },
                                    Message::ScanProgress,
                                );
                            }
                            ScannerMessage::Completed(c) => {
                                return Command::perform(
                                    async move { ScannerMessageWrapper::Completed(c) },
                                    Message::ScanProgress,
                                );
                            }
                            ScannerMessage::Error(e) => {
                                return Command::perform(
                                    async move { ScannerMessageWrapper::Error(e) },
                                    Message::ScanProgress,
                                );
                            }
                        },
                        Err(mpsc::error::TryRecvError::Empty) => {}
                        Err(mpsc::error::TryRecvError::Disconnected) => {
                            self.scan_rx = None;
                            return Command::perform(async {}, |_| Message::ScanFinished);
                        }
                    }
                }
                Command::none()
            }

            Message::ScanProgress(msg) => {
                match msg {
                    ScannerMessageWrapper::FoundFile(file) => {
                        self.files.push(file);
                        let count = self.files.len();
                        self.status_text = format!("Scanning\u{2026} {} files found so far", count);
                        self.progress = (count % 100) as f32;
                    }
                    ScannerMessageWrapper::Completed(total) => {
                        self.scanning = false;
                        self.progress = 100.0;
                        self.status_text = format!("Scan complete \u{2014} {} files found.", total);
                        if let Some(scan_id) = self.current_scan_id.clone() {
                            let _ = self.db.update_scan_status(&scan_id, &ScanStatus::Completed);
                            let _ = self.db.update_scan_totals(&scan_id, total, 0);
                        }
                        info!("Scan completed: {} files", total);
                    }
                    ScannerMessageWrapper::Error(e) => {
                        self.scanning = false;
                        if e != "Scan cancelled" {
                            error!("Scan error: {}", e);
                            self.status_text = format!("Scan error: {}", e);
                        }
                    }
                }
                Command::none()
            }

            Message::ScanFinished => {
                self.scanning = false;
                Command::none()
            }

            Message::CleanTick => {
                if let Some(rx) = &mut self.clean_rx {
                    match rx.try_recv() {
                        Ok(result) => {
                            if result.file_id.is_empty() {
                                self.cleaning = false;
                                self.clean_rx = None;
                                let cleaned = result.total_cleaned;
                                let total = self.files.len() as i32;
                                self.clean_progress = 100.0;
                                self.status_text = format!(
                                    "Cleaning complete \u{2014} {}/{} files cleaned.",
                                    cleaned, total
                                );
                                if let Some(scan_id) = self.current_scan_id.clone() {
                                    let _ = self.db.update_scan_totals(&scan_id, total, cleaned);
                                }
                                info!("Cleaning complete: {}/{} files", cleaned, total);
                            } else {
                                let new_status = if result.success {
                                    FileStatus::Cleaned
                                } else {
                                    FileStatus::Error(result.error_msg.clone().unwrap_or_default())
                                };

                                if let Some(f) =
                                    self.files.iter_mut().find(|f| f.id == result.file_id)
                                {
                                    f.status = new_status.clone();
                                }

                                if let Err(e) =
                                    self.db
                                        .update_file_status(&result.file_id, &new_status, None)
                                {
                                    warn!("DB update_file_status failed: {}", e);
                                }

                                let total = self.files.len() as i32;
                                let remaining = result.total_remaining;
                                self.clean_progress = if total > 0 {
                                    ((total - remaining) as f32 / total as f32) * 100.0
                                } else {
                                    0.0
                                };
                                self.status_text = format!(
                                    "Cleaning\u{2026} {} of {} files done",
                                    total - remaining,
                                    total
                                );
                            }
                        }
                        Err(mpsc::error::TryRecvError::Empty) => {}
                        Err(mpsc::error::TryRecvError::Disconnected) => {
                            self.clean_rx = None;
                            self.cleaning = false;
                        }
                    }
                }
                Command::none()
            }

            Message::ExportComplete(msg) => {
                if !msg.is_empty() {
                    self.status_text = msg;
                }
                Command::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let scan_tick = if self.scanning {
            iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::ScanTick)
        } else {
            Subscription::none()
        };

        let clean_tick = if self.cleaning {
            iced::time::every(std::time::Duration::from_millis(16)).map(|_| Message::CleanTick)
        } else {
            Subscription::none()
        };

        let dnd = iced::event::listen_with(|event, _status| {
            if let iced::Event::Window(_, iced::window::Event::FileDropped(path)) = event {
                Some(Message::FileDropped(path))
            } else {
                None
            }
        });

        Subscription::batch(vec![scan_tick, clean_tick, dnd])
    }

    fn view(&self) -> Element<'_, Message> {
        let content = match self.current_page {
            Page::Scanner => scanner_page::view(
                &self.files,
                self.selected_file_id.as_deref(),
                self.scanning,
                self.cleaning,
                self.progress,
                self.clean_progress,
                &self.status_text,
                &self.preferences,
                self.exiftool_available,
                &self.path_input,
                self.is_wsl,
            )
            .map(Message::ScannerPage),
            Page::History => history_page::view(&self.history).map(Message::HistoryPage),
        };

        let nav = row![
            button("Scanner").on_press(Message::SwitchPage(Page::Scanner)),
            button("History").on_press(Message::SwitchPage(Page::History)),
        ]
        .spacing(10)
        .padding(10);

        column![nav, content].into()
    }

    fn theme(&self) -> Theme {
        if self.preferences.theme == "dark" {
            Theme::Dark
        } else {
            Theme::Light
        }
    }
}

impl MetadataCleanerApp {
    fn start_scan(&mut self, path: String) -> Command<Message> {
        self.files.clear();
        self.scanning = true;
        self.progress = 0.0;
        self.selected_file_id = None;
        self.status_text = format!("Scanning {}\u{2026}", path);

        let scan_id = Uuid::new_v4().to_string();

        let scan_record = ScanRecord {
            id: scan_id.clone(),
            timestamp: Local::now(),
            root_path: path.clone(),
            recursive: self.preferences.recursive_default,
            total_files: 0,
            cleaned_files: 0,
            status: ScanStatus::InProgress,
        };
        if let Err(e) = self.db.save_scan(&scan_record) {
            error!("Failed to persist scan record: {}", e);
        }
        self.current_scan_id = Some(scan_id.clone());

        let (tx, rx) = mpsc::channel(512);
        let (cancel_tx, cancel_rx) = watch::channel(false);
        self.scan_rx = Some(rx);
        self.cancel_tx = Some(cancel_tx);

        let recursive = self.preferences.recursive_default;

        tokio::spawn(async move {
            Scanner::scan_directory(path, scan_id, recursive, tx, cancel_rx).await;
        });

        Command::none()
    }

    fn start_clean(&mut self) -> Command<Message> {
        let files_to_clean: Vec<FileRecord> = self
            .files
            .iter()
            .filter(|f| matches!(f.status, FileStatus::Scanned))
            .cloned()
            .collect();

        if files_to_clean.is_empty() {
            self.status_text =
                "No files to clean \u{2014} run a scan first, or all files are already cleaned."
                    .to_string();
            return Command::none();
        }

        let total = files_to_clean.len() as i32;
        self.cleaning = true;
        self.clean_progress = 0.0;
        self.status_text = format!("Cleaning {} files\u{2026}", total);

        let (tx, rx) = mpsc::channel::<CleanResult>(512);
        self.clean_rx = Some(rx);

        let backup_enabled = self.preferences.backup_enabled;

        tokio::spawn(async move {
            let mut cleaned = 0i32;
            let mut remaining = total;

            for file in files_to_clean {
                remaining -= 1;
                let path = file.path.clone();
                let id = file.id.clone();

                let result = tokio::task::spawn_blocking(move || {
                    if backup_enabled {
                        if let Err(e) = ExifTool::create_backup(&path) {
                            warn!("Backup failed for {}: {}", path, e);
                        }
                    }
                    ExifTool::clean_metadata(&path, backup_enabled)
                })
                .await;

                let (success, error_msg) = match result {
                    Ok(Ok(())) => {
                        cleaned += 1;
                        (true, None)
                    }
                    Ok(Err(e)) => {
                        error!("ExifTool clean failed for {}: {}", id, e);
                        (false, Some(e.to_string()))
                    }
                    Err(e) => {
                        error!("spawn_blocking panicked for {}: {}", id, e);
                        (false, Some(format!("Task error: {}", e)))
                    }
                };

                if tx
                    .send(CleanResult {
                        file_id: id,
                        success,
                        error_msg,
                        total_cleaned: cleaned,
                        total_remaining: remaining,
                    })
                    .await
                    .is_err()
                {
                    break;
                }
            }

            let _ = tx
                .send(CleanResult {
                    file_id: String::new(),
                    success: true,
                    error_msg: None,
                    total_cleaned: cleaned,
                    total_remaining: 0,
                })
                .await;
        });

        Command::none()
    }
}
