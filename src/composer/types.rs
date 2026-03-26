use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Deserializes a JSON value that can be a string or null into a String.
/// null → "".
fn null_as_empty_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

/// Deserializes a JSON value that can be a string or an array of strings into Vec<String>.
/// `"MIT"` → `vec!["MIT"]`, `["MIT", "Apache-2.0"]` → `vec!["MIT", "Apache-2.0"]`.
fn string_or_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct StringOrVecVisitor;

    impl<'de> Visitor<'de> for StringOrVecVisitor {
        type Value = Vec<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string or an array of strings")
        }

        fn visit_str<E>(self, v: &str) -> Result<Vec<String>, E>
        where
            E: de::Error,
        {
            Ok(vec![v.to_string()])
        }

        fn visit_seq<A>(self, seq: A) -> Result<Vec<String>, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            Vec::deserialize(de::value::SeqAccessDeserializer::new(seq))
        }

        fn visit_none<E>(self) -> Result<Vec<String>, E>
        where
            E: de::Error,
        {
            Ok(Vec::new())
        }

        fn visit_unit<E>(self) -> Result<Vec<String>, E>
        where
            E: de::Error,
        {
            Ok(Vec::new())
        }
    }

    deserializer.deserialize_any(StringOrVecVisitor)
}

/// StringOrBool handles JSON fields that can be either a string or a boolean.
/// Composer outputs `"abandoned": false` or `"abandoned": "replacement/pkg"`.
#[derive(Debug, Clone, Default)]
pub struct StringOrBool {
    pub value: String,
    pub set: bool,
}

impl Serialize for StringOrBool {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        if self.set {
            serializer.serialize_str(&self.value)
        } else {
            serializer.serialize_bool(false)
        }
    }
}

impl<'de> Deserialize<'de> for StringOrBool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StringOrBoolVisitor;

        impl<'de> Visitor<'de> for StringOrBoolVisitor {
            type Value = StringOrBool;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a string, boolean, or null")
            }

            fn visit_bool<E>(self, _v: bool) -> Result<StringOrBool, E>
            where
                E: de::Error,
            {
                Ok(StringOrBool {
                    value: String::new(),
                    set: false,
                })
            }

            fn visit_str<E>(self, v: &str) -> Result<StringOrBool, E>
            where
                E: de::Error,
            {
                Ok(StringOrBool {
                    value: v.to_string(),
                    set: true,
                })
            }

            fn visit_unit<E>(self) -> Result<StringOrBool, E>
            where
                E: de::Error,
            {
                // null → Set=true, Value=""  (matches Go behavior where null unmarshals as zero-value string)
                Ok(StringOrBool {
                    value: String::new(),
                    set: true,
                })
            }
        }

        deserializer.deserialize_any(StringOrBoolVisitor)
    }
}

/// PackageStatus represents the health status of a package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum PackageStatus {
    #[default]
    OK = 0,
    Outdated = 1,
    Abandoned = 2,
    Vulnerable = 3,
    Restricted = 4,
}

/// Package represents a Composer dependency.
#[derive(Debug, Clone, Default)]
pub struct Package {
    pub name: String,
    pub version: String,
    pub constraint: String,
    pub description: String,
    pub pkg_type: String,
    pub license: String,
    pub homepage: String,
    pub source: Source,
    pub is_dev: bool,
    pub status: PackageStatus,
    pub restricted_latest: Option<String>,
}

/// Source holds the source info of a package.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Source {
    #[serde(rename = "type", default)]
    pub source_type: String,
    #[serde(default)]
    pub url: String,
    #[serde(rename = "reference", default)]
    pub reference: String,
}

/// Known Symfony configuration keys from extra.symfony.
#[derive(Debug, Clone, Default)]
pub struct SymfonyExtra {
    pub require: String,
    pub allow_contrib: Option<bool>,
    pub docker: Option<bool>,
}

/// Author of a Composer package.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Author {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub homepage: String,
    #[serde(default)]
    pub role: String,
}

