use regex::Regex;
use std::sync::LazyLock;

static PACKAGE_NAME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-z0-9]([a-z0-9._-]*[a-z0-9])?/[a-z0-9]([a-z0-9._-]*[a-z0-9])?$").unwrap()
});

static CONSTRAINT_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9\^~>=<!|&,.*@_ -]+$").unwrap());

/// Validates and sanitizes a user-provided package name.
/// Returns the trimmed package name or an error if invalid.
/// Accepts format: vendor/package or vendor/package:constraint
pub fn validate_package_name(input: &str) -> Result<String, String> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Err("package name cannot be empty".to_string());
    }

    if trimmed.starts_with('-') {
        return Err("invalid package name: must not start with '-'".to_string());
    }

    if trimmed.contains(' ') {
        return Err("invalid package name: must not contain spaces".to_string());
    }

    let (name, constraint) = match trimmed.find(':') {
        Some(idx) => (&trimmed[..idx], Some(&trimmed[idx + 1..])),
        None => (trimmed, None),
    };

    if !PACKAGE_NAME_REGEX.is_match(name) {
        return Err(format!(
            "invalid package name {:?}: must match vendor/package format (lowercase alphanumeric, hyphens, dots, underscores)",
            name
        ));
    }

    if let Some(constraint) = constraint {
        if constraint.starts_with('-') {
            return Err("invalid version constraint: must not start with '-'".to_string());
        }
        if !CONSTRAINT_REGEX.is_match(constraint) {
            return Err(format!("invalid version constraint {:?}", constraint));
        }
    }

    Ok(trimmed.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_names() {
        let tests = vec![
            ("vendor/package", "vendor/package"),
            ("  vendor/package  ", "vendor/package"),
            ("symfony/framework-bundle", "symfony/framework-bundle"),
            ("my_vendor/my.package", "my_vendor/my.package"),
            ("vendor/package:^2.0", "vendor/package:^2.0"),
            ("vendor/package:>=1.0,<3.0", "vendor/package:>=1.0,<3.0"),
            ("vendor/package:~1.2.3", "vendor/package:~1.2.3"),
            ("vendor/package:dev-main", "vendor/package:dev-main"),
            ("vendor/package:1.0.0@beta", "vendor/package:1.0.0@beta"),
            ("vendor/package:v2.*", "vendor/package:v2.*"),
        ];

        for (input, expected) in tests {
            let result = validate_package_name(input);
            assert_eq!(result, Ok(expected.to_string()), "input={input:?}");
        }
    }

    #[test]
    fn invalid_names() {
        let tests = vec![
            ("", "empty string"),
            ("   ", "whitespace only"),
            ("-vendor/package", "starts with dash"),
            (
                "--repository=https://evil.com",
                "flag injection double dash",
            ),
            ("-no-scripts", "flag injection single dash"),
            ("vendor", "missing slash"),
            ("/package", "missing vendor"),
            ("vendor/", "missing package"),
            ("VENDOR/Package", "uppercase vendor"),
            ("vendor/Pack age", "space in package name"),
            ("../../etc/passwd", "path traversal"),
            ("vendor/package --no-scripts", "flag after package name"),
        ];

        for (input, name) in tests {
            let result = validate_package_name(input);
            assert!(result.is_err(), "expected error for {name}: {input:?}");
        }
    }
}
