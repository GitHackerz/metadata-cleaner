mod backend;
mod frontend;

use frontend::main::MetadataCleanerApp;
use iced::{Application, Settings};

fn main() -> iced::Result {
    backend::logger::init();

    MetadataCleanerApp::run(Settings {
        window: iced::window::Settings {
            size: iced::Size::new(1024.0, 768.0),
            ..Default::default()
        },
        ..Default::default()
    })
}
