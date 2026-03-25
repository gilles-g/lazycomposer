use ratatui::style::{Modifier, Style};

use crate::composer::PackageStatus;

use super::theme::*;

pub fn title_style() -> Style {
    Style::default()
        .fg(COLOR_BRIGHT)
        .add_modifier(Modifier::BOLD)
}

pub fn description_style() -> Style {
    Style::default().fg(COLOR_TEXT)
}

pub fn version_style() -> Style {
    Style::default().fg(COLOR_INFO)
}

pub fn dev_style() -> Style {
    Style::default()
        .fg(COLOR_WARNING)
        .add_modifier(Modifier::ITALIC)
}

pub fn key_style() -> Style {
    Style::default()
        .fg(COLOR_PRIMARY)
        .add_modifier(Modifier::BOLD)
}

pub fn muted_style() -> Style {
    Style::default().fg(COLOR_MUTED)
}

pub fn success_style() -> Style {
    Style::default().fg(COLOR_SUCCESS)
}

pub fn warning_style() -> Style {
    Style::default().fg(COLOR_WARNING)
}

pub fn error_style() -> Style {
    Style::default()
        .fg(COLOR_DANGER)
        .add_modifier(Modifier::BOLD)
}

pub fn output_style() -> Style {
    Style::default().fg(COLOR_BRIGHT)
}

pub fn section_header_style() -> Style {
    Style::default()
        .fg(COLOR_MUTED)
        .add_modifier(Modifier::BOLD)
}

pub fn package_ok_style() -> Style {
    Style::default().fg(COLOR_STATUS_OK)
}

pub fn package_outdated_style() -> Style {
    Style::default().fg(COLOR_STATUS_OUTDATED)
}

pub fn package_abandoned_style() -> Style {
    Style::default()
        .fg(COLOR_STATUS_ABANDONED)
        .add_modifier(Modifier::CROSSED_OUT)
}

pub fn package_vulnerable_style() -> Style {
    Style::default()
        .fg(COLOR_STATUS_VULNERABLE)
        .add_modifier(Modifier::BOLD)
}

pub fn package_restricted_style() -> Style {
    Style::default().fg(COLOR_STATUS_RESTRICTED)
}

/// Returns the appropriate style for a package status.
pub fn package_status_style(status: PackageStatus) -> Style {
    match status {
        PackageStatus::Outdated => package_outdated_style(),
        PackageStatus::Abandoned => package_abandoned_style(),
        PackageStatus::Vulnerable => package_vulnerable_style(),
        PackageStatus::Restricted => package_restricted_style(),
        _ => package_ok_style(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_status_style_all_statuses() {
        let tests = vec![
            (PackageStatus::OK, package_ok_style()),
            (PackageStatus::Outdated, package_outdated_style()),
            (PackageStatus::Abandoned, package_abandoned_style()),
            (PackageStatus::Vulnerable, package_vulnerable_style()),
            (PackageStatus::Restricted, package_restricted_style()),
        ];

        for (status, expected) in tests {
            assert_eq!(package_status_style(status), expected, "status={status:?}");
        }
    }

    #[test]
    fn package_status_style_defaults_to_ok() {
        // Constructing an unknown PackageStatus isn't possible with enum,
        // so just verify default returns OK style
        assert_eq!(package_status_style(PackageStatus::OK), package_ok_style());
    }
}
