use crate::composer::exec::{Executor, StreamHandle};
use crate::composer::types::*;
use crate::security;

/// Runner wraps an Executor to provide typed composer commands.
pub struct Runner {
    exec: Box<dyn Executor>,
}

impl Runner {
    pub fn new(exec: Box<dyn Executor>) -> Self {
        Runner { exec }
    }

    /// Checks if the composer binary is reachable.
    pub fn is_available(&self) -> bool {
        self.exec.run(".", &["--version"]).is_ok()
    }

    /// Returns the composer version string.
    pub fn version(&self) -> String {
        match self.exec.run(".", &["--version", "--no-ansi"]) {
            Ok(result) => {
                let raw = String::from_utf8_lossy(&result.stdout).trim().to_string();
                let parts: Vec<&str> = raw.split_whitespace().collect();
                for (i, p) in parts.iter().enumerate() {
                    if *p == "version" && i + 1 < parts.len() {
                        return parts[i + 1].to_string();
                    }
                }
                raw
            }
            Err(_) => "unknown".to_string(),
        }
    }

    /// Returns the absolute path of the composer binary.
    pub fn bin_path(&self) -> String {
        self.exec.look_path()
    }

    /// Runs `composer outdated --format=json --direct` and parses the result.
    pub fn outdated(&self, dir: &str) -> Result<OutdatedResult, String> {
        log::debug!("running: composer outdated --format=json --direct in {dir}");
        let result = self
            .exec
            .run(dir, &["outdated", "--format=json", "--direct"])
            .map_err(|e| format!("running composer outdated: {e}"))?;

        log::debug!(
            "composer outdated exit={} stdout={} bytes",
            result.exit_code,
            result.stdout.len()
        );

        if result.stdout.is_empty() {
            return Ok(OutdatedResult { installed: vec![] });
        }

        serde_json::from_slice(&result.stdout).map_err(|e| {
            log::error!(
                "composer outdated parse error: {e}\nraw output: {}",
                security::sanitize_log_output(&String::from_utf8_lossy(&result.stdout))
            );
            format!("parsing outdated output: {e}")
        })
    }

    /// Runs `composer audit --format=json` and parses the result.
    pub fn audit(&self, dir: &str) -> Result<AuditResult, String> {
        log::debug!("running: composer audit --format=json in {dir}");
        let result = self
            .exec
            .run(dir, &["audit", "--format=json"])
            .map_err(|e| format!("running composer audit: {e}"))?;

        log::debug!(
            "composer audit exit={} stdout={} bytes",
            result.exit_code,
            result.stdout.len()
        );

        if result.stdout.is_empty() {
            return Ok(AuditResult::default());
        }

        serde_json::from_slice(&result.stdout).map_err(|e| {
            log::error!(
                "composer audit parse error: {e}\nraw output: {}",
                security::sanitize_log_output(&String::from_utf8_lossy(&result.stdout))
            );
            format!("parsing audit output: {e}")
        })
    }

    /// Runs `composer show <pkg> --format=json` and parses the result.
    pub fn show(&self, dir: &str, pkg: &str) -> Result<ShowResult, String> {
        log::debug!("running: composer show {pkg} --format=json in {dir}");
        let result = self
            .exec
            .run(dir, &["show", pkg, "--format=json"])
            .map_err(|e| format!("running composer show: {e}"))?;

        log::debug!(
            "composer show exit={} stdout={} bytes",
            result.exit_code,
            result.stdout.len()
        );

        if result.stdout.is_empty() {
            return Err(format!("composer show {pkg}: no output"));
        }

        serde_json::from_slice(&result.stdout).map_err(|e| {
            log::error!(
                "composer show parse error: {e}\nraw output: {}",
                security::sanitize_log_output(&String::from_utf8_lossy(&result.stdout))
            );
            format!("parsing show output: {e}")
        })
    }

    /// Runs `composer show <pkg> --all --format=json` and parses the result.
    /// Returns all available versions (not just the installed one).
    pub fn show_all(&self, dir: &str, pkg: &str) -> Result<ShowResult, String> {
        log::debug!("running: composer show {pkg} --all --format=json in {dir}");
        let result = self
            .exec
            .run(dir, &["show", pkg, "--all", "--format=json"])
            .map_err(|e| format!("running composer show --all: {e}"))?;

        log::debug!(
            "composer show --all exit={} stdout={} bytes",
            result.exit_code,
            result.stdout.len()
        );

        if result.stdout.is_empty() {
            return Err(format!("composer show --all {pkg}: no output"));
        }

        serde_json::from_slice(&result.stdout).map_err(|e| {
            log::error!(
                "composer show --all parse error: {e}\nraw output: {}",
                security::sanitize_log_output(&String::from_utf8_lossy(&result.stdout))
            );
            format!("parsing show --all output: {e}")
        })
    }

