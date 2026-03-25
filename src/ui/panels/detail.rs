use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::composer::{OutdatedPackage, Package, PackageStatus};
use crate::ui::style::{styles, theme};
use crate::ui::text::wrap_field;

/// Render the detail panel for a package, optionally enriched with outdated info.
pub fn render_detail(
    pkg: Option<&Package>,
    outdated: Option<&OutdatedPackage>,
    area: Rect,
    buf: &mut Buffer,
    focused: bool,
) {
    let border_style = if focused {
        Style::default().fg(theme::COLOR_BORDER_FOCUS)
    } else {
        Style::default().fg(theme::COLOR_BORDER)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style);
    let inner = block.inner(area);
    block.render(area, buf);

    let pkg = match pkg {
        Some(p) => p,
        None => {
            let text = Paragraph::new(Span::styled("No package selected", styles::muted_style()));
            text.render(inner, buf);
            return;
        }
    };

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(Span::styled(&pkg.name, styles::title_style())));
    lines.push(Line::default());

    if !pkg.version.is_empty() {
        lines.push(styled_field(
            "Version:",
            &pkg.version,
            styles::version_style(),
        ));
    }

    if !pkg.constraint.is_empty() {
        let bounds = crate::composer::explain_constraint(&pkg.constraint);
        lines.push(Line::from(vec![
            Span::styled("Constraint:", styles::key_style()),
            Span::raw("  "),
            Span::styled(&pkg.constraint, styles::version_style()),
            Span::raw("  "),
            Span::styled(format!("({bounds})"), styles::muted_style()),
        ]));
    }

    if let Some(o) = outdated {
        let outdated_lines = if pkg.status == PackageStatus::Restricted {
            outdated_restricted_lines(pkg, o)
        } else if let Some(rv) = &pkg.restricted_latest {
            if pkg.status != PackageStatus::OK {
                outdated_within_framework_lines(rv)
            } else {
                vec![]
            }
        } else {
            outdated_standard_lines(o)
        };
        lines.extend(outdated_lines);

        if !o.warning.is_empty() {
            lines.push(Line::from(vec![
                Span::styled("⚠ ", styles::error_style()),
                Span::styled(&o.warning, styles::error_style()),
            ]));
        }

        if o.abandoned.set {
            let replacement = if o.abandoned.value.is_empty() {
                "no replacement"
            } else {
                &o.abandoned.value
            };
            lines.push(Line::from(vec![
                Span::styled("⚑ Abandoned", styles::warning_style()),
                Span::raw("  "),
                Span::styled(format!("→ {replacement}"), styles::muted_style()),
            ]));
        }
    }

    if !pkg.description.is_empty() {
        lines.extend(wrap_field(
            "Description:",
            &pkg.description,
            Style::default().fg(theme::COLOR_TEXT),
            inner.width,
        ));
    }
    if !pkg.pkg_type.is_empty() {
        lines.push(styled_field(
            "Type:",
            &pkg.pkg_type,
            Style::default().fg(theme::COLOR_TEXT),
        ));
    }
    if !pkg.license.is_empty() {
        lines.push(styled_field(
            "License:",
            &pkg.license,
            Style::default().fg(theme::COLOR_TEXT),
        ));
    }
    if !pkg.homepage.is_empty() {
        lines.push(styled_field(
            "Homepage:",
            &pkg.homepage,
            Style::default().fg(theme::COLOR_INFO),
        ));
    }
    if !pkg.source.url.is_empty() {
        lines.push(styled_field(
            "Source:",
            &pkg.source.url,
            Style::default().fg(theme::COLOR_INFO),
        ));
    }

    let dev_span = if pkg.is_dev {
        Span::styled("Yes", styles::dev_style())
    } else {
        Span::raw("No")
    };
    lines.push(Line::from(vec![
        Span::styled("Dev:  ", styles::key_style()),
        dev_span,
    ]));

    let status_span = match pkg.status {
        PackageStatus::Vulnerable => Span::styled("Vulnerable", styles::package_vulnerable_style()),
        PackageStatus::Abandoned => Span::styled("Abandoned", styles::package_abandoned_style()),
        PackageStatus::Outdated => Span::styled("Outdated", styles::package_outdated_style()),
        PackageStatus::Restricted => Span::styled("Restricted", styles::package_restricted_style()),
        _ => Span::styled("OK", styles::package_ok_style()),
    };
    lines.push(Line::from(vec![
        Span::styled("Status:  ", styles::key_style()),
        status_span,
    ]));

    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        "u: update  d: remove",
        styles::muted_style(),
    )));

    let paragraph = Paragraph::new(lines);
    paragraph.render(inner, buf);
}

/// Package Restricted by the framework (latest blocked, version within constraint being resolved).
fn outdated_restricted_lines<'a>(pkg: &'a Package, o: &'a OutdatedPackage) -> Vec<Line<'a>> {
    let mut lines = vec![Line::from(vec![
        Span::styled("Latest:", styles::key_style()),
        Span::raw("  "),
        Span::styled(&o.latest, styles::muted_style()),
        Span::styled(" (blocked by framework)", styles::muted_style()),
    ])];
    match &pkg.restricted_latest {
        Some(v) if v != &pkg.version => {
            lines.push(Line::from(vec![
                Span::styled("Available:", styles::key_style()),
                Span::raw(" "),
                Span::styled(v, styles::package_restricted_style()),
                Span::styled(" (within framework)", styles::muted_style()),
            ]));
        }
        Some(_) => {
            lines.push(Line::from(Span::styled(
                "◆ Up to date within framework constraint",
                styles::package_restricted_style(),
            )));
        }
        None => {
            lines.push(Line::from(Span::styled(
                "◆ Resolving version within framework…",
                styles::muted_style(),
            )));
        }
    }
    lines
}

