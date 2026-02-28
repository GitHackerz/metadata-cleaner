use iced::widget::{container, progress_bar, text, column};
use iced::{Element, Length};

pub fn view<'a, Message: 'a>(
    progress: f32,
    status_text: &str,
) -> Element<'a, Message> {
    container(
        column![
            text(status_text).size(14),
            progress_bar(0.0..=100.0, progress).height(10),
        ]
        .spacing(5)
    )
    .width(Length::Fill)
    .padding(10)
    .into()
}