/// Support information for a Composer package.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Support {
    #[serde(default)]
    pub email: String,
    #[serde(default)]
    pub issues: String,
    #[serde(default)]
    pub forum: String,
    #[serde(default)]
    pub wiki: String,
    #[serde(default)]
    pub irc: String,
    #[serde(default)]
    pub chat: String,
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub docs: String,
    #[serde(default)]
    pub rss: String,
    #[serde(default)]
    pub security: String,
}

/// Framework detected from the extra field of composer.json.
#[derive(Debug, Clone)]
pub enum FrameworkInfo {
    Symfony(SymfonyExtra),
}

/// ComposerJSON represents the parsed composer.json file.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ComposerJSON {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "type", default)]
    pub pkg_type: String,
    #[serde(deserialize_with = "string_or_vec", default)]
    pub license: Vec<String>,
    #[serde(default)]
    pub homepage: String,
    #[serde(default)]
    pub authors: Vec<Author>,
    #[serde(default)]
    pub support: Option<Support>,
    #[serde(default)]
    pub require: HashMap<String, String>,
    #[serde(rename = "require-dev", default)]
    pub require_dev: HashMap<String, String>,
    #[serde(default)]
    pub replace: HashMap<String, String>,
    #[serde(default)]
    pub conflict: HashMap<String, String>,
    #[serde(default)]
    pub provide: HashMap<String, String>,
    #[serde(default)]
    pub suggest: HashMap<String, String>,
    #[serde(default)]
    pub repositories: Option<serde_json::Value>,
    #[serde(rename = "minimum-stability", default)]
    pub minimum_stability: String,
    #[serde(rename = "prefer-stable", default)]
    pub prefer_stable: bool,
    #[serde(default)]
    pub autoload: Autoload,
    #[serde(rename = "autoload-dev", default)]
    pub autoload_dev: Autoload,
    #[serde(default)]
    pub scripts: HashMap<String, serde_json::Value>,
    #[serde(default)]
    pub config: Option<serde_json::Value>,
    #[serde(default)]
    pub extra: Option<serde_json::Value>,
}

/// Autoload represents autoloading configuration.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Autoload {
    #[serde(rename = "psr-4", default)]
    pub psr4: HashMap<String, serde_json::Value>,
}

/// ComposerLock represents the parsed composer.lock file.
#[derive(Debug, Clone, Deserialize)]
pub struct ComposerLock {
    #[serde(default)]
    pub packages: Vec<LockPackage>,
    #[serde(rename = "packages-dev", default)]
    pub packages_dev: Vec<LockPackage>,
    #[serde(rename = "content-hash", default)]
    pub content_hash: String,
}

/// LockPackage is a package entry in composer.lock.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct LockPackage {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(rename = "type", default)]
    pub pkg_type: String,
    #[serde(default)]
    pub license: Vec<String>,
    #[serde(default)]
    pub homepage: String,
    #[serde(default)]
    pub source: Source,
}

/// OutdatedResult holds the result of `composer outdated`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OutdatedResult {
    #[serde(default)]
    pub installed: Vec<OutdatedPackage>,
}

/// OutdatedPackage is a single outdated package entry.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct OutdatedPackage {
    #[serde(default)]
    pub name: String,
    #[serde(rename = "direct-dependency", default)]
    pub direct_dep: bool,
    #[serde(default, deserialize_with = "null_as_empty_string")]
    pub homepage: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub latest: String,
    #[serde(rename = "latest-status", default)]
    pub latest_status: String,
    #[serde(default, deserialize_with = "null_as_empty_string")]
    pub description: String,
    #[serde(default, deserialize_with = "null_as_empty_string")]
    pub warning: String,
    #[serde(default)]
    pub abandoned: StringOrBool,
}

/// AuditResult holds the result of `composer audit`.
#[derive(Debug, Clone, Default)]
pub struct AuditResult {
    pub advisories: HashMap<String, Vec<Advisory>>,
    pub abandoned: HashMap<String, Option<String>>,
}

impl<'de> Deserialize<'de> for AuditResult {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawAuditResult {
            #[serde(default)]
            advisories: serde_json::Value,
            #[serde(default)]
            abandoned: serde_json::Value,
        }

