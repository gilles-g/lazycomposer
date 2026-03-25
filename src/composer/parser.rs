use std::fs;
use std::path::Path;

use crate::composer::types::*;

/// Parser reads and parses composer.json and composer.lock files.
pub struct Parser;

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser {
    pub fn new() -> Self {
        Parser
    }

    /// Reads and parses composer.json from the given directory.
    pub fn parse_json(&self, dir: &str) -> Result<ComposerJSON, String> {
        let path = Path::new(dir).join("composer.json");
        let data = fs::read_to_string(&path).map_err(|e| format!("reading composer.json: {e}"))?;
        serde_json::from_str(&data).map_err(|e| format!("parsing composer.json: {e}"))
    }

    /// Reads and parses composer.lock from the given directory.
    pub fn parse_lock(&self, dir: &str) -> Result<ComposerLock, String> {
        let path = Path::new(dir).join("composer.lock");
        let data = fs::read_to_string(&path).map_err(|e| format!("reading composer.lock: {e}"))?;
        serde_json::from_str(&data).map_err(|e| format!("parsing composer.lock: {e}"))
    }

    /// Combines data from composer.json and composer.lock into a unified list of Package structs.
    pub fn merge_packages(&self, cj: &ComposerJSON, cl: &ComposerLock) -> Vec<Package> {
        let mut packages = Vec::with_capacity(cl.packages.len() + cl.packages_dev.len());

        for lp in &cl.packages {
            if let Some(constraint) = cj.require.get(&lp.name) {
                packages.push(lock_package_to_package(lp, false, constraint));
            }
        }

        for lp in &cl.packages_dev {
            if let Some(constraint) = cj.require_dev.get(&lp.name) {
                packages.push(lock_package_to_package(lp, true, constraint));
            }
        }

        packages
    }
}

/// Parses a version string like "v7.4.3" into (major, minor, patch).
pub fn parse_version(version: &str) -> Option<(u32, u32, u32)> {
    let v = version.strip_prefix('v').unwrap_or(version);
    let parts: Vec<&str> = v.splitn(3, '.').collect();
    let major = parts.first()?.parse().ok()?;
    let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
    let patch = parts
        .get(2)
        .and_then(|s| s.split('-').next()) // strip pre-release suffix
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    Some((major, minor, patch))
}

/// Checks if a version satisfies a Composer-style constraint.
///
/// Supported constraint formats:
/// - `^X.Y`  → >=X.Y.0, <(X+1).0.0   (caret: next major is breaking)
/// - `^X.Y.Z` → >=X.Y.Z, <(X+1).0.0
/// - `~X.Y`  → >=X.Y.0, <X.(Y+1).0   (tilde: next minor is breaking)
/// - `~X.Y.Z` → >=X.Y.Z, <X.(Y+1).0
/// - `X.Y.*` → >=X.Y.0, <X.(Y+1).0   (wildcard on patch)
/// - `X.*`   → >=X.0.0, <(X+1).0.0   (wildcard on minor)
/// - `>=X.Y` → >=X.Y.0               (lower bound only)
///
/// For unparseable constraints, returns true (don't restrict).
pub fn version_satisfies_constraint(version: &str, constraint: &str) -> bool {
    let (major, minor, patch) = match parse_version(version) {
        Some(v) => v,
        None => return true,
    };

    let c = constraint.trim();

    // ^X.Y.Z or ^X.Y — caret range: >=X.Y.Z, <(X+1).0.0
    if let Some(rest) = c.strip_prefix('^') {
        return match parse_version(rest) {
            Some((cm, cmin, cpatch)) => {
                major == cm && (minor > cmin || (minor == cmin && patch >= cpatch))
            }
            None => true,
        };
    }

    // ~X.Y.Z or ~X.Y — tilde range: >=X.Y.Z, <X.(Y+1).0
    if let Some(rest) = c.strip_prefix('~') {
        return match parse_version(rest) {
            Some((cm, cmin, cpatch)) => major == cm && minor == cmin && patch >= cpatch,
            None => true,
        };
    }

    // >=X.Y — lower bound only
    if let Some(rest) = c.strip_prefix(">=") {
        return match parse_version(rest.trim()) {
            Some((cm, cmin, cpatch)) => (major, minor, patch) >= (cm, cmin, cpatch),
            None => true,
        };
    }

    // X.* — wildcard on minor
    if let Some(prefix) = c.strip_suffix(".*") {
        let parts: Vec<&str> = prefix.splitn(2, '.').collect();
        return match parts.as_slice() {
            // X.Y.* → same as ~X.Y.0
            [maj_s, min_s] => {
                let cm: u32 = match maj_s.parse() {
                    Ok(v) => v,
                    Err(_) => return true,
                };
                let cmin: u32 = match min_s.parse() {
                    Ok(v) => v,
                    Err(_) => return true,
                };
                major == cm && minor == cmin
            }
            // X.* → same as ^X.0.0
            [maj_s] => {
                let cm: u32 = match maj_s.parse() {
                    Ok(v) => v,
                    Err(_) => return true,
                };
                major == cm
            }
            _ => true,
        };
    }

    // Exact version (fallback): X.Y.Z
    match parse_version(c) {
        Some((cm, cmin, cpatch)) => major == cm && minor == cmin && patch == cpatch,
        None => true,
    }
}

