use std::fs;
use std::path::PathBuf;
use thiserror::Error;

/// Config holds application configuration.
#[derive(Debug, Clone)]
pub struct Config {
    pub dir: String,
    pub composer_bin: String,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("composer.json not found in {dir}")]
    NoComposer { dir: String },
    #[error("composer binary {bin:?} not found in PATH")]
    InvalidBin { bin: String },
    #[error("{0}")]
    Other(String),
}

/// Determines the project directory and validates the composer binary.
pub fn resolve(dir: &str) -> Result<Config, ConfigError> {
    let dir = if dir.is_empty() {
        std::env::current_dir()
            .map_err(|e| ConfigError::Other(e.to_string()))?
            .to_string_lossy()
            .to_string()
    } else {
        dir.to_string()
    };

    // Canonicalize: make absolute and resolve symlinks
    let abs_dir =
        fs::canonicalize(&dir).map_err(|e| ConfigError::Other(format!("resolving path: {e}")))?;
    let dir = abs_dir.to_string_lossy().to_string();

    // Verify composer.json exists
    let path = PathBuf::from(&dir).join("composer.json");
    if !path.exists() {
        return Err(ConfigError::NoComposer { dir });
    }

    let composer_bin = std::env::var("COMPOSER_BIN").unwrap_or_else(|_| "composer".to_string());

    // Validate that the main binary is reachable
    let main_bin = composer_bin
        .split_whitespace()
        .next()
        .unwrap_or(&composer_bin);

    if which::which(main_bin).is_err() {
        return Err(ConfigError::InvalidBin { bin: composer_bin });
    }

    Ok(Config { dir, composer_bin })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::os::unix::fs::symlink;

    #[test]
    #[serial]
    fn resolve_with_composer_json() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("composer.json"), "{}").unwrap();

        let cfg = resolve(dir.path().to_str().unwrap()).unwrap();
        // canonicalize may resolve differently, just check it's not empty
        assert!(!cfg.dir.is_empty());
    }

    #[test]
    #[serial]
    fn resolve_missing_composer_json() {
        let dir = tempfile::tempdir().unwrap();
        let result = resolve(dir.path().to_str().unwrap());
        assert!(result.is_err());
        match result.unwrap_err() {
            ConfigError::NoComposer { dir: d } => {
                assert!(d.contains(dir.path().file_name().unwrap().to_str().unwrap()));
            }
            e => panic!("expected NoComposer, got {e:?}"),
        }
    }

    #[test]
    #[serial]
    fn resolve_empty_dir_uses_cwd() {
        let orig = std::env::current_dir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("composer.json"), "{}").unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let result = resolve("");
        std::env::set_current_dir(orig).unwrap();

        assert!(result.is_ok());
    }

    #[test]
    fn no_composer_error_message() {
        let err = ConfigError::NoComposer {
            dir: "/some/path".to_string(),
        };
        assert_eq!(err.to_string(), "composer.json not found in /some/path");
    }

    #[test]
    #[serial]
    fn resolve_canonicalizes_path() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("composer.json"), "{}").unwrap();

        let link_parent = tempfile::tempdir().unwrap();
        let link_path = link_parent.path().join("link");
        symlink(dir.path(), &link_path).unwrap();

        let cfg = resolve(link_path.to_str().unwrap()).unwrap();
        // Dir should be the resolved real path, not the symlink
        let real = fs::canonicalize(dir.path()).unwrap();
        assert_eq!(cfg.dir, real.to_string_lossy());
    }

    #[test]
    #[serial]
    fn resolve_relative_path_becomes_absolute() {
        let orig = std::env::current_dir().unwrap();
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("composer.json"), "{}").unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let result = resolve(".");
        std::env::set_current_dir(orig).unwrap();

        let cfg = result.unwrap();
        assert!(PathBuf::from(&cfg.dir).is_absolute());
    }

    #[test]
    #[serial]
    fn resolve_invalid_composer_bin() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("composer.json"), "{}").unwrap();

        std::env::set_var("COMPOSER_BIN", "nonexistent-binary-xyz-12345");
        let result = resolve(dir.path().to_str().unwrap());
        std::env::remove_var("COMPOSER_BIN");

        match result.unwrap_err() {
            ConfigError::InvalidBin { bin } => {
                assert_eq!(bin, "nonexistent-binary-xyz-12345");
            }
            e => panic!("expected InvalidBin, got {e:?}"),
        }
    }
}
