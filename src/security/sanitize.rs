use regex::Regex;
use std::sync::LazyLock;

const MAX_LOG_OUTPUT_LEN: usize = 500;

static SENSITIVE_PATTERNS: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)((password|passwd|token|secret|api[_-]?key|auth|bearer|credential)["']?\s*[:=]\s*["']?\S+|https?://[^@\s]+:[^@\s]+@)"#,
    )
    .unwrap()
});

/// Truncates output to a safe length and redacts sensitive patterns.
pub fn sanitize_log_output(output: &str) -> String {
    let truncated = if output.len() > MAX_LOG_OUTPUT_LEN {
        format!("{}... (truncated)", &output[..MAX_LOG_OUTPUT_LEN])
    } else {
        output.to_string()
    };

    SENSITIVE_PATTERNS
        .replace_all(&truncated, "[REDACTED]")
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_long_output() {
        let long: String = "x".repeat(1000);
        let result = sanitize_log_output(&long);
        assert!(result.len() <= MAX_LOG_OUTPUT_LEN + 20);
        assert!(result.contains("... (truncated)"));
    }

    #[test]
    fn short_output_unchanged() {
        let input = "some normal output";
        let result = sanitize_log_output(input);
        assert_eq!(result, input);
    }

    #[test]
    fn redacts_sensitive_patterns() {
        let tests = vec![
            (r#"{"password": "s3cret123"}"#, "password"),
            ("token=abc123def456", "token"),
            ("api_key: my-secret-key", "api key"),
            ("bearer: eyJhbGciOiJIUzI1NiJ9", "bearer"),
            (
                "https://user:pass@registry.example.com/packages",
                "url with creds",
            ),
            ("auth=mysecretvalue", "auth header"),
        ];

        for (input, name) in tests {
            let result = sanitize_log_output(input);
            assert!(
                result.contains("[REDACTED]"),
                "expected redaction for {name}: {input:?} → {result:?}"
            );
        }
    }

    #[test]
    fn preserves_normal_output() {
        let input = "Loading composer repositories with package information\nUpdating dependencies\nNothing to modify in lock file";
        let result = sanitize_log_output(input);
        assert_eq!(result, input);
    }
}
