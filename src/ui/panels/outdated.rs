use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::composer::OutdatedPackage;
use crate::ui::style::{styles, theme};

/// OutdatedPanel shows outdated packages in a color-coded list.
pub struct OutdatedPanel {
    pub packages: Vec<OutdatedPackage>,
    cursor: usize,
    pub width: u16,
    pub height: u16,
}

impl Default for OutdatedPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl OutdatedPanel {
    pub fn new() -> Self {
        OutdatedPanel {
            packages: vec![],
            cursor: 0,
            width: 0,
            height: 0,
        }
    }

    pub fn set_outdated(&mut self, packages: Vec<OutdatedPackage>) {
        self.packages = packages;
        self.cursor = 0;
    }

    pub fn selected_package(&self) -> Option<&str> {
        self.packages.get(self.cursor).map(|p| p.name.as_str())
    }

    pub fn selected_outdated_package(&self) -> Option<&OutdatedPackage> {
        self.packages.get(self.cursor)
    }

    pub fn set_size(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if !self.packages.is_empty() && self.cursor < self.packages.len() - 1 {
                    self.cursor += 1;
                }
            }
            _ => {}
        }
    }

    /// Render the outdated panel with colors.
    pub fn render(&self, area: Rect, buf: &mut Buffer, focused: bool) {
        let border_style = if focused {
            Style::default().fg(theme::COLOR_BORDER_FOCUS)
        } else {
            Style::default().fg(theme::COLOR_BORDER)
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(" Outdated ", styles::title_style()));
        let inner = block.inner(area);
        block.render(area, buf);

        if self.packages.is_empty() {
            let text = Paragraph::new(Span::styled(
                "No outdated packages found.",
                Style::default().fg(theme::COLOR_TEXT),
            ));
            text.render(inner, buf);
            return;
        }

        let visible_height = inner.height as usize;
        let scroll = if self.cursor >= visible_height {
            self.cursor - visible_height + 1
        } else {
            0
        };

        for (i, pkg) in self
            .packages
            .iter()
            .enumerate()
            .skip(scroll)
            .take(visible_height)
        {
            let y = inner.y + (i - scroll) as u16;
            if y >= inner.y + inner.height {
                break;
            }

            let selected = i == self.cursor;
            let color = theme::status_color(&pkg.latest_status);
            let color_style = Style::default().fg(color);
            let status = format_status(&pkg.latest_status);

            let prefix = if selected { "> " } else { "  " };
            let prefix_style = if selected {
                styles::key_style()
            } else {
                Style::default()
            };

            let line = Line::from(vec![
                Span::styled(prefix, prefix_style),
                Span::styled(&pkg.name, color_style),
                Span::raw("  "),
                Span::styled(status, color_style),
            ]);
            buf.set_line(inner.x, y, &line, inner.width);
        }

        // Count indicator
        if self.packages.len() > visible_height && visible_height > 0 {
            let info = format!(" {}/{} ", self.cursor + 1, self.packages.len());
            let x = inner.x + inner.width.saturating_sub(info.len() as u16);
            buf.set_string(
                x,
                inner.y + inner.height.saturating_sub(1),
                &info,
                styles::muted_style(),
            );
        }
    }

    pub fn view(&self, _focused: bool) -> String {
        if self.packages.is_empty() {
            return "No outdated packages found.".to_string();
        }
        let mut result = String::new();
        for (i, pkg) in self.packages.iter().enumerate() {
            let prefix = if i == self.cursor { "> " } else { "  " };
            let status = format_status(&pkg.latest_status);
            result.push_str(&format!("{prefix}{}  {status}\n", pkg.name));
        }
        result
    }
}

pub fn format_status(status: &str) -> String {
    match status {
        "semver-safe-update" => "● safe update".to_string(),
        "update-possible" => "▲ update possible".to_string(),
        _ => format!("✖ {status}"),
    }
}