    /// Runs `composer require <pkg>` with streaming output.
    pub fn require(&self, dir: &str, pkg: &str) -> Result<StreamHandle, String> {
        log::info!("running: composer require {pkg} in {dir}");
        self.exec.stream(dir, &["require", pkg])
    }

    /// Runs `composer remove <pkg>` with streaming output.
    pub fn remove(&self, dir: &str, pkg: &str) -> Result<StreamHandle, String> {
        log::info!("running: composer remove {pkg} in {dir}");
        self.exec.stream(dir, &["remove", pkg])
    }

    /// Runs `composer update <pkg>` with streaming output.
    /// If pkg is empty, updates all packages.
    pub fn update(&self, dir: &str, pkg: &str) -> Result<StreamHandle, String> {
        let mut args = vec!["update", "--no-interaction"];
        if !pkg.is_empty() {
            args.push(pkg);
        }
        log::info!("running: composer {} in {dir}", args.join(" "));
        self.exec.stream(dir, &args)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc;

    use super::*;
    use crate::composer::exec::{RunResult, StreamLine};

    struct MockExecutor {
        run_fn: Box<dyn Fn(&str, &[&str]) -> Result<RunResult, String> + Send + Sync>,
        stream_fn: Box<dyn Fn(&str, &[&str]) -> Result<StreamHandle, String> + Send + Sync>,
    }

    impl Executor for MockExecutor {
        fn run(&self, dir: &str, args: &[&str]) -> Result<RunResult, String> {
            (self.run_fn)(dir, args)
        }

        fn stream(&self, dir: &str, args: &[&str]) -> Result<StreamHandle, String> {
            (self.stream_fn)(dir, args)
        }

        fn bin(&self) -> String {
            "composer".to_string()
        }

        fn look_path(&self) -> String {
            "/usr/bin/composer".to_string()
        }
    }

    fn mock_run(
        f: impl Fn(&str, &[&str]) -> Result<RunResult, String> + Send + Sync + 'static,
    ) -> MockExecutor {
        MockExecutor {
            run_fn: Box::new(f),
            stream_fn: Box::new(|_, _| Err("not implemented".to_string())),
        }
    }

    fn mock_stream(
        f: impl Fn(&str, &[&str]) -> Result<StreamHandle, String> + Send + Sync + 'static,
    ) -> MockExecutor {
        MockExecutor {
            run_fn: Box::new(|_, _| Err("not implemented".to_string())),
            stream_fn: Box::new(f),
        }
    }

    #[test]
    fn runner_outdated() {
        let result = OutdatedResult {
            installed: vec![OutdatedPackage {
                name: "symfony/framework-bundle".to_string(),
                version: "v7.0.4".to_string(),
                latest: "v7.1.0".to_string(),
                latest_status: "semver-safe-update".to_string(),
                description: "Provides a tight integration".to_string(),
                ..Default::default()
            }],
        };
        let data = serde_json::to_vec(&result).unwrap();

        let runner = Runner::new(Box::new(mock_run(move |_, _| {
            Ok(RunResult {
                stdout: data.clone(),
                stderr: vec![],
                exit_code: 0,
            })
        })));

        let out = runner.outdated("/test").unwrap();
        assert_eq!(out.installed.len(), 1);
        assert_eq!(out.installed[0].name, "symfony/framework-bundle");
        assert_eq!(out.installed[0].latest, "v7.1.0");
    }

    #[test]
    fn runner_audit() {
        let result = AuditResult {
            advisories: [(
                "some/package".to_string(),
                vec![Advisory {
                    advisory_id: "ADV-001".to_string(),
                    package_name: "some/package".to_string(),
                    title: "Critical vulnerability".to_string(),
                    cve: Some("CVE-2024-0001".to_string()),
                    ..Default::default()
                }],
            )]
            .into_iter()
            .collect(),
            abandoned: Default::default(),
        };
        let data = serde_json::to_vec(&result).unwrap();

        let runner = Runner::new(Box::new(mock_run(move |_, _| {
            Ok(RunResult {
                stdout: data.clone(),
                stderr: vec![],
                exit_code: 0,
            })
        })));

        let out = runner.audit("/test").unwrap();
        let advisories = &out.advisories["some/package"];
        assert_eq!(advisories.len(), 1);
        assert_eq!(advisories[0].cve.as_deref(), Some("CVE-2024-0001"));
    }

    #[test]
    fn runner_update() {
        let runner = Runner::new(Box::new(mock_stream(|_, _| {
            let (tx, rx) = mpsc::channel();
            tx.send(StreamLine {
                text: "Loading composer repositories...".to_string(),
                err: None,
                done: false,
            })
            .unwrap();
            tx.send(StreamLine {
                text: "Updating symfony/framework-bundle".to_string(),
                err: None,
                done: false,
            })
            .unwrap();
            tx.send(StreamLine {
                text: String::new(),
                err: None,
                done: true,
            })
            .unwrap();
            Ok(StreamHandle {
                rx,
                child_pid: None,
            })
        })));

        let handle = runner.update("/test", "symfony/framework-bundle").unwrap();
        let mut lines = 0;
        for line in handle.rx {
            if !line.done {
                lines += 1;
            }
        }
        assert_eq!(lines, 2);
    }

    #[test]
    fn runner_is_available() {
        let runner = Runner::new(Box::new(mock_run(|_, _| {
            Ok(RunResult {
                stdout: b"Composer version 2.7.0".to_vec(),
                stderr: vec![],
                exit_code: 0,
            })
        })));
        assert!(runner.is_available());
    }

    #[test]
    fn runner_is_available_not_found() {
        let runner = Runner::new(Box::new(mock_run(
            |_, _| Err("exec: not found".to_string()),
        )));
        assert!(!runner.is_available());
    }

    #[test]
    fn runner_outdated_error() {
        let runner = Runner::new(Box::new(mock_run(|_, _| Err("network error".to_string()))));
        assert!(runner.outdated("/test").is_err());
    }

    #[test]
    fn runner_outdated_invalid_json() {
        let runner = Runner::new(Box::new(mock_run(|_, _| {
            Ok(RunResult {
                stdout: b"not json".to_vec(),
                stderr: vec![],
                exit_code: 0,
            })
        })));
        assert!(runner.outdated("/test").is_err());
    }

    #[test]
    fn runner_audit_error() {
        let runner = Runner::new(Box::new(mock_run(|_, _| Err("network error".to_string()))));
        assert!(runner.audit("/test").is_err());
    }

    #[test]
    fn runner_audit_invalid_json() {
        let runner = Runner::new(Box::new(mock_run(|_, _| {
            Ok(RunResult {
                stdout: b"not json".to_vec(),
                stderr: vec![],
                exit_code: 0,
            })
        })));
        assert!(runner.audit("/test").is_err());
    }

    #[test]
    fn runner_require() {
        let runner = Runner::new(Box::new(mock_stream(|_, _| {
            let (tx, rx) = mpsc::channel();
            tx.send(StreamLine {
                text: "Installing vendor/pkg".to_string(),
                err: None,
                done: false,
            })
            .unwrap();
            tx.send(StreamLine {
                text: String::new(),
                err: None,
                done: true,
            })
            .unwrap();
            Ok(StreamHandle {
                rx,
                child_pid: None,
            })
        })));

        let handle = runner.require("/test", "vendor/pkg").unwrap();
        let mut lines = 0;
        for line in handle.rx {
            if !line.done {
                lines += 1;
            }
        }
        assert_eq!(lines, 1);
    }

    #[test]
    fn runner_require_error() {
        let runner = Runner::new(Box::new(mock_stream(
            |_, _| Err("stream error".to_string()),
        )));
        assert!(runner.require("/test", "vendor/pkg").is_err());
    }

    #[test]
    fn runner_remove() {
        let runner = Runner::new(Box::new(mock_stream(|_, _| {
            let (tx, rx) = mpsc::channel();
            tx.send(StreamLine {
                text: "Removing vendor/pkg".to_string(),
                err: None,
                done: false,
            })
            .unwrap();
            tx.send(StreamLine {
                text: String::new(),
                err: None,
                done: true,
            })
            .unwrap();
            Ok(StreamHandle {
                rx,
                child_pid: None,
            })
        })));

        let handle = runner.remove("/test", "vendor/pkg").unwrap();
        let mut lines = 0;
        for line in handle.rx {
            if !line.done {
                lines += 1;
            }
        }
        assert_eq!(lines, 1);
    }

    #[test]
    fn runner_remove_error() {
        let runner = Runner::new(Box::new(mock_stream(
            |_, _| Err("stream error".to_string()),
        )));
        assert!(runner.remove("/test", "vendor/pkg").is_err());
    }

    #[test]
    fn runner_update_all() {
        let runner = Runner::new(Box::new(mock_stream(|_, _| {
            let (tx, rx) = mpsc::channel();
            tx.send(StreamLine {
                text: "Updating all".to_string(),
                err: None,
                done: false,
            })
            .unwrap();
            tx.send(StreamLine {
                text: String::new(),
                err: None,
                done: true,
            })
            .unwrap();
            Ok(StreamHandle {
                rx,
                child_pid: None,
            })
        })));

        let handle = runner.update("/test", "").unwrap();
        let mut lines = 0;
        for line in handle.rx {
            if !line.done {
                lines += 1;
            }
        }
        assert_eq!(lines, 1);
    }

    #[test]
    fn runner_update_error() {
        let runner = Runner::new(Box::new(mock_stream(
            |_, _| Err("stream error".to_string()),
        )));
        assert!(runner.update("/test", "vendor/pkg").is_err());
    }

    #[test]
    fn runner_outdated_empty_result() {
        let runner = Runner::new(Box::new(mock_run(|_, _| {
            Ok(RunResult {
                stdout: br#"{"installed":[]}"#.to_vec(),
                stderr: vec![],
                exit_code: 0,
            })
        })));
        let out = runner.outdated("/test").unwrap();
        assert_eq!(out.installed.len(), 0);
    }

    #[test]
    fn runner_show() {
        let json = br#"{
            "name": "vendor/pkg",
            "description": "A package",
            "keywords": ["test"],
            "type": "library",
            "homepage": "https://example.com",
            "versions": ["v1.0.0"],
            "licenses": [{"name": "MIT License", "osi": "MIT"}],
            "source": {"type": "git", "url": "https://github.com/vendor/pkg.git", "reference": "abc123"},
            "path": "/vendor/vendor/pkg",
            "released": "2026-01-01T00:00:00+00:00",
            "requires": {"php": ">=8.2"},
            "devRequires": {},
            "conflicts": {}
        }"#;

        let runner = Runner::new(Box::new(mock_run(move |_, _| {
            Ok(RunResult {
                stdout: json.to_vec(),
                stderr: vec![],
                exit_code: 0,
            })
        })));

        let result = runner.show("/test", "vendor/pkg").unwrap();
        assert_eq!(result.name, "vendor/pkg");
        assert_eq!(result.description, "A package");
        assert_eq!(result.pkg_type, "library");
        assert_eq!(result.versions, vec!["v1.0.0"]);
        assert_eq!(result.licenses.len(), 1);
        assert_eq!(result.licenses[0].osi, "MIT");
        assert_eq!(result.requires.len(), 1);
        assert!(result.requires.contains_key("php"));
        assert!(!result.released.is_empty());
    }