/// Package promoted from Restricted to Outdated (update available within framework constraint).
fn outdated_within_framework_lines(restricted_latest: &str) -> Vec<Line<'_>> {
    vec![
        Line::from(vec![
            Span::styled("Latest:", styles::key_style()),
            Span::raw("  "),
            Span::styled(restricted_latest, styles::package_outdated_style()),
            Span::styled(" (within framework)", styles::muted_style()),
        ]),
        Line::from(vec![
            Span::styled("Update:", styles::key_style()),
            Span::raw("  "),
            Span::styled(
                "● Safe update (within framework)",
                styles::package_outdated_style(),
            ),
        ]),
    ]
}

/// Standard outdated package (no framework restriction).
fn outdated_standard_lines(o: &OutdatedPackage) -> Vec<Line<'_>> {
    let status_color = theme::status_color(&o.latest_status);
    let version_style = Style::default().fg(status_color);
    let status_label = match o.latest_status.as_str() {
        "semver-safe-update" => "● Safe update (minor/patch)",
        "update-possible" => "▲ Update possible (major)",
        _ => &o.latest_status,
    };
    vec![
        Line::from(vec![
            Span::styled("Latest:", styles::key_style()),
            Span::raw("  "),
            Span::styled(&o.latest, version_style),
        ]),
        Line::from(vec![
            Span::styled("Update:", styles::key_style()),
            Span::raw("  "),
            Span::styled(status_label.to_string(), version_style),
        ]),
    ]
}

fn styled_field<'a>(label: &'a str, value: &'a str, value_style: Style) -> Line<'a> {
    Line::from(vec![
        Span::styled(label, styles::key_style()),
        Span::raw("  "),
        Span::styled(value, value_style),
    ])
}

/// String-based detail view (for tests).
pub fn detail_view(pkg: Option<&Package>, _width: u16, _height: u16, _focused: bool) -> String {
    match pkg {
        None => "No package selected".to_string(),
        Some(pkg) => {
            let mut b = String::new();
            b.push_str(&pkg.name);
            b.push_str("\n\n");
            if !pkg.version.is_empty() {
                b.push_str(&format!("Version:  {}\n", pkg.version));
            }
            if !pkg.constraint.is_empty() {
                let bounds = crate::composer::explain_constraint(&pkg.constraint);
                b.push_str(&format!("Constraint:  {}  ({bounds})\n", pkg.constraint));
            }
            if !pkg.description.is_empty() {
                b.push_str(&format!("Description:  {}\n", pkg.description));
            }
            if !pkg.pkg_type.is_empty() {
                b.push_str(&format!("Type:  {}\n", pkg.pkg_type));
            }
            if !pkg.license.is_empty() {
                b.push_str(&format!("License:  {}\n", pkg.license));
            }
            if !pkg.homepage.is_empty() {
                b.push_str(&format!("Homepage:  {}\n", pkg.homepage));
            }
            if !pkg.source.url.is_empty() {
                b.push_str(&format!("Source:  {}\n", pkg.source.url));
            }
            let dev_label = if pkg.is_dev { "Yes" } else { "No" };
            b.push_str(&format!("Dev:  {dev_label}\n"));
            let status_label = status_label(pkg.status);
            b.push_str(&format!("Status:  {status_label}\n"));
            b.push_str("\nu: update  d: remove");
            b
        }
    }
}

#[allow(dead_code)]
fn status_label(status: PackageStatus) -> &'static str {
    match status {
        PackageStatus::Vulnerable => "Vulnerable",
        PackageStatus::Abandoned => "Abandoned",
        PackageStatus::Outdated => "Outdated",
        PackageStatus::Restricted => "Restricted",
        _ => "OK",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composer::Source;

    #[test]
    fn detail_view_nil_package() {
        assert!(!detail_view(None, 60, 30, false).is_empty());
    }
    #[test]
    fn detail_view_nil_package_focused() {
        assert!(!detail_view(None, 60, 30, true).is_empty());
    }
    #[test]
    fn detail_view_with_package() {
        let pkg = Package {
            name: "vendor/pkg".to_string(),
            version: "1.0.0".to_string(),
            constraint: "^1.0".to_string(),
            description: "A package".to_string(),
            pkg_type: "library".to_string(),
            license: "MIT".to_string(),
            homepage: "https://example.com".to_string(),
            source: Source {
                url: "https://github.com/vendor/pkg".to_string(),
                ..Default::default()
            },
            is_dev: false,
            status: PackageStatus::OK,
            restricted_latest: None,
        };
        let view = detail_view(Some(&pkg), 60, 30, true);
        assert!(view.contains("Constraint:  ^1.0  (>=1.0.0, <2.0.0)"));
    }
    #[test]
    fn detail_view_dev_package() {
        let pkg = Package {
            name: "phpunit/phpunit".to_string(),
            version: "11.0.0".to_string(),
            is_dev: true,
            status: PackageStatus::Outdated,
            ..Default::default()
        };
        assert!(!detail_view(Some(&pkg), 60, 30, false).is_empty());
    }
    #[test]
    fn detail_view_no_framework_info() {
        let pkg = Package {
            name: "vendor/pkg".to_string(),
            version: "1.0.0".to_string(),
            ..Default::default()
        };
        let view = detail_view(Some(&pkg), 60, 30, true);
        assert!(!view.contains("Symfony"));
    }
    #[test]
    fn test_status_label() {
        for s in [
            PackageStatus::OK,
            PackageStatus::Outdated,
            PackageStatus::Abandoned,
            PackageStatus::Vulnerable,
            PackageStatus::Restricted,
        ] {
            assert!(!status_label(s).is_empty());
        }
    }
}
