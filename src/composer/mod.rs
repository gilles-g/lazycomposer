pub mod exec;
pub mod parser;
pub mod runner;
pub mod types;

pub use exec::{Executor, RealExecutor, RunResult, StreamLine};
pub use parser::{
    detect_framework, explain_constraint, find_best_version_in_constraint, is_blocked_by_framework,
    is_symfony_package, version_within_framework, Parser,
};
pub use runner::Runner;
pub use types::*;
