use ratatui::style::Color;

// Color palette — matches Composer CLI output colors.
pub const COLOR_PRIMARY: Color = Color::Rgb(0x00, 0xCC, 0x00);
pub const COLOR_SECONDARY: Color = Color::Rgb(0x00, 0xCC, 0xCC);
pub const COLOR_SUCCESS: Color = Color::Rgb(0x00, 0xCC, 0x00);
pub const COLOR_WARNING: Color = Color::Rgb(0xCC, 0xCC, 0x00);
pub const COLOR_DANGER: Color = Color::Rgb(0xCC, 0x00, 0x00);
pub const COLOR_INFO: Color = Color::Rgb(0x00, 0xCC, 0xCC);
pub const COLOR_MUTED: Color = Color::Rgb(0x80, 0x80, 0x80);
pub const COLOR_TEXT: Color = Color::Rgb(0xCC, 0xCC, 0xCC);
pub const COLOR_BRIGHT: Color = Color::Rgb(0xFF, 0xFF, 0xFF);
pub const COLOR_BACKGROUND: Color = Color::Rgb(0x00, 0x00, 0x00);
pub const COLOR_BORDER: Color = Color::Rgb(0x80, 0x80, 0x80);
pub const COLOR_BORDER_FOCUS: Color = Color::Rgb(0x00, 0xCC, 0x00);
pub const COLOR_TAB_ACTIVE: Color = Color::Rgb(0x00, 0xCC, 0x00);
pub const COLOR_TAB_INACTIVE: Color = Color::Rgb(0x80, 0x80, 0x80);

// Outdated status colors
pub const COLOR_PATCH: Color = COLOR_SUCCESS;
pub const COLOR_MINOR: Color = COLOR_WARNING;
pub const COLOR_MAJOR: Color = COLOR_DANGER;

// Package health status colors
pub const COLOR_STATUS_OK: Color = Color::Rgb(0x00, 0xCC, 0x00);
pub const COLOR_STATUS_OUTDATED: Color = Color::Rgb(0xCC, 0xCC, 0x00);
pub const COLOR_STATUS_ABANDONED: Color = Color::Rgb(0xCC, 0x00, 0x00);
pub const COLOR_STATUS_VULNERABLE: Color = Color::Rgb(0xCC, 0x00, 0x00);

/// Returns the appropriate color for an outdated status.
pub fn status_color(status: &str) -> Color {
    match status {
        "semver-safe-update" => COLOR_PATCH,
        "update-possible" => COLOR_MINOR,
        _ => COLOR_MAJOR,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_color_semver_safe() {
        assert_eq!(status_color("semver-safe-update"), COLOR_PATCH);
    }

    #[test]
    fn status_color_update_possible() {
        assert_eq!(status_color("update-possible"), COLOR_MINOR);
    }

    #[test]
    fn status_color_unknown() {
        assert_eq!(status_color("something-else"), COLOR_MAJOR);
    }

    #[test]
    fn status_color_empty() {
        assert_eq!(status_color(""), COLOR_MAJOR);
    }
}
