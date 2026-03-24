use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::composer::{AuditResult, OutdatedPackage, Package, PackageStatus};
use crate::ui::style::{styles, theme};

/// PackagesPanel shows the list of installed packages, split into require and require-dev.
pub struct PackagesPanel {
    pub packages: Vec<Package>,
    pub prod_items: Vec<usize>,
    pub dev_items: Vec<usize>,
    pub prod_cursor: usize,
    pub dev_cursor: usize,
    pub focus_dev: bool,
    pub width: u16,
    pub height: u16,
    filtering: bool,
    filter_text: String,
}

impl Default for PackagesPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl PackagesPanel {
    pub fn new() -> Self {
        PackagesPanel {
            packages: vec![],
            prod_items: vec![],
            dev_items: vec![],
            prod_cursor: 0,
            dev_cursor: 0,
            focus_dev: false,
            width: 0,
            height: 0,
            filtering: false,
            filter_text: String::new(),
        }
    }

    pub fn set_packages(&mut self, packages: Vec<Package>) {
        self.packages = packages;
        self.rebuild_lists();
    }

    fn rebuild_lists(&mut self) {
        self.prod_items.clear();
        self.dev_items.clear();
        let filter = self.filter_text.to_lowercase();
        for (i, pkg) in self.packages.iter().enumerate() {
            if !filter.is_empty() && !pkg.name.to_lowercase().contains(&filter) {
                continue;
            }
            if pkg.is_dev {
                self.dev_items.push(i);
            } else {
                self.prod_items.push(i);
            }
        }
        // Clamp cursors
        if !self.prod_items.is_empty() {
            self.prod_cursor = self.prod_cursor.min(self.prod_items.len() - 1);
        } else {
            self.prod_cursor = 0;
        }
        if !self.dev_items.is_empty() {
            self.dev_cursor = self.dev_cursor.min(self.dev_items.len() - 1);
        } else {
            self.dev_cursor = 0;
        }
    }

    pub fn start_filter(&mut self) {
        self.filtering = true;
        self.filter_text.clear();
    }

    pub fn stop_filter(&mut self) {
        self.filtering = false;
        self.filter_text.clear();
        self.rebuild_lists();
    }

    pub fn update_statuses(
        &mut self,
        outdated: Option<&[OutdatedPackage]>,
        audit: Option<&AuditResult>,
    ) {
        let mut outdated_names = std::collections::HashSet::new();
        let mut abandoned_names = std::collections::HashSet::new();
        let mut vulnerable_names = std::collections::HashSet::new();

        if let Some(outdated) = outdated {
            for o in outdated {
                outdated_names.insert(o.name.clone());
                if o.abandoned.set {
                    abandoned_names.insert(o.name.clone());
                }
            }
        }

        if let Some(audit) = audit {
            for name in audit.abandoned.keys() {
                abandoned_names.insert(name.clone());
            }
            for (name, advs) in &audit.advisories {
                if !advs.is_empty() {
                    vulnerable_names.insert(name.clone());
                }
            }
        }

        for pkg in &mut self.packages {
            pkg.status = if vulnerable_names.contains(&pkg.name) {
                PackageStatus::Vulnerable
            } else if abandoned_names.contains(&pkg.name) {
                PackageStatus::Abandoned
            } else if outdated_names.contains(&pkg.name) {
                PackageStatus::Outdated
            } else {
                PackageStatus::OK
            };
        }

        self.rebuild_lists();
    }

    pub fn selected_package(&self) -> Option<&Package> {
        let (items, cursor) = if self.focus_dev {
            (&self.dev_items, self.dev_cursor)
        } else {
            (&self.prod_items, self.prod_cursor)
        };
        items.get(cursor).map(|&i| &self.packages[i])
    }

    pub fn set_size(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
    }