/// Render the outdated detail panel with colors.
pub fn render_outdated_detail(
    pkg: Option<&OutdatedPackage>,
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
            Paragraph::new(Span::styled("No package selected", styles::muted_style()))
                .render(inner, buf);
            return;
        }
    };

    let status_color = theme::status_color(&pkg.latest_status);
    let version_style = Style::default().fg(status_color);

    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(&pkg.name, styles::title_style())),
        Line::default(),
        Line::from(vec![
            Span::styled("Current:  ", styles::key_style()),
            Span::styled(&pkg.version, styles::version_style()),
        ]),
        Line::from(vec![
            Span::styled("Latest:  ", styles::key_style()),
            Span::styled(&pkg.latest, version_style),
        ]),
    ];

    let status_label = match pkg.latest_status.as_str() {
        "semver-safe-update" => "● Safe update (minor/patch)",
        "update-possible" => "▲ Update possible (major)",
        _ => &pkg.latest_status,
    };
    lines.push(Line::from(vec![
        Span::styled("Status:  ", styles::key_style()),
        Span::styled(status_label.to_string(), version_style),
    ]));
    lines.push(Line::default());

    if !pkg.description.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Description:  ", styles::key_style()),
            Span::raw(&pkg.description),
        ]));
    }
    if !pkg.homepage.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Homepage:  ", styles::key_style()),
            Span::styled(&pkg.homepage, Style::default().fg(theme::COLOR_INFO)),
        ]));
    }

    if !pkg.warning.is_empty() {
        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled("⚠ ", styles::error_style()),
            Span::styled(&pkg.warning, styles::error_style()),
        ]));
    }

    if pkg.abandoned.set {
        let replacement = if pkg.abandoned.value.is_empty() {
            "no replacement"
        } else {
            &pkg.abandoned.value
        };
        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled("⚑ Abandoned", styles::warning_style()),
            Span::raw("  "),
            Span::styled(format!("→ {replacement}"), styles::muted_style()),
        ]));
    }

    if pkg.direct_dep {
        lines.push(Line::from(vec![
            Span::styled("Direct:  ", styles::key_style()),
            Span::raw("Yes"),
        ]));
    } else {
        lines.push(Line::from(vec![
            Span::styled("Direct:  ", styles::key_style()),
            Span::styled("No (transitive)", styles::muted_style()),
        ]));
    }

    lines.push(Line::default());
    lines.push(Line::from(Span::styled(
        "u: update this package",
        styles::muted_style(),
    )));

    Paragraph::new(lines).render(inner, buf);
}