        let raw = RawAuditResult::deserialize(deserializer)?;

        let advisories = match raw.advisories {
            serde_json::Value::Object(map) => {
                let mut result: HashMap<String, Vec<Advisory>> = HashMap::new();
                for (pkg_name, value) in map {
                    let advs = match value {
                        // Normal case: array of advisories
                        serde_json::Value::Array(_) => {
                            serde_json::from_value(value).map_err(de::Error::custom)?
                        }
                        // Composer quirk: object with numeric keys instead of array
                        serde_json::Value::Object(inner) => {
                            let mut vec = Vec::new();
                            for (_key, adv_val) in inner {
                                let adv: Advisory =
                                    serde_json::from_value(adv_val).map_err(de::Error::custom)?;
                                vec.push(adv);
                            }
                            vec
                        }
                        _ => Vec::new(),
                    };
                    result.insert(pkg_name, advs);
                }
                result
            }
            _ => HashMap::new(), // [] or null
        };

        let abandoned = match &raw.abandoned {
            serde_json::Value::Object(_) => {
                serde_json::from_value(raw.abandoned).map_err(de::Error::custom)?
            }
            _ => HashMap::new(), // [] or null
        };

        Ok(AuditResult {
            advisories,
            abandoned,
        })
    }
}

impl Serialize for AuditResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut s = serializer.serialize_struct("AuditResult", 2)?;
        s.serialize_field("advisories", &self.advisories)?;
        s.serialize_field("abandoned", &self.abandoned)?;
        s.end()
    }
}

/// ShowLicense represents a license entry from `composer show --format=json`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ShowLicense {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub osi: String,
}

/// ShowResult holds the result of `composer show <pkg> --format=json`.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ShowResult {
    #[serde(default, deserialize_with = "null_as_empty_string")]
    pub name: String,
    #[serde(default, deserialize_with = "null_as_empty_string")]
    pub description: String,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(rename = "type", default, deserialize_with = "null_as_empty_string")]
    pub pkg_type: String,
    #[serde(default, deserialize_with = "null_as_empty_string")]
    pub homepage: String,
    #[serde(default)]
    pub versions: Vec<String>,
    #[serde(default)]
    pub licenses: Vec<ShowLicense>,
    #[serde(default)]
    pub source: Source,
    #[serde(default, deserialize_with = "null_as_empty_string")]
    pub path: String,
    #[serde(default, deserialize_with = "null_as_empty_string")]
    pub released: String,
    #[serde(default)]
    pub requires: HashMap<String, String>,
    #[serde(rename = "devRequires", default)]
    pub dev_requires: HashMap<String, String>,
    #[serde(default)]
    pub conflicts: HashMap<String, String>,
    #[serde(default)]
    pub replaces: HashMap<String, String>,
    #[serde(default)]
    pub provides: HashMap<String, String>,
    #[serde(default)]
    pub suggests: HashMap<String, String>,
}