/// Returns a human-readable explanation of a Composer constraint's bounds.
/// e.g. "^7.4" → ">=7.4.0, <8.0.0"
pub fn explain_constraint(constraint: &str) -> String {
    let c = constraint.trim();

    if let Some(rest) = c.strip_prefix('^') {
        if let Some((major, minor, patch)) = parse_version(rest) {
            return format!(">={major}.{minor}.{patch}, <{}.0.0", major + 1);
        }
    }

    if let Some(rest) = c.strip_prefix('~') {
        if let Some((major, minor, patch)) = parse_version(rest) {
            return format!(">={major}.{minor}.{patch}, <{major}.{}.0", minor + 1);
        }
    }

    if let Some(rest) = c.strip_prefix(">=") {
        if let Some((major, minor, patch)) = parse_version(rest.trim()) {
            return format!(">={major}.{minor}.{patch}");
        }
    }

    if let Some(prefix) = c.strip_suffix(".*") {
        let parts: Vec<&str> = prefix.splitn(2, '.').collect();
        match parts.as_slice() {
            [maj_s, min_s] => {
                if let (Ok(major), Ok(minor)) = (maj_s.parse::<u32>(), min_s.parse::<u32>()) {
                    return format!(">={major}.{minor}.0, <{major}.{}.0", minor + 1);
                }
            }
            [maj_s] => {
                if let Ok(major) = maj_s.parse::<u32>() {
                    return format!(">={major}.0.0, <{}.0.0", major + 1);
                }
            }
            _ => {}
        }
    }

    c.to_string()
}

/// Checks if a package name belongs to the Symfony ecosystem.
pub fn is_symfony_package(name: &str) -> bool {
    name.starts_with("symfony/")
}

/// Checks if a version satisfies the framework constraint.
pub fn version_within_framework(version: &str, constraint: &str) -> bool {
    version_satisfies_constraint(version, constraint)
}

/// Analyzes the raw extra field and detects the framework.
pub fn detect_framework(extra: &serde_json::Value) -> Option<FrameworkInfo> {
    if let Some(symfony) = extra.get("symfony").and_then(|v| v.as_object()) {
        let require = symfony
            .get("require")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let allow_contrib = symfony.get("allow-contrib").and_then(|v| v.as_bool());
        let docker = symfony.get("docker").and_then(|v| v.as_bool());
        return Some(FrameworkInfo::Symfony(SymfonyExtra {
            require,
            allow_contrib,
            docker,
        }));
    }
    None
}

