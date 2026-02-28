use iced::widget::{column, row, text, scrollable, button};
use iced::{Element, Length, Color};
use crate::backend::models::{FileRecord, FileStatus};

#[derive(Debug, Clone)]
pub enum FileListMessage {
    SelectFile(String),
}

pub fn view<'a>(
    files: &'a [FileRecord],
    selected_file_id: Option<&'a str>,
) -> Element<'a, FileListMessage> {
    let content = column(
        files.iter().map(|file| {
            let is_selected = Some(file.id.as_str()) == selected_file_id;
            
            let status_color = match file.status {
                FileStatus::Scanned => Color::from_rgb(0.8, 0.8, 0.0), // Yellow
                FileStatus::Cleaned => Color::from_rgb(0.0, 0.8, 0.0), // Green
                FileStatus::Error(_) => Color::from_rgb(0.8, 0.0, 0.0), // Red
                FileStatus::Skipped => Color::from_rgb(0.5, 0.5, 0.5), // Grey
            };

            let row_content = row![
                text(&file.path).width(Length::FillPortion(3)).size(14),
                text(&file.file_type).width(Length::FillPortion(1)).size(14),
                text(format!("{:?}", file.status)).style(iced::theme::Text::Color(status_color)).width(Length::FillPortion(1)).size(14),
            ]
            .spacing(10)
            .padding(5);

            button(row_content)
                .on_press(FileListMessage::SelectFile(file.id.clone()))
                .style(if is_selected { iced::theme::Button::Primary } else { iced::theme::Button::Secondary })
                .width(Length::Fill)
                .into()
        }).collect::<Vec<_>>()
    )
    .spacing(2);

    scrollable(content)
        .height(Length::Fill)
        .width(Length::Fill)
        .into()
}
