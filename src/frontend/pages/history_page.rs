use iced::widget::{column, row, text, scrollable, button, container};
use iced::{Element, Length, Color};
use crate::backend::models::{ScanRecord, ScanStatus};
use crate::frontend::ui;

#[derive(Debug, Clone)]
pub enum HistoryPageMessage {
    LoadHistory,
    ViewScan(String),
}

pub fn view<'a>(
    history: &'a [ScanRecord],
) -> Element<'a, HistoryPageMessage> {
    let header = ui::header("Scan History");

    let list = if history.is_empty() {
        column![
            text("No scan history yet. Run a scan from the Scanner page.")
                .size(16)
                .style(iced::theme::Text::Color(Color::from_rgb(0.5, 0.5, 0.5)))
        ]
        .spacing(10)
    } else {
        column(
            history
                .iter()
                .map(|scan| {
                    let status_color = match &scan.status {
                        ScanStatus::Completed => Color::from_rgb(0.0, 0.7, 0.0),
                        ScanStatus::InProgress => Color::from_rgb(0.9, 0.7, 0.0),
                        ScanStatus::Failed(_) => Color::from_rgb(0.8, 0.0, 0.0),
                    };
                    let status_label = match &scan.status {
                        ScanStatus::Completed => "Completed".to_string(),
                        ScanStatus::InProgress => "In Progress".to_string(),
                        ScanStatus::Failed(msg) => format!("Failed: {}", msg),
                    };

                    container(
                        row![
                            text(scan.timestamp.format("%Y-%m-%d %H:%M:%S").to_string())
                                .size(13)
                                .width(Length::Fixed(180.0)),
                            text(&scan.root_path)
                                .size(13)
                                .width(Length::Fill),
                            text(format!("{} files", scan.total_files))
                                .size(13)
                                .width(Length::Fixed(80.0)),
                            text(format!("{} cleaned", scan.cleaned_files))
                                .size(13)
                                .width(Length::Fixed(90.0)),
                            text(status_label)
                                .size(13)
                                .style(iced::theme::Text::Color(status_color))
                                .width(Length::Fixed(140.0)),
                            button("View Files")
                                .on_press(HistoryPageMessage::ViewScan(scan.id.clone()))
                                .padding([4, 8]),
                        ]
                        .spacing(12)
                        .padding(8),
                    )
                    .style(iced::theme::Container::Box)
                    .into()
                })
                .collect::<Vec<_>>(),
        )
        .spacing(6)
    };

    column![
        header,
        scrollable(list).height(Length::Fill),
        button("Refresh").on_press(HistoryPageMessage::LoadHistory),
    ]
    .padding(20)
    .spacing(16)
    .into()
}