    #[test]
    fn runner_show_error() {
        let runner = Runner::new(Box::new(mock_run(|_, _| Err("network error".to_string()))));
        assert!(runner.show("/test", "vendor/pkg").is_err());
    }

    #[test]
    fn runner_show_empty_output() {
        let runner = Runner::new(Box::new(mock_run(|_, _| {
            Ok(RunResult {
                stdout: vec![],
                stderr: vec![],
                exit_code: 1,
            })
        })));
        let err = runner.show("/test", "vendor/pkg").unwrap_err();
        assert!(err.contains("no output"));
    }

    #[test]
    fn runner_show_invalid_json() {
        let runner = Runner::new(Box::new(mock_run(|_, _| {
            Ok(RunResult {
                stdout: b"not json".to_vec(),
                stderr: vec![],
                exit_code: 0,
            })
        })));
        assert!(runner.show("/test", "vendor/pkg").is_err());
    }

    #[test]
    fn runner_show_null_fields() {
        let json = br#"{
            "name": "vendor/pkg",
            "description": null,
            "keywords": [],
            "type": "library",
            "homepage": null,
            "versions": ["v1.0.0"],
            "licenses": [],
            "source": {"type": "git", "url": "https://example.com", "reference": "abc"},
            "path": null,
            "released": null,
            "requires": {},
            "devRequires": {},
            "conflicts": {}
        }"#;

