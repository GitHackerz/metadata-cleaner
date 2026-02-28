use log::LevelFilter;
use env_logger::{Builder, Target};
use std::io::Write;

pub fn init() {
    let mut builder = Builder::new();
    builder
        .format(|buf, record| {
            writeln!(
                buf,
                "{} [{}] - {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                record.level(),
                record.args()
            )
        })
        // Our crates at INFO
        .filter_module("metadata_cleaner", LevelFilter::Info)
        // Silence chatty third-party internals
        .filter_module("wgpu",            LevelFilter::Warn)
        .filter_module("wgpu_core",       LevelFilter::Warn)
        .filter_module("wgpu_hal",        LevelFilter::Warn)
        .filter_module("naga",            LevelFilter::Warn)
        .filter_module("iced",            LevelFilter::Warn)
        .filter_module("iced_wgpu",       LevelFilter::Warn)
        .filter_module("iced_winit",      LevelFilter::Warn)
        .filter_module("zbus",            LevelFilter::Warn)
        .filter_module("ashpd",           LevelFilter::Warn)
        .filter_module("tracing",         LevelFilter::Warn)
        // Fall-back for everything else: WARN
        .filter(None, LevelFilter::Warn)
        .target(Target::Stdout)
        .init();
}

