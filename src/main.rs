use clap::Parser;
use std::process;

/// A TUI for managing PHP Composer dependencies, inspired by lazygit.
#[derive(Parser)]
#[command(about)]
#[command(version = long_version())]
struct Cli {
    /// Path to the project directory (defaults to current directory)
    path: Option<String>,
}

fn long_version() -> &'static str {
    concat!(
        "version=",
        env!("CARGO_PKG_VERSION"),
        ", commit=",
        env!("LC_GIT_COMMIT"),
        ", build date=",
        env!("LC_BUILD_DATE"),
        ", os=",
        env!("LC_OS"),
        ", arch=",
        env!("LC_ARCH"),
    )
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = lazycomposer::logger::init() {
        eprintln!("Warning: could not init logger: {e}");
    }

    let dir = cli.path.unwrap_or_default();

    let cfg = match lazycomposer::config::resolve(&dir) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Error: {e}");
            process::exit(1);
        }
    };

    log::info!("starting lazycomposer in {}", cfg.dir);

    let exec = lazycomposer::composer::RealExecutor::new(&cfg.composer_bin);
    let runner = lazycomposer::composer::Runner::new(Box::new(exec));

    let mut app = lazycomposer::ui::app::App::new(cfg.dir, runner, "dev".to_string());

    if let Err(e) = app.run() {
        log::error!("program error: {e}");
        eprintln!("Error: {e}");
        process::exit(1);
    }
}