fn lock_package_to_package(lp: &LockPackage, is_dev: bool, constraint: &str) -> Package {
    let license = if lp.license.is_empty() {
        String::new()
    } else {
        lp.license.join(", ")
    };

    Package {
        name: lp.name.clone(),
        version: lp.version.clone(),
        constraint: constraint.to_string(),
        description: lp.description.clone(),
        pkg_type: lp.pkg_type.clone(),
        license,
        homepage: lp.homepage.clone(),
        source: lp.source.clone(),
        is_dev,
        status: PackageStatus::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn testdata_dir() -> String {
        format!("{}/testdata", env!("CARGO_MANIFEST_DIR"))
    }

    #[test]
    fn parse_json() {
        let p = Parser::new();
        let cj = p.parse_json(&testdata_dir()).unwrap();
        assert_eq!(cj.name, "test/project");
        assert_eq!(cj.require.len(), 4);
        assert_eq!(cj.require_dev.len(), 2);
        assert_eq!(cj.require["symfony/framework-bundle"], "^7.0");
        assert!(cj.extra.is_some());
    }

    #[test]
    fn parse_lock() {
        let p = Parser::new();
        let cl = p.parse_lock(&testdata_dir()).unwrap();
        assert_eq!(cl.packages.len(), 3);
        assert_eq!(cl.packages_dev.len(), 2);
        assert_eq!(cl.packages[0].name, "symfony/framework-bundle");
    }

    #[test]
    fn merge_packages() {
        let p = Parser::new();
        let cj = p.parse_json(&testdata_dir()).unwrap();
        let cl = p.parse_lock(&testdata_dir()).unwrap();
        let packages = p.merge_packages(&cj, &cl);

        // php is in require but not in lock, so 3 require + 2 require-dev = 5
        assert_eq!(packages.len(), 5);

        let dev_count = packages.iter().filter(|p| p.is_dev).count();
        assert_eq!(dev_count, 2);

        let sfb = packages
            .iter()
            .find(|p| p.name == "symfony/framework-bundle")
            .expect("symfony/framework-bundle not found");
        assert_eq!(sfb.version, "v7.0.4");
        assert!(!sfb.is_dev);
    }

    #[test]
    fn parse_json_not_found() {
        let p = Parser::new();
        assert!(p.parse_json("/nonexistent").is_err());
    }

    #[test]
    fn parse_lock_not_found() {
        let p = Parser::new();
        assert!(p.parse_lock("/nonexistent").is_err());
    }

    #[test]
    fn parse_json_invalid() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("composer.json"), "not json").unwrap();
        let p = Parser::new();
        assert!(p.parse_json(dir.path().to_str().unwrap()).is_err());
    }

    #[test]
    fn parse_lock_invalid() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("composer.lock"), "not json").unwrap();
        let p = Parser::new();
        assert!(p.parse_lock(dir.path().to_str().unwrap()).is_err());
    }

    #[test]
    fn merge_packages_empty_lock() {
        let p = Parser::new();
        let cj = ComposerJSON {
            name: String::new(),
            description: String::new(),
            pkg_type: String::new(),
            license: String::new(),
            require: [("vendor/pkg".to_string(), "^1.0".to_string())]
                .into_iter()
                .collect(),
            require_dev: Default::default(),
            autoload: Default::default(),
            autoload_dev: Default::default(),
            extra: None,
        };
        let cl = ComposerLock {
            packages: vec![],
            packages_dev: vec![],
            content_hash: String::new(),
        };
        assert_eq!(p.merge_packages(&cj, &cl).len(), 0);
    }

    #[test]
    fn merge_packages_only_dev() {
        let p = Parser::new();
        let cj = ComposerJSON {
            name: String::new(),
            description: String::new(),
            pkg_type: String::new(),
            license: String::new(),
            require: Default::default(),
            require_dev: [("phpunit/phpunit".to_string(), "^11.0".to_string())]
                .into_iter()
                .collect(),
            autoload: Default::default(),
            autoload_dev: Default::default(),
            extra: None,
        };
        let cl = ComposerLock {
            packages: vec![],
            packages_dev: vec![LockPackage {
                name: "phpunit/phpunit".to_string(),
                version: "11.0.0".to_string(),
                license: vec!["BSD-3-Clause".to_string()],
                ..Default::default()
            }],
            content_hash: String::new(),
        };
        let packages = p.merge_packages(&cj, &cl);
        assert_eq!(packages.len(), 1);
        assert!(packages[0].is_dev);
        assert_eq!(packages[0].license, "BSD-3-Clause");
    }

    #[test]
    fn merge_packages_multiple_licenses() {
        let p = Parser::new();
        let cj = ComposerJSON {
            name: String::new(),
            description: String::new(),
            pkg_type: String::new(),
            license: String::new(),
            require: [("vendor/pkg".to_string(), "^1.0".to_string())]
                .into_iter()
                .collect(),
            require_dev: Default::default(),
            autoload: Default::default(),
            autoload_dev: Default::default(),
            extra: None,
        };
        let cl = ComposerLock {
            packages: vec![LockPackage {
                name: "vendor/pkg".to_string(),
                version: "1.0.0".to_string(),
                license: vec!["MIT".to_string(), "Apache-2.0".to_string()],
                ..Default::default()
            }],
            packages_dev: vec![],
            content_hash: String::new(),
        };
        let packages = p.merge_packages(&cj, &cl);
        assert_eq!(packages[0].license, "MIT, Apache-2.0");
    }

    #[test]
    fn merge_packages_no_license() {
        let p = Parser::new();
        let cj = ComposerJSON {
            name: String::new(),
            description: String::new(),
            pkg_type: String::new(),
            license: String::new(),
            require: [("vendor/pkg".to_string(), "^1.0".to_string())]
                .into_iter()
                .collect(),
            require_dev: Default::default(),
            autoload: Default::default(),
            autoload_dev: Default::default(),
            extra: None,
        };
        let cl = ComposerLock {
            packages: vec![LockPackage {
                name: "vendor/pkg".to_string(),
                version: "1.0.0".to_string(),
                license: vec![],
                ..Default::default()
            }],
            packages_dev: vec![],
            content_hash: String::new(),
        };
        let packages = p.merge_packages(&cj, &cl);
        assert_eq!(packages[0].license, "");
    }

    #[test]
    fn merge_packages_transitive_filtered() {
        let p = Parser::new();
        let cj = ComposerJSON {
            name: String::new(),
            description: String::new(),
            pkg_type: String::new(),
            license: String::new(),
            require: [("vendor/pkg".to_string(), "^1.0".to_string())]
                .into_iter()
                .collect(),
            require_dev: Default::default(),
            autoload: Default::default(),
            autoload_dev: Default::default(),
            extra: None,
        };
        let cl = ComposerLock {
            packages: vec![
                LockPackage {
                    name: "vendor/pkg".to_string(),
                    version: "1.0.0".to_string(),
                    ..Default::default()
                },
                LockPackage {
                    name: "vendor/transitive".to_string(),
                    version: "2.0.0".to_string(),
                    ..Default::default()
                },
            ],
            packages_dev: vec![],
            content_hash: String::new(),
        };
        let packages = p.merge_packages(&cj, &cl);
        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "vendor/pkg");
    }

    #[test]
    fn merge_packages_source_info() {
        let p = Parser::new();
        let cj = ComposerJSON {
            name: String::new(),
            description: String::new(),
            pkg_type: String::new(),
            license: String::new(),
            require: [("vendor/pkg".to_string(), "^1.0".to_string())]
                .into_iter()
                .collect(),
            require_dev: Default::default(),
            autoload: Default::default(),
            autoload_dev: Default::default(),
            extra: None,
        };
        let cl = ComposerLock {
            packages: vec![LockPackage {
                name: "vendor/pkg".to_string(),
                version: "1.0.0".to_string(),
                source: Source {
                    source_type: "git".to_string(),
                    url: "https://github.com/vendor/pkg.git".to_string(),
                    reference: "abc123".to_string(),
                },
                ..Default::default()
            }],
            packages_dev: vec![],
            content_hash: String::new(),
        };
        let packages = p.merge_packages(&cj, &cl);
        assert_eq!(packages[0].source.url, "https://github.com/vendor/pkg.git");
        assert_eq!(packages[0].source.reference, "abc123");
    }

    #[test]
    fn detect_framework_symfony() {
        let extra: serde_json::Value = serde_json::from_str(
            r#"{"symfony": {"require": "7.0.*", "allow-contrib": false, "docker": true}}"#,
        )
        .unwrap();
        let fw = detect_framework(&extra).unwrap();
        match fw {
            FrameworkInfo::Symfony(sf) => {
                assert_eq!(sf.require, "7.0.*");
                assert_eq!(sf.allow_contrib, Some(false));
                assert_eq!(sf.docker, Some(true));
            }
        }
    }

    #[test]
    fn detect_framework_symfony_partial() {
        let extra: serde_json::Value =
            serde_json::from_str(r#"{"symfony": {"require": "6.4.*"}}"#).unwrap();
        let fw = detect_framework(&extra).unwrap();
        match fw {
            FrameworkInfo::Symfony(sf) => {
                assert_eq!(sf.require, "6.4.*");
                assert_eq!(sf.allow_contrib, None);
                assert_eq!(sf.docker, None);
            }
        }
    }

    #[test]
    fn detect_framework_none() {
        let extra: serde_json::Value =
            serde_json::from_str(r#"{"branch-alias": {"dev-main": "1.0-dev"}}"#).unwrap();
        assert!(detect_framework(&extra).is_none());
    }

    #[test]
    fn detect_framework_empty_extra() {
        let extra: serde_json::Value = serde_json::from_str(r#"{}"#).unwrap();
        assert!(detect_framework(&extra).is_none());
    }

    #[test]
    fn detect_framework_from_testdata() {
        let p = Parser::new();
        let cj = p.parse_json(&testdata_dir()).unwrap();
        let fw = detect_framework(cj.extra.as_ref().unwrap()).unwrap();
        match fw {
            FrameworkInfo::Symfony(sf) => {
                assert_eq!(sf.require, "7.0.*");
                assert_eq!(sf.allow_contrib, Some(false));
                assert_eq!(sf.docker, Some(true));
            }
        }
    }

    #[test]
    fn parse_version_test() {
        assert_eq!(parse_version("v7.4.3"), Some((7, 4, 3)));
        assert_eq!(parse_version("8.0.1"), Some((8, 0, 1)));
        assert_eq!(parse_version("v7.0"), Some((7, 0, 0)));
        assert_eq!(parse_version("7"), Some((7, 0, 0)));
        assert_eq!(parse_version("dev-main"), None);
    }

    #[test]
    fn is_symfony_package_test() {
        assert!(is_symfony_package("symfony/framework-bundle"));
        assert!(is_symfony_package("symfony/console"));
        assert!(!is_symfony_package("doctrine/orm"));
        assert!(!is_symfony_package("twig/twig"));
    }

    // --- Caret (^) constraint tests ---

    #[test]
    fn caret_allows_same_major() {
        // ^7.0 means >=7.0.0, <8.0.0
        assert!(version_satisfies_constraint("v7.0.0", "^7.0"));
        assert!(version_satisfies_constraint("v7.4.3", "^7.0"));
        assert!(version_satisfies_constraint("v7.99.0", "^7.0"));
    }

    #[test]
    fn caret_blocks_next_major() {
        assert!(!version_satisfies_constraint("v8.0.0", "^7.0"));
        assert!(!version_satisfies_constraint("v9.0.0", "^7.4"));
    }

    #[test]
    fn caret_blocks_lower_minor() {
        // ^7.4 means >=7.4.0 — so 7.3.x is too low
        assert!(!version_satisfies_constraint("v7.3.0", "^7.4"));
        assert!(!version_satisfies_constraint("v7.3.99", "^7.4"));
    }

    #[test]
    fn caret_with_patch() {
        // ^7.4.2 means >=7.4.2, <8.0.0
        assert!(version_satisfies_constraint("v7.4.2", "^7.4.2"));
        assert!(version_satisfies_constraint("v7.4.5", "^7.4.2"));
        assert!(version_satisfies_constraint("v7.5.0", "^7.4.2"));
        assert!(!version_satisfies_constraint("v7.4.1", "^7.4.2"));
        assert!(!version_satisfies_constraint("v8.0.0", "^7.4.2"));
    }

    // --- Tilde (~) constraint tests ---

    #[test]
    fn tilde_allows_same_minor() {
        // ~7.4 means >=7.4.0, <7.5.0
        assert!(version_satisfies_constraint("v7.4.0", "~7.4"));
        assert!(version_satisfies_constraint("v7.4.9", "~7.4"));
    }

    #[test]
    fn tilde_blocks_next_minor() {
        assert!(!version_satisfies_constraint("v7.5.0", "~7.4"));
        assert!(!version_satisfies_constraint("v8.0.0", "~7.4"));
    }

    #[test]
    fn tilde_blocks_lower_patch() {
        // ~7.4.3 means >=7.4.3, <7.5.0
        assert!(version_satisfies_constraint("v7.4.3", "~7.4.3"));
        assert!(version_satisfies_constraint("v7.4.9", "~7.4.3"));
        assert!(!version_satisfies_constraint("v7.4.2", "~7.4.3"));
        assert!(!version_satisfies_constraint("v7.5.0", "~7.4.3"));
    }

    // --- Wildcard (*) constraint tests ---

    #[test]
    fn wildcard_patch() {
        // 7.4.* means >=7.4.0, <7.5.0
        assert!(version_satisfies_constraint("v7.4.0", "7.4.*"));
        assert!(version_satisfies_constraint("v7.4.99", "7.4.*"));
        assert!(!version_satisfies_constraint("v7.5.0", "7.4.*"));
        assert!(!version_satisfies_constraint("v7.3.0", "7.4.*"));
        assert!(!version_satisfies_constraint("v8.0.0", "7.4.*"));
    }

    #[test]
    fn wildcard_minor() {
        // 7.* means >=7.0.0, <8.0.0
        assert!(version_satisfies_constraint("v7.0.0", "7.*"));
        assert!(version_satisfies_constraint("v7.99.0", "7.*"));
        assert!(!version_satisfies_constraint("v8.0.0", "7.*"));
        assert!(!version_satisfies_constraint("v6.0.0", "7.*"));
    }

    // --- >= constraint tests ---

    #[test]
    fn gte_constraint() {
        assert!(version_satisfies_constraint("v7.4.0", ">=7.4"));
        assert!(version_satisfies_constraint("v8.0.0", ">=7.4"));
        assert!(version_satisfies_constraint("v99.0.0", ">=7.4"));
        assert!(!version_satisfies_constraint("v7.3.9", ">=7.4"));
    }

    // --- Edge cases ---

    #[test]
    fn unparseable_version_allows() {
        assert!(version_satisfies_constraint("dev-main", "^7.0"));
        assert!(version_satisfies_constraint("dev-main", "7.4.*"));
    }

    #[test]
    fn version_within_framework_delegates() {
        assert!(version_within_framework("v7.4.3", "^7.0"));
        assert!(!version_within_framework("v8.0.0", "^7.0"));
        assert!(version_within_framework("v7.4.0", "7.4.*"));
        assert!(!version_within_framework("v7.5.0", "7.4.*"));
    }

    // --- explain_constraint tests ---

    #[test]
    fn explain_caret() {
        assert_eq!(explain_constraint("^7.4"), ">=7.4.0, <8.0.0");
        assert_eq!(explain_constraint("^7.4.2"), ">=7.4.2, <8.0.0");
        assert_eq!(explain_constraint("^11.0"), ">=11.0.0, <12.0.0");
    }

    #[test]
    fn explain_tilde() {
        assert_eq!(explain_constraint("~7.4"), ">=7.4.0, <7.5.0");
        assert_eq!(explain_constraint("~7.4.3"), ">=7.4.3, <7.5.0");
    }

    #[test]
    fn explain_wildcard() {
        assert_eq!(explain_constraint("7.4.*"), ">=7.4.0, <7.5.0");
        assert_eq!(explain_constraint("7.*"), ">=7.0.0, <8.0.0");
    }

    #[test]
    fn explain_gte() {
        assert_eq!(explain_constraint(">=7.4"), ">=7.4.0");
    }

    #[test]
    fn explain_exact() {
        assert_eq!(explain_constraint("7.4.3"), "7.4.3");
    }

    #[test]
    fn explain_unparseable() {
        assert_eq!(explain_constraint("dev-main"), "dev-main");
    }
}
