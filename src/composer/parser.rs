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
            if cj.require.contains_key(&lp.name) {
                packages.push(lock_package_to_package(lp, false));
            }
        }

        for lp in &cl.packages_dev {
            if cj.require_dev.contains_key(&lp.name) {
                packages.push(lock_package_to_package(lp, true));
            }
        }

        packages
    }
}

fn lock_package_to_package(lp: &LockPackage, is_dev: bool) -> Package {
    let license = if lp.license.is_empty() {
        String::new()
    } else {
        lp.license.join(", ")
    };

    Package {
        name: lp.name.clone(),
        version: lp.version.clone(),
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
}
