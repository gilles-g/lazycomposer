use crate::composer::{AuditResult, OutdatedResult, Package, StreamLine};
use std::sync::mpsc;

/// Action represents commands that the UI can trigger.
#[derive(Debug)]
pub enum Action {
    None,
    Quit,
    RunRequire(String),
    RunRemove(String),
    RunUpdate(String),
    RunUpdateAll,
    InputSubmit(String),
    InputCancel,
    SwitchTab(usize),
    Refresh,
}

/// Messages sent asynchronously to the app.
pub enum AppMsg {
    PackagesLoaded {
        packages: Vec<Package>,
        lock_hash: String,
        err: Option<String>,
    },
    OutdatedLoaded {
        result: Option<OutdatedResult>,
        err: Option<String>,
    },
    AuditLoaded {
        result: Option<AuditResult>,
        err: Option<String>,
    },
    CommandOutput {
        line: String,
    },
    CommandFinished {
        err: Option<String>,
    },
    CommandStreamStarted {
        rx: mpsc::Receiver<StreamLine>,
        title: String,
    },
    ComposerInfo {
        version: String,
        path: String,
    },
}