/// Advisory is a security advisory for a package.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct Advisory {
    #[serde(rename = "advisoryId", default)]
    pub advisory_id: String,
    #[serde(rename = "packageName", default)]
    pub package_name: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub link: String,
    #[serde(default)]
    pub cve: Option<String>,
    #[serde(rename = "affectedVersions", default)]
    pub affected_versions: String,
    #[serde(rename = "reportedAt", default)]
    pub reported_at: String,
    #[serde(default)]
    pub severity: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_or_bool_unmarshal_string() {
        let s: StringOrBool = serde_json::from_str(r#""replacement/pkg""#).unwrap();
        assert!(s.set);
        assert_eq!(s.value, "replacement/pkg");
    }

    #[test]
    fn string_or_bool_unmarshal_bool_false() {
        let s: StringOrBool = serde_json::from_str("false").unwrap();
        assert!(!s.set);
        assert_eq!(s.value, "");
    }

    #[test]
    fn string_or_bool_unmarshal_bool_true() {
        let s: StringOrBool = serde_json::from_str("true").unwrap();
        assert!(!s.set);
    }

    #[test]
    fn string_or_bool_unmarshal_empty_string() {
        let s: StringOrBool = serde_json::from_str(r#""""#).unwrap();
        assert!(s.set);
        assert_eq!(s.value, "");
    }

    #[test]
    fn string_or_bool_unmarshal_null() {
        let s: StringOrBool = serde_json::from_str("null").unwrap();
        assert!(s.set);
        assert_eq!(s.value, "");
    }

    #[test]
    fn string_or_bool_in_struct() {
        #[derive(Deserialize)]
        struct Wrapper {
            abandoned: StringOrBool,
        }

        let tests = vec![
            (r#"{"abandoned":"new/pkg"}"#, true, "new/pkg"),
            (r#"{"abandoned":false}"#, false, ""),
            (r#"{"abandoned":true}"#, false, ""),
        ];

        for (input, want_set, want_val) in tests {
            let w: Wrapper = serde_json::from_str(input).unwrap();
            assert_eq!(w.abandoned.set, want_set, "input={input}");
            assert_eq!(w.abandoned.value, want_val, "input={input}");
        }
    }

    #[test]
    fn outdated_package_unmarshal() {
        let input = r#"{
            "name": "vendor/pkg",
            "direct-dependency": true,
            "version": "1.0.0",
            "latest": "2.0.0",
            "latest-status": "update-possible",
            "description": "A package",
            "abandoned": "vendor/new-pkg"
        }"#;

        let pkg: OutdatedPackage = serde_json::from_str(input).unwrap();
        assert_eq!(pkg.name, "vendor/pkg");
        assert!(pkg.direct_dep);
        assert_eq!(pkg.version, "1.0.0");
        assert_eq!(pkg.latest, "2.0.0");
        assert_eq!(pkg.latest_status, "update-possible");
        assert!(pkg.abandoned.set);
        assert_eq!(pkg.abandoned.value, "vendor/new-pkg");
    }

    #[test]
    fn outdated_package_abandoned_false() {
        let input = r#"{
            "name": "vendor/pkg",
            "version": "1.0.0",
            "latest": "1.1.0",
            "latest-status": "semver-safe-update",
            "abandoned": false
        }"#;

        let pkg: OutdatedPackage = serde_json::from_str(input).unwrap();
        assert!(!pkg.abandoned.set);
    }

    #[test]
    fn advisory_nullable_fields() {
        let input = r#"{
            "advisoryId": "ADV-001",
            "packageName": "vendor/pkg",
            "title": "Critical bug",
            "link": "https://example.com",
            "cve": null,
            "affectedVersions": ">=1.0 <2.0",
            "reportedAt": "2024-01-01",
            "severity": null
        }"#;

        let adv: Advisory = serde_json::from_str(input).unwrap();
        assert!(adv.cve.is_none());
        assert!(adv.severity.is_none());
        assert_eq!(adv.advisory_id, "ADV-001");
    }

    #[test]
    fn advisory_with_cve() {
        let input = r#"{
            "advisoryId": "ADV-002",
            "packageName": "vendor/pkg",
            "title": "XSS vulnerability",
            "link": "https://example.com",
            "cve": "CVE-2024-1234",
            "affectedVersions": ">=1.0 <1.5",
            "reportedAt": "2024-06-01",
            "severity": "high"
        }"#;

        let adv: Advisory = serde_json::from_str(input).unwrap();
        assert_eq!(adv.cve.as_deref(), Some("CVE-2024-1234"));
        assert_eq!(adv.severity.as_deref(), Some("high"));
    }

    #[test]
    fn audit_result_unmarshal() {
        let input = r#"{
            "advisories": {
                "vendor/pkg": [{
                    "advisoryId": "ADV-001",
                    "packageName": "vendor/pkg",
                    "title": "Bug",
                    "link": "https://example.com",
                    "cve": "CVE-2024-0001",
                    "affectedVersions": ">=1.0",
                    "reportedAt": "2024-01-01",
                    "severity": "critical"
                }]
            },
            "abandoned": {
                "old/pkg": "new/pkg",
                "dead/pkg": null
            }
        }"#;

        let result: AuditResult = serde_json::from_str(input).unwrap();
        assert_eq!(result.advisories.len(), 1);
        assert_eq!(result.advisories["vendor/pkg"].len(), 1);
        assert_eq!(result.abandoned.len(), 2);
        assert_eq!(result.abandoned["old/pkg"].as_deref(), Some("new/pkg"));
        assert!(result.abandoned["dead/pkg"].is_none());
    }

    #[test]
    fn audit_result_advisories_as_object() {
        // Composer quirk: when advisories are keyed by numeric strings instead of an array
        let input = r#"{
            "advisories": {
                "twig/twig": {
                    "1": {
                        "advisoryId": "ADV-100",
                        "packageName": "twig/twig",
                        "title": "Sandbox bypass",
                        "link": "https://example.com",
                        "cve": "CVE-2024-0100",
                        "affectedVersions": ">=1.0",
                        "reportedAt": "2024-09-01",
                        "severity": "medium"
                    },
                    "2": {
                        "advisoryId": "ADV-101",
                        "packageName": "twig/twig",
                        "title": "Another issue",
                        "link": "https://example.com",
                        "cve": null,
                        "affectedVersions": ">=2.0",
                        "reportedAt": "2024-11-01",
                        "severity": "low"
                    }
                }
            },
            "abandoned": []
        }"#;

        let result: AuditResult = serde_json::from_str(input).unwrap();
        assert_eq!(result.advisories.len(), 1);
        assert_eq!(result.advisories["twig/twig"].len(), 2);
        assert!(result.abandoned.is_empty());
    }

    #[test]
    fn show_result_unmarshal() {
        let input = r#"{
            "name": "symfony/framework-bundle",
            "description": "Provides a tight integration",
            "keywords": [],
            "type": "symfony-bundle",
            "homepage": "https://symfony.com",
            "versions": ["v7.4.7"],
            "licenses": [{"name": "MIT License", "osi": "MIT"}],
            "source": {"type": "git", "url": "https://github.com/symfony/framework-bundle.git", "reference": "abc123"},
            "path": "/vendor/symfony/framework-bundle",
            "released": "2026-03-06T15:39:55+00:00",
            "requires": {"php": ">=8.2", "symfony/cache": "^6.4.12|^7.0|^8.0"},
            "devRequires": {"phpunit/phpunit": "^11.0"},
            "conflicts": {"doctrine/persistence": "<1.3"}
        }"#;

        let show: ShowResult = serde_json::from_str(input).unwrap();
        assert_eq!(show.name, "symfony/framework-bundle");
        assert_eq!(show.versions, vec!["v7.4.7"]);
        assert_eq!(show.licenses.len(), 1);
        assert_eq!(show.licenses[0].osi, "MIT");
        assert_eq!(show.requires.len(), 2);
        assert_eq!(show.dev_requires.len(), 1);
        assert_eq!(show.conflicts.len(), 1);
        assert!(!show.released.is_empty());
        assert!(!show.path.is_empty());
    }

    #[test]
    fn show_result_null_fields() {
        let input = r#"{
            "name": "vendor/pkg",
            "description": null,
            "keywords": [],
            "type": null,
            "homepage": null,
            "versions": [],
            "licenses": [],
            "source": {"type": "git", "url": "", "reference": ""},
            "path": null,
            "released": null,
            "requires": {},
            "devRequires": {},
            "conflicts": {}
        }"#;

        let show: ShowResult = serde_json::from_str(input).unwrap();
        assert_eq!(show.name, "vendor/pkg");
        assert_eq!(show.description, "");
        assert_eq!(show.pkg_type, "");
        assert_eq!(show.homepage, "");
        assert_eq!(show.path, "");
        assert_eq!(show.released, "");
        assert!(show.versions.is_empty());
        assert!(show.licenses.is_empty());
    }

    #[test]
    fn show_result_minimal() {
        let input = r#"{"name": "a/b"}"#;
        let show: ShowResult = serde_json::from_str(input).unwrap();
        assert_eq!(show.name, "a/b");
        assert!(show.requires.is_empty());
        assert!(show.dev_requires.is_empty());
        assert!(show.conflicts.is_empty());
        assert!(show.keywords.is_empty());
    }

    #[test]
    fn show_result_with_all_deps() {
        let input = r#"{
            "name": "symfony/framework-bundle",
            "requires": {"php": ">=8.2", "symfony/cache": "^7.0", "symfony/config": "^7.0"},
            "devRequires": {"phpunit/phpunit": "^11.0", "phpstan/phpstan": "^1.0"},
            "conflicts": {"doctrine/persistence": "<1.3", "symfony/console": "<6.4"}
        }"#;

        let show: ShowResult = serde_json::from_str(input).unwrap();
        assert_eq!(show.requires.len(), 3);
        assert_eq!(show.dev_requires.len(), 2);
        assert_eq!(show.conflicts.len(), 2);
        assert_eq!(show.requires["php"], ">=8.2");
        assert_eq!(show.dev_requires["phpunit/phpunit"], "^11.0");
        assert_eq!(show.conflicts["doctrine/persistence"], "<1.3");
    }

    #[test]
    fn show_result_multiple_licenses() {
        let input = r#"{
            "name": "a/b",
            "licenses": [
                {"name": "MIT License", "osi": "MIT"},
                {"name": "Apache License 2.0", "osi": "Apache-2.0"}
            ]
        }"#;

        let show: ShowResult = serde_json::from_str(input).unwrap();
        assert_eq!(show.licenses.len(), 2);
        assert_eq!(show.licenses[0].name, "MIT License");
        assert_eq!(show.licenses[1].osi, "Apache-2.0");
    }

    #[test]
    fn show_result_multiple_versions() {
        let input = r#"{
            "name": "a/b",
            "versions": ["v7.4.7", "v7.4.6", "v7.4.5"]
        }"#;

        let show: ShowResult = serde_json::from_str(input).unwrap();
        assert_eq!(show.versions.len(), 3);
        assert_eq!(show.versions[0], "v7.4.7");
    }

    #[test]
    fn show_license_default() {
        let lic: ShowLicense = serde_json::from_str(r#"{}"#).unwrap();
        assert_eq!(lic.name, "");
        assert_eq!(lic.osi, "");
    }

    #[test]
    fn package_status_constants() {
        assert_eq!(PackageStatus::OK as u8, 0);
        assert_eq!(PackageStatus::Outdated as u8, 1);
        assert_eq!(PackageStatus::Abandoned as u8, 2);
        assert_eq!(PackageStatus::Vulnerable as u8, 3);
        assert_eq!(PackageStatus::Restricted as u8, 4);
    }

    // --- string_or_vec tests ---

    #[test]
    fn string_or_vec_from_string() {
        #[derive(Deserialize)]
        struct W {
            #[serde(deserialize_with = "string_or_vec", default)]
            license: Vec<String>,
        }
        let w: W = serde_json::from_str(r#"{"license": "MIT"}"#).unwrap();
        assert_eq!(w.license, vec!["MIT"]);
    }

    #[test]
    fn string_or_vec_from_array() {
        #[derive(Deserialize)]
        struct W {
            #[serde(deserialize_with = "string_or_vec", default)]
            license: Vec<String>,
        }
        let w: W = serde_json::from_str(r#"{"license": ["MIT", "Apache-2.0"]}"#).unwrap();
        assert_eq!(w.license, vec!["MIT", "Apache-2.0"]);
    }

    #[test]
    fn string_or_vec_missing_field() {
        #[derive(Deserialize)]
        struct W {
            #[serde(deserialize_with = "string_or_vec", default)]
            license: Vec<String>,
        }
        let w: W = serde_json::from_str(r#"{}"#).unwrap();
        assert!(w.license.is_empty());
    }

    // --- Author tests ---

    #[test]
    fn author_full() {
        let input = r#"{
            "name": "John Doe",
            "email": "john@example.com",
            "homepage": "https://johndoe.dev",
            "role": "Developer"
        }"#;
        let author: Author = serde_json::from_str(input).unwrap();
        assert_eq!(author.name, "John Doe");
        assert_eq!(author.email, "john@example.com");
        assert_eq!(author.homepage, "https://johndoe.dev");
        assert_eq!(author.role, "Developer");
    }

    #[test]
    fn author_partial() {
        let input = r#"{"name": "Jane"}"#;
        let author: Author = serde_json::from_str(input).unwrap();
        assert_eq!(author.name, "Jane");
        assert_eq!(author.email, "");
    }

    // --- Support tests ---

    #[test]
    fn support_full() {
        let input = r#"{
            "issues": "https://github.com/test/issues",
            "source": "https://github.com/test",
            "docs": "https://docs.test.com",
            "email": "support@test.com"
        }"#;
        let support: Support = serde_json::from_str(input).unwrap();
        assert_eq!(support.issues, "https://github.com/test/issues");
        assert_eq!(support.source, "https://github.com/test");
        assert_eq!(support.docs, "https://docs.test.com");
        assert_eq!(support.email, "support@test.com");
        assert_eq!(support.forum, "");
    }

    // --- ComposerJSON new fields tests ---

    #[test]
    fn composer_json_all_new_fields() {
        let input = r#"{
            "name": "test/pkg",
            "license": ["MIT", "Apache-2.0"],
            "homepage": "https://example.com",
            "minimum-stability": "dev",
            "prefer-stable": true,
            "authors": [{"name": "Alice"}],
            "support": {"issues": "https://issues.example.com"},
            "require": {"php": ">=8.2"},
            "require-dev": {},
            "replace": {"old/pkg": "self.version"},
            "conflict": {"bad/pkg": "<1.0"},
            "provide": {"psr/log-implementation": "1.0"},
            "suggest": {"ext-intl": "For i18n"},
            "repositories": [{"type": "vcs", "url": "https://github.com/x/y"}],
            "scripts": {"test": "phpunit"},
            "config": {"sort-packages": true}
        }"#;

        let cj: ComposerJSON = serde_json::from_str(input).unwrap();
        assert_eq!(cj.name, "test/pkg");
        assert_eq!(cj.license, vec!["MIT", "Apache-2.0"]);
        assert_eq!(cj.homepage, "https://example.com");
        assert_eq!(cj.minimum_stability, "dev");
        assert!(cj.prefer_stable);
        assert_eq!(cj.authors.len(), 1);
        assert_eq!(cj.authors[0].name, "Alice");
        assert_eq!(
            cj.support.as_ref().unwrap().issues,
            "https://issues.example.com"
        );
        assert_eq!(cj.replace["old/pkg"], "self.version");
        assert_eq!(cj.conflict["bad/pkg"], "<1.0");
        assert_eq!(cj.provide["psr/log-implementation"], "1.0");
        assert_eq!(cj.suggest["ext-intl"], "For i18n");
        assert!(cj.repositories.is_some());
        assert!(cj.scripts.contains_key("test"));
        assert!(cj.config.is_some());
    }

    #[test]
    fn composer_json_minimal() {
        let input = r#"{"name": "a/b"}"#;
        let cj: ComposerJSON = serde_json::from_str(input).unwrap();
        assert_eq!(cj.name, "a/b");
        assert!(cj.license.is_empty());
        assert!(cj.homepage.is_empty());
        assert!(cj.minimum_stability.is_empty());
        assert!(!cj.prefer_stable);
        assert!(cj.authors.is_empty());
        assert!(cj.support.is_none());
        assert!(cj.replace.is_empty());
        assert!(cj.conflict.is_empty());
        assert!(cj.provide.is_empty());
        assert!(cj.suggest.is_empty());
        assert!(cj.repositories.is_none());
        assert!(cj.scripts.is_empty());
        assert!(cj.config.is_none());
    }

    #[test]
    fn composer_json_license_as_string() {
        let input = r#"{"license": "MIT"}"#;
        let cj: ComposerJSON = serde_json::from_str(input).unwrap();
        assert_eq!(cj.license, vec!["MIT"]);
    }

    #[test]
    fn composer_json_license_as_array() {
        let input = r#"{"license": ["MIT", "GPL-3.0"]}"#;
        let cj: ComposerJSON = serde_json::from_str(input).unwrap();
        assert_eq!(cj.license, vec!["MIT", "GPL-3.0"]);
    }
}