pub fn outdated_detail_view(
    pkg: Option<&OutdatedPackage>,
    _width: u16,
    _height: u16,
    _focused: bool,
) -> String {
    let pkg = match pkg {
        None => return "No package selected".to_string(),
        Some(p) => p,
    };
    let mut b = String::new();
    b.push_str(&format!(
        "{}\n\nCurrent:  {}\nLatest:  {}\n",
        pkg.name, pkg.version, pkg.latest
    ));
    match pkg.latest_status.as_str() {
        "semver-safe-update" => b.push_str("Status:  ● Safe update (minor/patch)\n"),
        "update-possible" => b.push_str("Status:  ▲ Update possible (major)\n"),
        _ => b.push_str(&format!("Status:  {}\n", pkg.latest_status)),
    }
    b.push('\n');
    if !pkg.description.is_empty() {
        b.push_str(&format!("Description:  {}\n", pkg.description));
    }
    if !pkg.homepage.is_empty() {
        b.push_str(&format!("Homepage:  {}\n", pkg.homepage));
    }
    if !pkg.warning.is_empty() {
        b.push_str(&format!("\n⚠ {}\n", pkg.warning));
    }
    if pkg.abandoned.set {
        let r = if pkg.abandoned.value.is_empty() {
            "no replacement"
        } else {
            &pkg.abandoned.value
        };
        b.push_str(&format!("\n⚑ Abandoned → {r}\n"));
    }
    if pkg.direct_dep {
        b.push_str("Direct:  Yes\n");
    } else {
        b.push_str("Direct:  No (transitive)\n");
    }
    b.push_str("\nu: update this package");
    b
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composer::StringOrBool;

    fn sample_outdated() -> Vec<OutdatedPackage> {
        vec![
            OutdatedPackage {
                name: "symfony/framework-bundle".to_string(),
                version: "v7.0.4".to_string(),
                latest: "v7.1.0".to_string(),
                latest_status: "semver-safe-update".to_string(),
                description: "Framework bundle".to_string(),
                direct_dep: true,
                ..Default::default()
            },
            OutdatedPackage {
                name: "doctrine/orm".to_string(),
                version: "3.0.0".to_string(),
                latest: "4.0.0".to_string(),
                latest_status: "update-possible".to_string(),
                description: "ORM".to_string(),
                direct_dep: true,
                ..Default::default()
            },
        ]
    }

    #[test]
    fn set_outdated() {
        let mut p = OutdatedPanel::new();
        p.set_outdated(sample_outdated());
        assert_eq!(p.packages.len(), 2);
    }
    #[test]
    fn set_outdated_empty() {
        let mut p = OutdatedPanel::new();
        p.set_outdated(vec![]);
        assert_eq!(p.packages.len(), 0);
    }
    #[test]
    fn selected_package() {
        let mut p = OutdatedPanel::new();
        p.set_size(80, 40);
        p.set_outdated(sample_outdated());
        let _name = p.selected_package();
    }
    #[test]
    fn selected_outdated_package_empty() {
        let mut p = OutdatedPanel::new();
        p.set_size(80, 40);
        assert!(p.selected_outdated_package().is_none());
    }
    #[test]
    fn set_size() {
        let mut p = OutdatedPanel::new();
        p.set_size(100, 50);
        assert_eq!(p.width, 100);
        assert_eq!(p.height, 50);
    }
    #[test]
    fn view_empty() {
        let mut p = OutdatedPanel::new();
        p.set_size(80, 40);
        assert!(!p.view(true).is_empty());
    }
    #[test]
    fn view_with_data() {
        let mut p = OutdatedPanel::new();
        p.set_size(80, 40);
        p.set_outdated(sample_outdated());
        assert!(!p.view(true).is_empty());
    }
    #[test]
    fn outdated_detail_nil() {
        assert!(!outdated_detail_view(None, 60, 30, false).is_empty());
    }
    #[test]
    fn outdated_detail_with_package() {
        let pkg = OutdatedPackage {
            name: "vendor/pkg".to_string(),
            version: "1.0.0".to_string(),
            latest: "2.0.0".to_string(),
            latest_status: "update-possible".to_string(),
            description: "A package".to_string(),
            homepage: "https://example.com".to_string(),
            direct_dep: true,
            ..Default::default()
        };
        assert!(!outdated_detail_view(Some(&pkg), 60, 30, true).is_empty());
    }
    #[test]
    fn outdated_detail_with_warning() {
        let pkg = OutdatedPackage {
            name: "vendor/pkg".to_string(),
            version: "1.0.0".to_string(),
            latest: "2.0.0".to_string(),
            latest_status: "update-possible".to_string(),
            warning: "Something is wrong".to_string(),
            ..Default::default()
        };
        assert!(!outdated_detail_view(Some(&pkg), 60, 30, false).is_empty());
    }
    #[test]
    fn outdated_detail_abandoned() {
        let pkg = OutdatedPackage {
            name: "old/pkg".to_string(),
            version: "1.0.0".to_string(),
            latest: "1.0.0".to_string(),
            latest_status: "up-to-date".to_string(),
            abandoned: StringOrBool {
                value: "new/pkg".to_string(),
                set: true,
            },
            ..Default::default()
        };
        assert!(!outdated_detail_view(Some(&pkg), 60, 30, false).is_empty());
    }
    #[test]
    fn outdated_detail_abandoned_no_replacement() {
        let pkg = OutdatedPackage {
            name: "dead/pkg".to_string(),
            version: "1.0.0".to_string(),
            latest: "1.0.0".to_string(),
            latest_status: "up-to-date".to_string(),
            abandoned: StringOrBool {
                value: String::new(),
                set: true,
            },
            ..Default::default()
        };
        assert!(!outdated_detail_view(Some(&pkg), 60, 30, false).is_empty());
    }
    #[test]
    fn outdated_detail_semver_safe() {
        let pkg = OutdatedPackage {
            name: "vendor/pkg".to_string(),
            version: "1.0.0".to_string(),
            latest: "1.0.1".to_string(),
            latest_status: "semver-safe-update".to_string(),
            ..Default::default()
        };
        assert!(!outdated_detail_view(Some(&pkg), 60, 30, false).is_empty());
    }
    #[test]
    fn outdated_detail_transitive() {
        let pkg = OutdatedPackage {
            name: "vendor/pkg".to_string(),
            version: "1.0.0".to_string(),
            latest: "2.0.0".to_string(),
            latest_status: "update-possible".to_string(),
            direct_dep: false,
            ..Default::default()
        };
        assert!(!outdated_detail_view(Some(&pkg), 60, 30, false).is_empty());
    }
    #[test]
    fn test_format_status() {
        for s in ["semver-safe-update", "update-possible", "unknown-status"] {
            assert!(!format_status(s).is_empty());
        }
    }
}
