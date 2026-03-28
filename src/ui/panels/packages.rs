use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::composer::{AuditResult, FrameworkInfo, OutdatedPackage, Package, PackageStatus};
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
    filter_outdated: bool,
    pub prod_scroll: usize,
    pub dev_scroll: usize,
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
            filter_outdated: false,
            prod_scroll: 0,
            dev_scroll: 0,
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
            if self.filter_outdated && pkg.status == PackageStatus::OK {
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
        framework: Option<&FrameworkInfo>,
    ) {
        let mut outdated_names = std::collections::HashSet::new();
        let mut restricted_names = std::collections::HashSet::new();
        let mut abandoned_names = std::collections::HashSet::new();
        let mut vulnerable_names = std::collections::HashSet::new();

        if let Some(outdated) = outdated {
            for o in outdated {
                if crate::composer::is_blocked_by_framework(
                    &o.name, &o.version, &o.latest, framework,
                ) {
                    restricted_names.insert(o.name.clone());
                } else {
                    outdated_names.insert(o.name.clone());
                }
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
            } else if restricted_names.contains(&pkg.name) {
                PackageStatus::Restricted
            } else {
                PackageStatus::OK
            };
        }

        self.rebuild_lists();
    }

    /// Resolves the best version within the framework constraint for a Restricted package.
    /// If the best version is newer than the current version, promote to Outdated.
    pub fn resolve_restricted(&mut self, name: &str, best_version: Option<String>) {
        if let Some(pkg) = self.packages.iter_mut().find(|p| p.name == name) {
            let is_newer = best_version.as_ref().is_some_and(|bv| {
                let best = crate::composer::parser::parse_version(bv);
                let current = crate::composer::parser::parse_version(&pkg.version);
                matches!((best, current), (Some(b), Some(c)) if b > c)
            });
            pkg.restricted_latest = best_version;
            if is_newer {
                pkg.status = PackageStatus::Outdated;
            } else {
                pkg.status = PackageStatus::OK;
            }
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

    pub fn toggle_outdated_filter(&mut self) {
        self.filter_outdated = !self.filter_outdated;
        self.rebuild_lists();
    }

    pub fn is_outdated_filter(&self) -> bool {
        self.filter_outdated
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

        let (items_len, cursor, scroll) = if self.focus_dev {
            (
                self.dev_items.len(),
                &mut self.dev_cursor,
                &mut self.dev_scroll,
            )
        } else {
            (
                self.prod_items.len(),
                &mut self.prod_cursor,
                &mut self.prod_scroll,
            )
        };

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if *cursor > 0 {
                    *cursor -= 1;
                    if *cursor < *scroll {
                        *scroll = *cursor;
                    }
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
    pub fn render(&mut self, area: Rect, buf: &mut Buffer, focused: bool) {
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
        } else if self.filter_outdated {
            " require [outdated] ".to_string()
        } else {
            " require ".to_string()
        };
        let prod_block = Block::default()
            .borders(Borders::ALL)
            .border_style(prod_border_style)
            .title(Span::styled(prod_title, styles::title_style()));
        let prod_inner = prod_block.inner(prod_area);
        prod_block.render(prod_area, buf);
        let prod_items = self.prod_items.clone();
        let prod_cursor = self.prod_cursor;
        let focus_dev = self.focus_dev;
        self.render_list(&prod_items, prod_cursor, !focus_dev, true, prod_inner, buf);

        // Dev panel
        let dev_border_style = if focused && self.focus_dev {
            Style::default().fg(theme::COLOR_BORDER_FOCUS)
        } else {
            Style::default().fg(theme::COLOR_BORDER)
        };
        let dev_block = Block::default()
            .borders(Borders::ALL)
            .border_style(dev_border_style)
            .title(Span::styled(
                if self.filter_outdated {
                    " require-dev [outdated] "
                } else {
                    " require-dev "
                },
                styles::dev_style(),
            ));
        let dev_inner = dev_block.inner(dev_area);
        dev_block.render(dev_area, buf);
        let dev_items = self.dev_items.clone();
        let dev_cursor = self.dev_cursor;
        self.render_list(&dev_items, dev_cursor, focus_dev, false, dev_inner, buf);
    }

    fn render_list(
        &mut self,
        items: &[usize],
        cursor: usize,
        is_focused: bool,
        is_prod: bool,
        area: Rect,
        buf: &mut Buffer,
    ) {
        let visible_height = area.height as usize;
        let scroll = if is_prod {
            &mut self.prod_scroll
        } else {
            &mut self.dev_scroll
        };
        // Adjust scroll only when cursor goes outside the visible window
        if visible_height > 0 && cursor >= *scroll + visible_height {
            *scroll = cursor - visible_height + 1;
        }
        if cursor < *scroll {
            *scroll = cursor;
        }
        let scroll = *scroll;

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
/// Render the framework info panel.
pub fn render_framework_panel(framework: &FrameworkInfo, area: Rect, buf: &mut Buffer) {
    let border_style = Style::default().fg(theme::COLOR_BORDER);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(" Framework ", styles::title_style()));
    let inner = block.inner(area);
    block.render(area, buf);

    let mut lines: Vec<Line> = Vec::new();

    match framework {
        FrameworkInfo::Symfony(sf) => {
            lines.push(Line::from(Span::styled(
                "Symfony",
                Style::default()
                    .fg(theme::COLOR_PRIMARY)
                    .add_modifier(Modifier::BOLD),
            )));
            if !sf.require.is_empty() {
                lines.push(styled_field(
                    "require:",
                    &sf.require,
                    Style::default().fg(theme::COLOR_TEXT),
                ));
            }
            if let Some(contrib) = sf.allow_contrib {
                lines.push(styled_field(
                    "allow-contrib:",
                    if contrib { "true" } else { "false" },
                    Style::default().fg(theme::COLOR_TEXT),
                ));
            }
            if let Some(docker) = sf.docker {
                lines.push(styled_field(
                    "docker:",
                    if docker { "true" } else { "false" },
                    Style::default().fg(theme::COLOR_TEXT),
                ));
            }
        }
    }

    let paragraph = Paragraph::new(lines);
    paragraph.render(inner, buf);
}

/// String-based framework panel view (for tests).
pub fn framework_view(framework: &FrameworkInfo) -> String {
    let mut b = String::new();
    match framework {
        FrameworkInfo::Symfony(sf) => {
            b.push_str("Symfony\n");
            if !sf.require.is_empty() {
                b.push_str(&format!("require:  {}\n", sf.require));
            }
            if let Some(contrib) = sf.allow_contrib {
                b.push_str(&format!("allow-contrib:  {contrib}\n"));
            }
            if let Some(docker) = sf.docker {
                b.push_str(&format!("docker:  {docker}\n"));
            }
        }
    }
    b
}

fn styled_field<'a>(label: &'a str, value: &'a str, value_style: Style) -> Line<'a> {
    Line::from(vec![
        Span::styled(label, styles::key_style()),
        Span::raw("  "),
        Span::styled(value, value_style),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composer::Advisory;

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
        p.update_statuses(Some(&outdated), None, None);
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
        p.update_statuses(None, Some(&audit), None);
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
        p.update_statuses(None, Some(&audit), None);
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
        p.update_statuses(Some(&outdated), Some(&audit), None);
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
        p.update_statuses(Some(&outdated), None, None);
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
        p.update_statuses(None, None, None);
        for pkg in &p.packages {
            assert_eq!(pkg.status, PackageStatus::OK, "{}", pkg.name);
        }
    }
    #[test]
    fn update_statuses_framework_blocks_outdated() {
        let mut p = PackagesPanel::new();
        p.set_size(80, 40);
        p.set_packages(sample_packages());
        let framework = FrameworkInfo::Symfony(crate::composer::SymfonyExtra {
            require: "7.0.*".to_string(),
            allow_contrib: None,
            docker: None,
        });
        // symfony/framework-bundle latest is 7.4.7 which is outside 7.0.*
        let outdated = vec![
            OutdatedPackage {
                name: "symfony/framework-bundle".to_string(),
                version: "v7.0.4".to_string(),
                latest: "v7.4.7".to_string(),
                latest_status: "semver-safe-update".to_string(),
                ..Default::default()
            },
            OutdatedPackage {
                name: "doctrine/orm".to_string(),
                version: "3.0.0".to_string(),
                latest: "3.1.0".to_string(),
                latest_status: "semver-safe-update".to_string(),
                ..Default::default()
            },
        ];
        p.update_statuses(Some(&outdated), None, Some(&framework));
        for pkg in &p.packages {
            if pkg.name == "symfony/framework-bundle" {
                assert_eq!(
                    pkg.status,
                    PackageStatus::Restricted,
                    "symfony pkg should be Restricted when latest is outside framework constraint"
                );
            }
            if pkg.name == "doctrine/orm" {
                assert_eq!(
                    pkg.status,
                    PackageStatus::Outdated,
                    "non-symfony pkg should still be outdated"
                );
            }
        }
    }
    #[test]
    fn update_statuses_framework_allows_outdated_within_constraint() {
        let mut p = PackagesPanel::new();
        p.set_size(80, 40);
        p.set_packages(sample_packages());
        let framework = FrameworkInfo::Symfony(crate::composer::SymfonyExtra {
            require: "7.0.*".to_string(),
            allow_contrib: None,
            docker: None,
        });
        // latest 7.0.5 is within 7.0.* so the package IS outdated
        let outdated = vec![OutdatedPackage {
            name: "symfony/framework-bundle".to_string(),
            version: "v7.0.4".to_string(),
            latest: "v7.0.5".to_string(),
            latest_status: "semver-safe-update".to_string(),
            ..Default::default()
        }];
        p.update_statuses(Some(&outdated), None, Some(&framework));
        for pkg in &p.packages {
            if pkg.name == "symfony/framework-bundle" {
                assert_eq!(
                    pkg.status,
                    PackageStatus::Outdated,
                    "symfony pkg should be outdated when latest is within framework constraint"
                );
            }
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
    fn framework_view_symfony() {
        let fw = FrameworkInfo::Symfony(crate::composer::SymfonyExtra {
            require: "7.0.*".to_string(),
            allow_contrib: Some(false),
            docker: Some(true),
        });
        let view = framework_view(&fw);
        assert!(view.contains("Symfony"));
        assert!(view.contains("7.0.*"));
        assert!(view.contains("allow-contrib:"));
        assert!(view.contains("docker:"));
    }
    #[test]
    fn framework_view_symfony_partial() {
        let fw = FrameworkInfo::Symfony(crate::composer::SymfonyExtra {
            require: "6.4.*".to_string(),
            allow_contrib: None,
            docker: None,
        });
        let view = framework_view(&fw);
        assert!(view.contains("Symfony"));
        assert!(view.contains("6.4.*"));
        assert!(!view.contains("allow-contrib:"));
        assert!(!view.contains("docker:"));
    }
}