    pub fn is_filtering(&self) -> bool {
        self.filtering
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        // Filter mode input handling
        if self.filtering {
            match key.code {
                KeyCode::Esc => {
                    self.stop_filter();
                }
                KeyCode::Enter => {
                    // Accept filter, exit filter mode but keep the filter text
                    self.filtering = false;
                }
                KeyCode::Backspace => {
                    self.filter_text.pop();
                    self.rebuild_lists();
                }
                KeyCode::Char(c) => {
                    self.filter_text.push(c);
                    self.rebuild_lists();
                }
                _ => {}
            }
            return;
        }

        if key.code == KeyCode::Tab || key.code == KeyCode::BackTab {
            // Handled by app.rs for global tab cycling
            return;
        }

        let (items_len, cursor) = if self.focus_dev {
            (self.dev_items.len(), &mut self.dev_cursor)
        } else {
            (self.prod_items.len(), &mut self.prod_cursor)
        };

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if *cursor > 0 {
                    *cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if items_len > 0 && *cursor < items_len - 1 {
                    *cursor += 1;
                }
            }
            KeyCode::Esc => {
                // Clear filter if any
                if !self.filter_text.is_empty() {
                    self.filter_text.clear();
                    self.rebuild_lists();
                }
            }
            _ => {}
        }
    }

    /// Render the packages panel (two stacked sub-panels: require + require-dev)
    pub fn render(&self, area: Rect, buf: &mut Buffer, focused: bool) {
        let half_h = area.height / 2;
        let prod_h = half_h;
        let dev_h = area.height.saturating_sub(prod_h);

        let prod_area = Rect::new(area.x, area.y, area.width, prod_h);
        let dev_area = Rect::new(area.x, area.y + prod_h, area.width, dev_h);

        // Prod panel
        let prod_border_style = if focused && !self.focus_dev {
            Style::default().fg(theme::COLOR_BORDER_FOCUS)
        } else {
            Style::default().fg(theme::COLOR_BORDER)
        };
        let prod_title = if !self.filter_text.is_empty() || self.filtering {
            let filter_indicator = if self.filtering { "/" } else { "" };
            format!(" require [{}{}] ", filter_indicator, self.filter_text)
        } else {
            " require ".to_string()
        };
        let prod_block = Block::default()
            .borders(Borders::ALL)
            .border_style(prod_border_style)
            .title(Span::styled(prod_title, styles::title_style()));
        let prod_inner = prod_block.inner(prod_area);
        prod_block.render(prod_area, buf);
        self.render_list(
            &self.prod_items,
            self.prod_cursor,
            !self.focus_dev,
            prod_inner,
            buf,
        );

        // Dev panel
        let dev_border_style = if focused && self.focus_dev {
            Style::default().fg(theme::COLOR_BORDER_FOCUS)
        } else {
            Style::default().fg(theme::COLOR_BORDER)
        };
        let dev_block = Block::default()
            .borders(Borders::ALL)
            .border_style(dev_border_style)
            .title(Span::styled(" require-dev ", styles::dev_style()));
        let dev_inner = dev_block.inner(dev_area);
        dev_block.render(dev_area, buf);
        self.render_list(
            &self.dev_items,
            self.dev_cursor,
            self.focus_dev,
            dev_inner,
            buf,
        );
    }

    fn render_list(
        &self,
        items: &[usize],
        cursor: usize,
        is_focused: bool,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let visible_height = area.height as usize;
        let scroll = if cursor >= visible_height {
            cursor - visible_height + 1
        } else {
            0
        };

        for (i, &idx) in items.iter().enumerate().skip(scroll).take(visible_height) {
            let y = area.y + (i - scroll) as u16;
            if y >= area.y + area.height {
                break;
            }

            let pkg = &self.packages[idx];
            let selected = is_focused && i == cursor;
            let name_style = styles::package_status_style(pkg.status);
            let version_style = styles::version_style();

            let prefix = if selected { "> " } else { "  " };
            let prefix_style = if selected {
                Style::default()
                    .fg(theme::COLOR_PRIMARY)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let line = Line::from(vec![
                Span::styled(prefix, prefix_style),
                Span::styled(&pkg.name, name_style),
                Span::raw(" "),
                Span::styled(&pkg.version, version_style),
            ]);

            buf.set_line(area.x, y, &line, area.width);
        }

        // Show count at bottom if space
        if items.len() > visible_height && visible_height > 0 {
            let info = format!(" {}/{} ", cursor + 1, items.len());
            let info_style = styles::muted_style();
            let x = area.x + area.width.saturating_sub(info.len() as u16);
            buf.set_string(x, area.y + area.height.saturating_sub(1), &info, info_style);
        }
    }

    /// String-based view (for tests)
    pub fn view(&self, _focused: bool) -> String {
        let mut result = String::new();
        result.push_str("require\n");
        for (i, &idx) in self.prod_items.iter().enumerate() {
            let pkg = &self.packages[idx];
            let prefix = if !self.focus_dev && i == self.prod_cursor {
                "> "
            } else {
                "  "
            };
            result.push_str(&format!("{prefix}{} {}\n", pkg.name, pkg.version));
        }
        result.push_str("\nrequire-dev\n");
        for (i, &idx) in self.dev_items.iter().enumerate() {
            let pkg = &self.packages[idx];
            let prefix = if self.focus_dev && i == self.dev_cursor {
                "> "
            } else {
                "  "
            };
            result.push_str(&format!("{prefix}{} {}\n", pkg.name, pkg.version));
        }
        result
    }
}

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

    // Outdated info: latest version and update type
    if let Some(o) = outdated {
        let status_color = theme::status_color(&o.latest_status);
        let version_style = Style::default().fg(status_color);
        lines.push(Line::from(vec![
            Span::styled("Latest:", styles::key_style()),
            Span::raw("  "),
            Span::styled(&o.latest, version_style),
        ]));
        let status_label = match o.latest_status.as_str() {
            "semver-safe-update" => "● Safe update (minor/patch)",
            "update-possible" => "▲ Update possible (major)",
            _ => &o.latest_status,
        };
        lines.push(Line::from(vec![
            Span::styled("Update:", styles::key_style()),
            Span::raw("  "),
            Span::styled(status_label.to_string(), version_style),
        ]));

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
        lines.push(styled_field(
            "Description:",
            &pkg.description,
            Style::default().fg(theme::COLOR_TEXT),
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
        _ => "OK",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composer::{Advisory, Source};

    fn sample_packages() -> Vec<Package> {
        vec![
            Package {
                name: "symfony/framework-bundle".to_string(),
                version: "v7.0.4".to_string(),
                is_dev: false,
                ..Default::default()
            },
            Package {
                name: "doctrine/orm".to_string(),
                version: "3.0.0".to_string(),
                is_dev: false,
                ..Default::default()
            },
            Package {
                name: "twig/twig".to_string(),
                version: "v3.8.0".to_string(),
                is_dev: false,
                ..Default::default()
            },
            Package {
                name: "phpunit/phpunit".to_string(),
                version: "11.0.0".to_string(),
                is_dev: true,
                ..Default::default()
            },
            Package {
                name: "phpstan/phpstan".to_string(),
                version: "1.10.50".to_string(),
                is_dev: true,
                ..Default::default()
            },
        ]
    }

    #[test]
    fn set_packages_splits_prod_and_dev() {
        let mut p = PackagesPanel::new();
        p.set_size(80, 40);
        p.set_packages(sample_packages());
        assert_eq!(p.prod_items.len(), 3);
        assert_eq!(p.dev_items.len(), 2);
    }
    #[test]
    fn set_packages_no_dev() {
        let mut p = PackagesPanel::new();
        p.set_size(80, 40);
        p.set_packages(vec![Package {
            name: "symfony/console".to_string(),
            version: "v7.0.0".to_string(),
            is_dev: false,
            ..Default::default()
        }]);
        assert_eq!(p.prod_items.len(), 1);
        assert_eq!(p.dev_items.len(), 0);
    }
    #[test]
    fn set_packages_empty() {
        let mut p = PackagesPanel::new();
        p.set_size(80, 40);
        p.set_packages(vec![]);
        assert_eq!(p.prod_items.len(), 0);
        assert_eq!(p.dev_items.len(), 0);
    }
    #[test]
    fn selected_package_prod_focused() {
        let mut p = PackagesPanel::new();
        p.set_size(80, 40);
        p.set_packages(sample_packages());
        let pkg = p.selected_package().unwrap();
        assert!(!pkg.is_dev);
    }
    #[test]
    fn selected_package_dev_focused() {
        let mut p = PackagesPanel::new();
        p.set_size(80, 40);
        p.set_packages(sample_packages());
        p.focus_dev = true;
        let pkg = p.selected_package().unwrap();
        assert!(pkg.is_dev);
    }
    #[test]
    fn selected_package_empty_list() {
        let p = PackagesPanel::new();
        assert!(p.selected_package().is_none());
    }
    #[test]
    fn focus_toggle() {
        let mut p = PackagesPanel::new();
        p.set_size(80, 40);
        p.set_packages(sample_packages());
        assert!(!p.focus_dev);
        p.focus_dev = true;
        assert!(p.focus_dev);
        p.focus_dev = false;
        assert!(!p.focus_dev);
    }
    #[test]
    fn update_statuses_outdated() {
        let mut p = PackagesPanel::new();
        p.set_size(80, 40);
        p.set_packages(sample_packages());
        let outdated = vec![OutdatedPackage {
            name: "doctrine/orm".to_string(),
            version: "3.0.0".to_string(),
            latest: "3.1.0".to_string(),
            latest_status: "semver-safe-update".to_string(),
            ..Default::default()
        }];
        p.update_statuses(Some(&outdated), None);
        for pkg in &p.packages {
            if pkg.name == "doctrine/orm" {
                assert_eq!(pkg.status, PackageStatus::Outdated);
            }
            if pkg.name == "symfony/framework-bundle" {
                assert_eq!(pkg.status, PackageStatus::OK);
            }
        }
    }
    #[test]
    fn update_statuses_abandoned() {
        let mut p = PackagesPanel::new();
        p.set_size(80, 40);
        p.set_packages(sample_packages());
        let audit = AuditResult {
            advisories: Default::default(),
            abandoned: [("twig/twig".to_string(), Some("new/pkg".to_string()))]
                .into_iter()
                .collect(),
        };
        p.update_statuses(None, Some(&audit));
        for pkg in &p.packages {
            if pkg.name == "twig/twig" {
                assert_eq!(pkg.status, PackageStatus::Abandoned);
            }
        }
    }
    #[test]
    fn update_statuses_vulnerable() {
        let mut p = PackagesPanel::new();
        p.set_size(80, 40);
        p.set_packages(sample_packages());
        let audit = AuditResult {
            advisories: [(
                "symfony/framework-bundle".to_string(),
                vec![Advisory {
                    advisory_id: "ADV-001".to_string(),
                    title: "XSS".to_string(),
                    ..Default::default()
                }],
            )]
            .into_iter()
            .collect(),
            abandoned: Default::default(),
        };
        p.update_statuses(None, Some(&audit));
        for pkg in &p.packages {
            if pkg.name == "symfony/framework-bundle" {
                assert_eq!(pkg.status, PackageStatus::Vulnerable);
            }
        }
    }
    #[test]
    fn update_statuses_priority() {
        let mut p = PackagesPanel::new();
        p.set_size(80, 40);
        p.set_packages(sample_packages());
        let outdated = vec![OutdatedPackage {
            name: "doctrine/orm".to_string(),
            version: "3.0.0".to_string(),
            latest: "3.1.0".to_string(),
            ..Default::default()
        }];
        let audit = AuditResult {
            advisories: [(
                "doctrine/orm".to_string(),
                vec![Advisory {
                    advisory_id: "ADV-002".to_string(),
                    title: "SQL injection".to_string(),
                    ..Default::default()
                }],
            )]
            .into_iter()
            .collect(),
            abandoned: Default::default(),
        };
        p.update_statuses(Some(&outdated), Some(&audit));
        for pkg in &p.packages {
            if pkg.name == "doctrine/orm" {
                assert_eq!(pkg.status, PackageStatus::Vulnerable);
            }
        }
    }
    #[test]
    fn update_statuses_abandoned_from_outdated() {
        let mut p = PackagesPanel::new();
        p.set_size(80, 40);
        p.set_packages(sample_packages());
        let outdated = vec![OutdatedPackage {
            name: "twig/twig".to_string(),
            version: "v3.8.0".to_string(),
            abandoned: crate::composer::StringOrBool {
                value: "twig/twig2".to_string(),
                set: true,
            },
            ..Default::default()
        }];
        p.update_statuses(Some(&outdated), None);
        for pkg in &p.packages {
            if pkg.name == "twig/twig" {
                assert_eq!(pkg.status, PackageStatus::Abandoned);
            }
        }
    }
    #[test]
    fn update_statuses_nil_audit_and_outdated() {
        let mut p = PackagesPanel::new();
        p.set_size(80, 40);
        p.set_packages(sample_packages());
        p.update_statuses(None, None);
        for pkg in &p.packages {
            assert_eq!(pkg.status, PackageStatus::OK, "{}", pkg.name);
        }
    }
    #[test]
    fn set_size() {
        let mut p = PackagesPanel::new();
        p.set_size(100, 50);
        assert_eq!(p.width, 100);
        assert_eq!(p.height, 50);
    }
    #[test]
    fn view_not_empty() {
        let mut p = PackagesPanel::new();
        p.set_size(80, 40);
        p.set_packages(sample_packages());
        assert!(!p.view(true).is_empty());
    }
    #[test]
    fn view_unfocused() {
        let mut p = PackagesPanel::new();
        p.set_size(80, 40);
        p.set_packages(sample_packages());
        assert!(!p.view(false).is_empty());
    }
    #[test]
    fn is_filtering() {
        let mut p = PackagesPanel::new();
        p.set_size(80, 40);
        p.set_packages(sample_packages());
        assert!(!p.is_filtering());
    }
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
        };
        assert!(!detail_view(Some(&pkg), 60, 30, true).is_empty());
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
    fn test_status_label() {
        for s in [
            PackageStatus::OK,
            PackageStatus::Outdated,
            PackageStatus::Abandoned,
            PackageStatus::Vulnerable,
        ] {
            assert!(!status_label(s).is_empty());
        }
    }
}
