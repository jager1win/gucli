#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tracing_subscriber::{fmt, EnvFilter};
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::fmt::format::Writer;
use chrono::Local;
use gucli_lib::files::LineLimitedWriter;

struct LogTime;

impl FormatTime for LogTime {
    fn format_time(&self, w: &mut Writer<'_>) -> std::fmt::Result {
        write!(w, "{}", Local::now().format("%Y-%m-%d %H:%M:%S%.3f"))
    }
}

fn init_tracing() {
    let log_path = gucli_lib::files::full_path_log();
    let file_writer = LineLimitedWriter::new(log_path, 100);

    let format = fmt::format()
        .with_timer(LogTime)
        .with_level(true)
        .with_target(false)
        .with_ansi(false)
        .compact();

    let subscriber = fmt::Subscriber::builder()
        .with_env_filter(EnvFilter::new("info"))
        .event_format(format)
        .with_writer(file_writer)
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to init logger");
}

fn main() {
    init_tracing();
    gucli_lib::run();
}
