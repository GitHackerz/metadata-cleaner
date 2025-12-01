use iced::widget::{column, row, text, scrollable, button};
use iced::{Element, Length};
use crate::backend::models::ScanRecord;
use crate::frontend::ui;

#[derive(Debug, Clone)]
pub enum HistoryPageMessage {
    LoadHistory,
}

pub fn view<'a>(
    history: &'a [ScanRecord],
) -> Element<'a, HistoryPageMessage> {
    let header = ui::header("Scan History");

    let list = column(
        history.iter().map(|scan| {
            row![
                text(&scan.timestamp.format("%Y-%m-%d %H:%M:%S").to_string()),
                text(&scan.root_path),
                text(format!("Files: {}", scan.total_files)),
                text(format!("Cleaned: {}", scan.cleaned_files)),
                text(format!("{:?}", scan.status)),
            ]
            .spacing(20)
            .padding(10)
            .into()
        }).collect()
    )
    .spacing(10);

    column![
        header,
        scrollable(list).height(Length::Fill),
        button("Refresh History").on_press(HistoryPageMessage::LoadHistory),
    ]
    .padding(20)
    .spacing(20)
    .into()
}