        let runner = Runner::new(Box::new(mock_run(move |_, _| {
            Ok(RunResult {
                stdout: json.to_vec(),
                stderr: vec![],
                exit_code: 0,
            })
        })));

        let result = runner.show("/test", "vendor/pkg").unwrap();
        assert_eq!(result.name, "vendor/pkg");
        assert_eq!(result.description, "");
        assert_eq!(result.homepage, "");
        assert_eq!(result.path, "");
        assert_eq!(result.released, "");
    }

    #[test]
    fn runner_show_all() {
        let json = br#"{
            "name": "symfony/console",
            "versions": ["v7.4.7", "v7.4.6", "v7.0.5", "v7.0.4", "v6.4.0"]
        }"#;

        let runner = Runner::new(Box::new(mock_run(move |_, _| {
            Ok(RunResult {
                stdout: json.to_vec(),
                stderr: vec![],
                exit_code: 0,
            })
        })));

        let result = runner.show_all("/test", "symfony/console").unwrap();
        assert_eq!(result.name, "symfony/console");
        assert_eq!(result.versions.len(), 5);
    }

    #[test]
    fn runner_audit_empty_result() {
        let runner = Runner::new(Box::new(mock_run(|_, _| {
            Ok(RunResult {
                stdout: br#"{"advisories":{},"abandoned":{}}"#.to_vec(),
                stderr: vec![],
                exit_code: 0,
            })
        })));
        let out = runner.audit("/test").unwrap();
        assert_eq!(out.advisories.len(), 0);
        assert_eq!(out.abandoned.len(), 0);
    }
}
