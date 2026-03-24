use log::{Level, LevelFilter, Metadata, Record};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::sync::Mutex;

static LOGGER: LazyLogger = LazyLogger {
    file: Mutex::new(None),
};

struct LazyLogger {
    file: Mutex<Option<fs::File>>,
}

impl log::Log for LazyLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= Level::Debug
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            if let Ok(mut guard) = self.file.lock() {
                if let Some(ref mut f) = *guard {
                    let _ = writeln!(
                        f,
                        "[{}] {}:{} {}",
                        record.level(),
                        record.file().unwrap_or("?"),
                        record.line().unwrap_or(0),
                        record.args()
                    );
                }
            }
        }
    }

    fn flush(&self) {
        if let Ok(mut guard) = self.file.lock() {
            if let Some(ref mut f) = *guard {
                let _ = f.flush();
            }
        }
    }
}

/// Initializes the file logger. Call once at startup.
/// Logs go to ~/.local/state/lazycomposer/debug.log
pub fn init() -> Result<(), Box<dyn std::error::Error>> {
    let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/tmp"));
    let dir = format!("{home}/.local/state/lazycomposer");
    fs::create_dir_all(&dir)?;

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(format!("{dir}/debug.log"))?;

    if let Ok(mut guard) = LOGGER.file.lock() {
        *guard = Some(file);
    }

    log::set_logger(&LOGGER).map_err(|e| format!("{e}"))?;
    log::set_max_level(LevelFilter::Debug);

    Ok(())
}
