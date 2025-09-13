#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use chrono::Local;
use gucli_lib::files::LineLimitedWriter;
use nix::libc;
use std::fs::{File, OpenOptions};
use std::os::unix::io::AsRawFd;
use tracing_subscriber::fmt::format::Writer;
use tracing_subscriber::fmt::time::FormatTime;
use tracing_subscriber::{EnvFilter, fmt};

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

    tracing::subscriber::set_global_default(subscriber).expect("Failed to init logger");
}

// lock - single instance
fn enforce_single_instance() -> Result<File, String> {
    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open("/tmp/gucli.lock")
        .map_err(|e| format!("Failed to open lock file: {e}"))?;

    unsafe {
        if libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) != 0 {
            return Err("Another instance is already running".to_string());
        }
    }

    Ok(file)
}

fn main() {
    init_tracing();
    let _lock = match enforce_single_instance() {
        Ok(file) => file,
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    };
    gucli_lib::run();
}
