use crate::backend::models::{FileRecord, UserPreferences};
use crate::frontend::components::{file_list, progress_bar, settings_panel};
use crate::frontend::ui;
use iced::widget::{column, container, row, text, text_input};
use iced::{Alignment, Color, Element, Length};

#[derive(Debug, Clone)]
pub enum ScannerPageMessage {
    FileList(file_list::FileListMessage),
    Settings(settings_panel::SettingsMessage),
    SelectDirectory,
    CancelScan,
    CleanMetadata,
    ExportReport,
    PathInputChanged(String),
    ScanPath,
}

#[allow(clippy::too_many_arguments)]
pub fn view<'a>(
    files: &'a [FileRecord],
    selected_file_id: Option<&'a str>,
    scanning: bool,
    cleaning: bool,
    progress: f32,
    clean_progress: f32,
    status_text: &str,
    prefs: &UserPreferences,
    exiftool_available: bool,
    path_input: &'a str,
    is_wsl: bool,
) -> Element<'a, ScannerPageMessage> {
    let header = ui::header("Metadata Scanner");

    let file_list_view = file_list::view(files, selected_file_id).map(ScannerPageMessage::FileList);

    let settings_view = settings_panel::view(prefs).map(ScannerPageMessage::Settings);

    // ----- Toolbar buttons -----
    // On WSL the XDG portal is unavailable — user must type path manually
    let browse_label = if is_wsl {
        "Browse (N/A)"
    } else {
        "Browse\u{2026}"
    };
    let select_btn = if is_wsl {
        ui::primary_button(browse_label) // no on_press = disabled
    } else {
        ui::primary_button(browse_label).on_press(ScannerPageMessage::SelectDirectory)
    };

    let path_field = text_input("Type or paste a folder path\u{2026}", path_input)
        .on_input(ScannerPageMessage::PathInputChanged)
        .on_submit(ScannerPageMessage::ScanPath)
        .width(Length::Fill)
        .padding(8);

    let scan_btn = if scanning {
        ui::primary_button("Cancel").on_press(ScannerPageMessage::CancelScan)
    } else {
        ui::primary_button("Scan").on_press(ScannerPageMessage::ScanPath)
    };

    let clean_btn = if exiftool_available && !cleaning && !files.is_empty() {
        ui::primary_button("Clean Metadata").on_press(ScannerPageMessage::CleanMetadata)
    } else {
        // Disabled appearance when conditions not met
        ui::primary_button("Clean Metadata")
    };

    let export_btn = if !files.is_empty() {
        ui::primary_button("Export Report").on_press(ScannerPageMessage::ExportReport)
    } else {
        ui::primary_button("Export Report")
    };

    let toolbar = row![select_btn, path_field, scan_btn, clean_btn, export_btn]
        .spacing(8)
        .align_items(Alignment::Center);

    // ----- ExifTool warning -----
    let exiftool_warning: Option<Element<ScannerPageMessage>> = if !exiftool_available {
        Some(
            container(
                text("ExifTool not found — install it from https://exiftool.org")
                    .size(13)
                    .style(Color::from_rgb(0.9, 0.5, 0.1)),
            )
            .padding(8)
            .into(),
        )
    } else {
        None
    };

    // ----- Main content area -----
    let empty_hint = if is_wsl {
        "Type a folder path in the box above and press Enter or click Scan."
    } else {
        "Drag a folder onto the window, or click Browse above."
    };
    let main_content: Element<ScannerPageMessage> = if files.is_empty() && !scanning {
        container(
            text(empty_hint)
                .size(20)
                .style(iced::theme::Text::Color(Color::from_rgb(0.5, 0.5, 0.5))),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into()
    } else {
        file_list_view
    };

    // ----- Progress section -----
    let mut progress_col = column![].spacing(4);
    if scanning || progress > 0.0 {
        progress_col = progress_col.push(progress_bar::view(progress, status_text));
    }
    if cleaning || clean_progress > 0.0 {
        let clean_label = format!("Cleaning progress: {}%", clean_progress as u32);
        progress_col = progress_col.push(progress_bar::view(clean_progress, &clean_label));
    }
    if !scanning && clean_progress == 0.0 {
        progress_col = progress_col.push(text(status_text).size(13));
    }

    // ----- File count summary -----
    let total = files.len();
    let cleaned = files
        .iter()
        .filter(|f| matches!(f.status, crate::backend::models::FileStatus::Cleaned))
        .count();
    let errors = files
        .iter()
        .filter(|f| matches!(f.status, crate::backend::models::FileStatus::Error(_)))
        .count();

    let summary: Element<ScannerPageMessage> = if total > 0 {
        text(format!(
            "{} files  |  {} cleaned  |  {} errors",
            total, cleaned, errors
        ))
        .size(13)
        .style(Color::from_rgb(0.6, 0.6, 0.6))
        .into()
    } else {
        column![].into()
    };

    // ----- Assemble layout -----
    let mut left_col = column![main_content]
        .push(progress_col)
        .push(summary)
        .spacing(6);

    if let Some(warning) = exiftool_warning {
        left_col = left_col.push(warning);
    }

    left_col = left_col.push(container(toolbar).padding(12).center_x());

    column![
        header,
        row![
            left_col.width(Length::FillPortion(3)),
            container(settings_view)
                .width(Length::FillPortion(1))
                .style(iced::theme::Container::Box),
        ]
        .spacing(10)
        .height(Length::Fill),
    ]
    .into()
}
