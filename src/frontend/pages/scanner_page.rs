use iced::widget::{column, row, text, button, container};
use iced::{Element, Length, Alignment};
use crate::backend::models::{FileRecord, UserPreferences};
use crate::frontend::components::{file_list, progress_bar, settings_panel};
use crate::frontend::ui;

#[derive(Debug, Clone)]
pub enum ScannerPageMessage {
    FileList(file_list::FileListMessage),
    Settings(settings_panel::SettingsMessage),
    StartScan,
    CleanMetadata,
    ExportReport,
}

pub fn view<'a>(
    files: &'a [FileRecord],
    selected_file_id: Option<&'a str>,
    scanning: bool,
    progress: f32,
    status_text: &str,
    prefs: &UserPreferences,
) -> Element<'a, ScannerPageMessage> {
    let header = ui::header("Metadata Scanner");

    let file_list_view = file_list::view(files, selected_file_id)
        .map(ScannerPageMessage::FileList);

    let settings_view = settings_panel::view(prefs)
        .map(ScannerPageMessage::Settings);

    let actions = row![
        ui::primary_button("Scan Folder").on_press(ScannerPageMessage::StartScan),
        ui::primary_button("Clean Metadata").on_press(ScannerPageMessage::CleanMetadata),
        ui::primary_button("Export Report").on_press(ScannerPageMessage::ExportReport),
    ]
    .spacing(20)
    .align_items(Alignment::Center);

    let main_content = if files.is_empty() && !scanning {
        container(
            text("Drag and drop a folder here to start scanning")
                .size(24)
                .style(iced::theme::Text::Color(iced::Color::from_rgb(0.5, 0.5, 0.5)))
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into()
    } else {
        file_list_view
    };

    let progress_view = if scanning || progress > 0.0 {
        progress_bar::view(progress, status_text)
    } else {
        column![].into()
    };

    column![
        header,
        row![
            column![
                main_content,
                progress_view,
                container(actions).padding(20).center_x(),
            ].width(Length::FillPortion(3)),
            container(settings_view)
                .width(Length::FillPortion(1))
                .style(iced::theme::Container::Box),
        ].spacing(10).height(Length::Fill),
    ]
    .into()
}
