use iced::widget::{column, row, text, checkbox, container, pick_list};
use iced::{Element, Length};
use crate::backend::models::UserPreferences;

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    RecursiveChanged(bool),
    BackupChanged(bool),
    ThemeChanged(String),
}

pub fn view<'a>(
    prefs: &UserPreferences,
) -> Element<'a, SettingsMessage> {
    let recursive_toggle = checkbox(
        "Recursive Scan (Scan subfolders)",
        prefs.recursive_default,
    )
    .on_toggle(SettingsMessage::RecursiveChanged);

    let backup_toggle = checkbox(
        "Create Backup (Keep original files)",
        prefs.backup_enabled,
    )
    .on_toggle(SettingsMessage::BackupChanged);

    let themes = vec!["light".to_string(), "dark".to_string()];
    let theme_picker = pick_list(
        themes,
        Some(prefs.theme.clone()),
        SettingsMessage::ThemeChanged,
    );

    container(
        column![
            text("Settings").size(20),
            recursive_toggle,
            backup_toggle,
            row![
                text("Theme:"),
                theme_picker,
            ].spacing(10),
        ]
        .spacing(20)
    )
    .padding(20)
    .into()
}
