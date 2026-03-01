use crate::backend::models::UserPreferences;
use iced::widget::{checkbox, column, container, pick_list, row, text};
use iced::Element;

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    RecursiveToggled(bool),
    BackupToggled(bool),
    ThemeSelected(String),
}

pub fn view<'a>(prefs: &UserPreferences) -> Element<'a, SettingsMessage> {
    let recursive_toggle = checkbox("Recursive Scan (Scan subfolders)", prefs.recursive_default)
        .on_toggle(SettingsMessage::RecursiveToggled);

    let backup_toggle = checkbox("Create Backup (Keep original files)", prefs.backup_enabled)
        .on_toggle(SettingsMessage::BackupToggled);

    let themes = vec!["light".to_string(), "dark".to_string()];
    let theme_picker = pick_list(
        themes,
        Some(prefs.theme.clone()),
        SettingsMessage::ThemeSelected,
    );

    container(
        column![
            text("Settings").size(20),
            recursive_toggle,
            backup_toggle,
            row![text("Theme:"), theme_picker,].spacing(10),
        ]
        .spacing(20),
    )
    .padding(20)
    .into()
}
