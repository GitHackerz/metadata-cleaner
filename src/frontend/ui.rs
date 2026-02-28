use iced::widget::{container, text, button};
use iced::{Element, Length, Theme, Background};
use iced::theme;

pub fn header<'a, Message: 'a>(title: &str) -> Element<'a, Message> {
    container(
        text(title)
            .size(20)
            .width(Length::Fill)
            .horizontal_alignment(iced::alignment::Horizontal::Center)
    )
    .padding(10)
    .style(theme::Container::Custom(Box::new(HeaderStyle)))
    .into()
}

struct HeaderStyle;

impl container::StyleSheet for HeaderStyle {
    type Style = Theme;

    fn appearance(&self, theme: &Self::Style) -> container::Appearance {
        let palette = theme.extended_palette();
        container::Appearance {
            background: Some(Background::Color(palette.background.strong.color)),
            text_color: Some(palette.background.strong.text),
            ..Default::default()
        }
    }
}

pub fn primary_button<'a, Message>(label: &str) -> button::Button<'a, Message> {
    button(text(label).horizontal_alignment(iced::alignment::Horizontal::Center))
        .padding(10)
        .width(Length::Fixed(150.0))
}
