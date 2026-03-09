use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use chrono::Local;
use log::{Level, LevelFilter, Metadata, Record};

struct SimpleFileLogger {
    log_path: PathBuf,
}

impl log::Log for SimpleFileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Info
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
            let msg = format!(
                "[{}] {:<5} - {}
",
                timestamp,
                record.level(),
                record.args()
            );

            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.log_path)
            {
                let _ = file.write_all(msg.as_bytes());
            }
        }
    }

    fn flush(&self) {}
}

pub fn init() -> anyhow::Result<()> {
    let cache_dir = std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .or_else(|_| {
            std::env::var("HOME")
                .map(|h| PathBuf::from(h).join(".cache"))
        })
        .map_err(|_| anyhow::anyhow!("Could not determine cache directory"))?;

    let log_dir = cache_dir.join("hyprrun");
    if !log_dir.exists() {
        std::fs::create_dir_all(&log_dir)?;
    }
    let log_path = log_dir.join("hyprrun.log");

    let logger = SimpleFileLogger { log_path };

    log::set_boxed_logger(Box::new(logger))
        .map(|()| log::set_max_level(LevelFilter::Info))
        .map_err(|e| anyhow::anyhow!("Failed to set logger: {}", e))
}
