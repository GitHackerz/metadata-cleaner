use iced::{Application, Command, Element, Settings, Theme, Subscription, window};
use iced::widget::{column, row, button, text, container};
use iced::executor;
use std::path::PathBuf;
use tokio::sync::mpsc;
use std::sync::Arc;
use tokio::sync::Mutex;

use crate::backend::models::{FileRecord, ScanRecord, UserPreferences, ScanStatus};
use crate::backend::db::Database;
use crate::backend::scanner::{Scanner, ScannerMessage};
use crate::backend::exiftool::ExifTool;
use crate::frontend::pages::{scanner_page, history_page};
use crate::frontend::components::{file_list, settings_panel};

pub struct MetadataCleanerApp {
    db: Arc<Database>,
    current_page: Page,
    files: Vec<FileRecord>,
    history: Vec<ScanRecord>,
    preferences: UserPreferences,
    scanning: bool,
    progress: f32,
    status_text: String,
    selected_file_id: Option<String>,
    rx: Option<mpsc::Receiver<ScannerMessage>>,
}

#[derive(Debug, Clone, PartialEq)]
enum Page {
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
    Tick,
}

// Wrapper to make ScannerMessage cloneable or compatible with Iced messages if needed, 
// but here we just use a custom Debug wrapper or similar if direct mapping is hard.
// For simplicity, we'll define a local wrapper or just map fields.
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
        
        let db = Arc::new(Database::new(db_path).expect("Failed to init DB"));
        let preferences = db.get_preferences().unwrap_or_default();

        (
            Self {
                db,
                current_page: Page::Scanner,
                files: Vec::new(),
                history: Vec::new(),
                preferences,
                scanning: false,
                progress: 0.0,
                status_text: String::from("Ready"),
                selected_file_id: None,
                rx: None,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Metadata Cleaner")
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SwitchPage(page) => {
                self.current_page = page;
                if self.current_page == Page::History {
                    // Load history
                    if let Ok(history) = self.db.get_recent_scans(50) {
                        self.history = history;
                    }
                }
                Command::none()
            }
            Message::ScannerPage(msg) => match msg {
                scanner_page::ScannerPageMessage::StartScan => {
                    if let Some(path) = &self.preferences.last_scan_path {
                        self.start_scan(path.clone())
                    } else {
                        // Trigger file picker if no path (or handle via drag drop primarily)
                        self.status_text = "Please drag and drop a folder first".to_string();
                        Command::none()
                    }
                }
                scanner_page::ScannerPageMessage::CleanMetadata => {
                    // Implement cleaning logic here (iterate files, call ExifTool, update DB)
                    self.status_text = "Cleaning not fully implemented in this demo step".to_string();
                    Command::none()
                }
                scanner_page::ScannerPageMessage::ExportReport => {
                    let files = self.files.clone();
                    let status_text = self.status_text.clone();
                    
                    return Command::perform(async move {
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name("scan_report.json")
                            .add_filter("JSON", &["json"])
                            .add_filter("CSV", &["csv"])
                            .save_file() 
                        {
                            let path_str = path.to_string_lossy().to_string();
                            let res = if path_str.ends_with(".csv") {
                                crate::backend::export::export_csv(&files, &path_str)
                            } else {
                                crate::backend::export::export_json(&files, &path_str)
                            };
                            
                            match res {
                                Ok(_) => format!("Report exported to {}", path_str),
                                Err(e) => format!("Export failed: {}", e),
                            }
                        } else {
                            status_text
                        }
                    }, |msg| Message::ScanProgress(ScannerMessageWrapper::Error(msg)));
                }
                scanner_page::ScannerPageMessage::FileList(file_list::FileListMessage::SelectFile(id)) => {
                    self.selected_file_id = Some(id);
                    Command::none()
                }
                scanner_page::ScannerPageMessage::Settings(settings_msg) => {
                    match settings_msg {
                        settings_panel::SettingsMessage::RecursiveChanged(val) => self.preferences.recursive_default = val,
                        settings_panel::SettingsMessage::BackupChanged(val) => self.preferences.backup_enabled = val,
                        settings_panel::SettingsMessage::ThemeChanged(val) => self.preferences.theme = val,
                    }
                    let _ = self.db.save_preferences(&self.preferences);
                    Command::none()
                }
            },
            Message::HistoryPage(msg) => match msg {
                history_page::HistoryPageMessage::LoadHistory => {
                    if let Ok(history) = self.db.get_recent_scans(50) {
                        self.history = history;
                    }
                    Command::none()
                }
            },
            Message::FileDropped(path) => {
                let path_str = path.to_string_lossy().to_string();
                self.preferences.last_scan_path = Some(path_str.clone());
                let _ = self.db.save_preferences(&self.preferences);
                self.start_scan(path_str)
            }
            Message::ScanProgress(msg) => {
                match msg {
                    ScannerMessageWrapper::FoundFile(file) => {
                        self.files.push(file);
                        self.status_text = format!("Found {} files...", self.files.len());
                    }
                    ScannerMessageWrapper::Completed(total) => {
                        self.scanning = false;
                        self.progress = 100.0;
                        self.status_text = format!("Scan complete. Found {} files.", total);
                    }
                    ScannerMessageWrapper::Error(e) => {
                        self.status_text = format!("Error: {}", e);
                    }
                }
                Command::none()
            }
            Message::ScanFinished => {
                self.scanning = false;
                Command::none()
            }
            Message::Tick => {
                if let Some(rx) = &mut self.rx {
                    // Poll receiver
                    match rx.try_recv() {
                        Ok(msg) => {
                            match msg {
                                ScannerMessage::FoundFile(f) => {
                                    return Command::perform(async { ScannerMessageWrapper::FoundFile(f) }, Message::ScanProgress);
                                }
                                ScannerMessage::Completed(c) => {
                                    return Command::perform(async { ScannerMessageWrapper::Completed(c) }, Message::ScanProgress);
                                }
                                ScannerMessage::Error(e) => {
                                    return Command::perform(async { ScannerMessageWrapper::Error(e) }, Message::ScanProgress);
                                }
                            }
                        }
                        Err(mpsc::error::TryRecvError::Empty) => {}
                        Err(mpsc::error::TryRecvError::Disconnected) => {
                            self.rx = None;
                            return Command::perform(async {}, |_| Message::ScanFinished);
                        }
                    }
                }
                Command::none()
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        let tick = if self.scanning {
            iced::time::every(std::time::Duration::from_millis(10)).map(|_| Message::Tick)
        } else {
            Subscription::none()
        };

        let dnd = iced::event::listen().map(|event| {
            if let iced::Event::Window(_, iced::window::Event::FileDropped(path)) = event {
                Some(Message::FileDropped(path))
            } else {
                None
            }
        }).filter_map(|m| m);

        Subscription::batch(vec![tick, dnd])
    }

    fn view(&self) -> Element<Message> {
        let content = match self.current_page {
            Page::Scanner => scanner_page::view(
                &self.files,
                self.selected_file_id.as_deref(),
                self.scanning,
                self.progress,
                &self.status_text,
                &self.preferences
            ).map(Message::ScannerPage),
            Page::History => history_page::view(&self.history).map(Message::HistoryPage),
        };

        let nav = row![
            button("Scanner").on_press(Message::SwitchPage(Page::Scanner)),
            button("History").on_press(Message::SwitchPage(Page::History)),
        ].spacing(10).padding(10);

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
        self.status_text = format!("Scanning {}...", path);
        
        let (tx, rx) = mpsc::channel(100);
        self.rx = Some(rx);
        
        let recursive = self.preferences.recursive_default;
        let scan_id = uuid::Uuid::new_v4().to_string();

        // Spawn scanner
        tokio::spawn(async move {
            Scanner::scan_directory(path, scan_id, recursive, tx).await;
        });

        Command::none()
    }
}
