use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::composer::{Advisory, AuditResult, OutdatedPackage, Package, WhyEntry};
use crate::ui::style::{styles, theme};
use crate::ui::text::wrap_field;

/// Public enum for exposing selected audit entry to the detail panel.
pub enum SelectedAuditEntry<'a> {
    Advisory {
        pkg: &'a str,
        advisory: &'a Advisory,
        installed_version: &'a str,
        latest_version: &'a str,
        is_direct: bool,
        required_by: &'a [WhyEntry],
    },
    Abandoned {
        pkg: &'a str,
        replaced_by: &'a str,
    },
}

struct AdvisoryEntry {
    pkg: String,
    advisory: Advisory,
    installed_version: String,
    latest_version: String,
    is_direct: bool,
    required_by: Vec<WhyEntry>,
}

struct AbandonEntry {
    pkg: String,
    replaced_by: String,
}

pub struct AuditPanel {
    advisories: Vec<AdvisoryEntry>,
    abandoned: Vec<AbandonEntry>,
    pub cursor: usize,
    pub offset: usize,
    pub width: u16,
    pub height: u16,
}

impl Default for AuditPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl AuditPanel {
    pub fn new() -> Self {
        AuditPanel {
            advisories: vec![],
            abandoned: vec![],
            cursor: 0,
            offset: 0,
            width: 0,
            height: 0,
        }
    }

    pub fn set_audit(&mut self, result: Option<&AuditResult>) {
        self.advisories.clear();
        self.abandoned.clear();
        self.cursor = 0;
        self.offset = 0;
        let result = match result {
            Some(r) => r,
            None => return,
        };
        for (pkg, advs) in &result.advisories {
            for adv in advs {
                self.advisories.push(AdvisoryEntry {
                    pkg: pkg.clone(),
                    advisory: adv.clone(),
                    installed_version: String::new(),
                    latest_version: String::new(),
                    is_direct: false,
                    required_by: vec![],
                });
            }
        }
        for (pkg, replacement) in &result.abandoned {
            self.abandoned.push(AbandonEntry {
                pkg: pkg.clone(),
                replaced_by: replacement.clone().unwrap_or_default(),
            });
        }
    }

    pub fn total_items(&self) -> usize {
        self.advisories.len() + self.abandoned.len()
    }

    pub fn set_size(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
    }

    /// Update installed/latest version info by cross-referencing with packages and outdated data.
    pub fn update_versions(&mut self, packages: &[Package], outdated: &[OutdatedPackage]) {
        for entry in &mut self.advisories {
            // Find installed version from packages (only direct deps are in this list)
            if let Some(pkg) = packages.iter().find(|p| p.name == entry.pkg) {
                entry.installed_version = pkg.version.clone();
                entry.is_direct = true;
            } else {
                entry.is_direct = false;
            }
            // Find latest version from outdated data
            if let Some(out) = outdated.iter().find(|o| o.name == entry.pkg) {
                if entry.installed_version.is_empty() {
                    entry.installed_version = out.version.clone();
                }
                entry.latest_version = out.latest.clone();
            }
        }
    }

    /// Returns package names of advisories that are transitive (not direct deps).
    pub fn transitive_advisory_packages(&self) -> Vec<String> {
        self.advisories
            .iter()
            .filter(|e| !e.is_direct)
            .map(|e| e.pkg.clone())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect()
    }

    /// Update the "required by" info for a transitive advisory package.
    pub fn set_why_result(&mut self, pkg: &str, entries: Vec<WhyEntry>) {
        for advisory in &mut self.advisories {
            if advisory.pkg == pkg {
                advisory.required_by = entries.clone();
            }
        }
    }

    pub fn selected_entry(&self) -> Option<SelectedAuditEntry<'_>> {
        if self.cursor < self.advisories.len() {
            let entry = &self.advisories[self.cursor];
            return Some(SelectedAuditEntry::Advisory {
                pkg: &entry.pkg,
                advisory: &entry.advisory,
                installed_version: &entry.installed_version,
                latest_version: &entry.latest_version,
                is_direct: entry.is_direct,
                required_by: &entry.required_by,
            });
        }
        let abandon_idx = self.cursor - self.advisories.len();
        if abandon_idx < self.abandoned.len() {
            let entry = &self.abandoned[abandon_idx];
            return Some(SelectedAuditEntry::Abandoned {
                pkg: &entry.pkg,
                replaced_by: &entry.replaced_by,
            });
        }
        None
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        let total = self.total_items();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    if self.cursor < self.offset {
                        self.offset = self.cursor;
                    }
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if total > 0 && self.cursor < total - 1 {
                    self.cursor += 1;
                    let visible = if self.height > 6 {
                        (self.height - 6) as usize
                    } else {
                        1
                    };
                    if self.cursor >= self.offset + visible {
                        self.offset = self.cursor - visible + 1;
                    }
                }
            }
            _ => {}
        }
    }

    /// Render the audit panel with full colors.
    pub fn render(&self, area: Rect, buf: &mut Buffer, focused: bool) {
        let border_style = if focused {
            Style::default().fg(theme::COLOR_BORDER_FOCUS)
        } else {
            Style::default().fg(theme::COLOR_BORDER)
        };
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(border_style)
            .title(Span::styled(" Audit ", styles::title_style()));
        let inner = block.inner(area);
        block.render(area, buf);

        if self.total_items() == 0 {
            Paragraph::new(Span::styled(
                "✓ No security issues found",
                styles::success_style(),
            ))
            .render(inner, buf);
            return;
        }

        let mut lines: Vec<Line> = Vec::new();
        let mut item_idx = 0;

        if !self.advisories.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("⚠ Security Advisories ({})", self.advisories.len()),
                styles::error_style(),
            )));
            lines.push(Line::default());

            for entry in &self.advisories {
                let cursor = if item_idx == self.cursor {
                    Span::styled("> ", styles::key_style())
                } else {
                    Span::raw("  ")
                };
                let cve = entry
                    .advisory
                    .cve
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .unwrap_or(&entry.advisory.advisory_id);
                let mut spans = vec![
                    cursor,
                    Span::styled(cve.to_string(), styles::error_style()),
                    Span::raw(" "),
                    Span::raw(&entry.pkg),
                ];
                if !entry.installed_version.is_empty() {
                    spans.push(Span::styled(
                        format!(" ({})", entry.installed_version),
                        styles::muted_style(),
                    ));
                    if !entry.latest_version.is_empty() {
                        spans.push(Span::styled(
                            format!(" → {}", entry.latest_version),
                            styles::version_style(),
                        ));
                    }
                }
                lines.push(Line::from(spans));
                let mut detail_spans = vec![Span::raw("    "), Span::raw(&entry.advisory.title)];
                if !entry.is_direct && !entry.required_by.is_empty() {
                    let names: Vec<&str> =
                        entry.required_by.iter().map(|w| w.name.as_str()).collect();
                    detail_spans.push(Span::styled(
                        format!("  via {}", names.join(", ")),
                        styles::warning_style(),
                    ));
                }
                lines.push(Line::from(detail_spans));
                item_idx += 1;
            }
        }

        if !self.abandoned.is_empty() {
            if !self.advisories.is_empty() {
                lines.push(Line::default());
            }
            lines.push(Line::from(Span::styled(
                format!("⚑ Abandoned Packages ({})", self.abandoned.len()),
                styles::warning_style(),
            )));
            lines.push(Line::default());

            for entry in &self.abandoned {
                let cursor = if item_idx == self.cursor {
                    Span::styled("> ", styles::key_style())
                } else {
                    Span::raw("  ")
                };
                let replacement = if entry.replaced_by.is_empty() {
                    "no replacement"
                } else {
                    &entry.replaced_by
                };
                lines.push(Line::from(vec![
                    cursor,
                    Span::styled(&entry.pkg, styles::warning_style()),
                    Span::raw("  "),
                    Span::styled(format!("→ {replacement}"), styles::muted_style()),
                ]));
                item_idx += 1;
            }
        }

        Paragraph::new(lines)
            .scroll((self.offset as u16, 0))
            .render(inner, buf);
    }

    pub fn view(&self, _focused: bool) -> String {
        if self.total_items() == 0 {
            return "✓ No security issues found".to_string();
        }
        let mut b = String::new();
        let mut item_idx = 0;
        if !self.advisories.is_empty() {
            b.push_str(&format!(
                "⚠ Security Advisories ({})\n\n",
                self.advisories.len()
            ));
            for entry in &self.advisories {
                let cursor = if item_idx == self.cursor { "> " } else { "  " };
                let cve = entry
                    .advisory
                    .cve
                    .as_deref()
                    .filter(|s| !s.is_empty())
                    .unwrap_or(&entry.advisory.advisory_id);
                let mut line = format!("{cursor}{cve} {}", entry.pkg);
                if !entry.installed_version.is_empty() {
                    line.push_str(&format!(" ({})", entry.installed_version));
                    if !entry.latest_version.is_empty() {
                        line.push_str(&format!(" → {}", entry.latest_version));
                    }
                }
                b.push_str(&line);
                b.push('\n');
                b.push_str(&format!("    {}\n", entry.advisory.title));
                item_idx += 1;
            }
        }
        if !self.abandoned.is_empty() {
            if !self.advisories.is_empty() {
                b.push('\n');
            }
            b.push_str(&format!(
                "⚑ Abandoned Packages ({})\n\n",
                self.abandoned.len()
            ));
            for entry in &self.abandoned {
                let cursor = if item_idx == self.cursor { "> " } else { "  " };
                let replacement = if entry.replaced_by.is_empty() {
                    "no replacement"
                } else {
                    &entry.replaced_by
                };
                b.push_str(&format!("{cursor}{}  → {replacement}\n", entry.pkg));
                item_idx += 1;
            }
        }
        b
    }
}

/// Render the detail panel for the selected audit entry.
pub fn render_audit_detail(
    entry: Option<SelectedAuditEntry<'_>>,
    area: Rect,
    buf: &mut Buffer,
    focused: bool,
    scroll: &mut u16,
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

    let entry = match entry {
        Some(e) => e,
        None => {
            Paragraph::new(Span::styled("No advisory selected", styles::muted_style()))
                .render(inner, buf);
            return;
        }
    };

    let lines = match entry {
        SelectedAuditEntry::Advisory {
            pkg,
            advisory,
            installed_version,
            latest_version,
            is_direct,
            required_by,
        } => build_advisory_detail_lines(
            pkg,
            advisory,
            installed_version,
            latest_version,
            is_direct,
            required_by,
            inner.width,
        ),
        SelectedAuditEntry::Abandoned { pkg, replaced_by } => {
            build_abandoned_detail_lines(pkg, replaced_by)
        }
    };

    let max_scroll = (lines.len() as u16).saturating_sub(inner.height);
    if *scroll > max_scroll {
        *scroll = max_scroll;
    }

    Paragraph::new(lines)
        .scroll((*scroll, 0))
        .render(inner, buf);
}

fn severity_style(severity: &str) -> Style {
    match severity.to_lowercase().as_str() {
        "critical" => styles::error_style(),
        "high" => Style::default()
            .fg(theme::COLOR_DANGER)
            .add_modifier(ratatui::style::Modifier::empty()),
        "medium" => styles::warning_style(),
        "low" => styles::muted_style(),
        _ => styles::description_style(),
    }
}

fn build_advisory_detail_lines<'a>(
    pkg: &'a str,
    advisory: &'a Advisory,
    installed_version: &'a str,
    latest_version: &'a str,
    is_direct: bool,
    required_by: &'a [WhyEntry],
    width: u16,
) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(pkg, styles::title_style())),
        Line::default(),
    ];

    // Title
    lines.extend(wrap_field(
        "Title:",
        &advisory.title,
        styles::description_style(),
        width,
    ));

    // Advisory ID
    lines.push(Line::from(vec![
        Span::styled("Advisory:  ", styles::key_style()),
        Span::raw(&advisory.advisory_id),
    ]));

    // CVE
    if let Some(cve) = advisory.cve.as_deref().filter(|s| !s.is_empty()) {
        lines.push(Line::from(vec![
            Span::styled("CVE:       ", styles::key_style()),
            Span::styled(cve, styles::error_style()),
        ]));
    }

    // Severity
    if let Some(severity) = advisory.severity.as_deref().filter(|s| !s.is_empty()) {
        lines.push(Line::from(vec![
            Span::styled("Severity:  ", styles::key_style()),
            Span::styled(severity, severity_style(severity)),
        ]));
    }

    // Affected versions
    if !advisory.affected_versions.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Affected:  ", styles::key_style()),
            Span::styled(&advisory.affected_versions, styles::warning_style()),
        ]));
    }

    // Installed version
    if !installed_version.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Installed: ", styles::key_style()),
            Span::styled(installed_version, styles::error_style()),
        ]));
    }

    // Latest available version (fix target)
    if !latest_version.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Update to: ", styles::key_style()),
            Span::styled(latest_version, styles::version_style()),
        ]));
    }

    // Dependency type
    if !is_direct && !required_by.is_empty() {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            "⚑ Transitive dependency",
            styles::warning_style(),
        )));
        for why_entry in required_by {
            let mut spans: Vec<Span> = vec![
                Span::styled("  Required by: ", styles::key_style()),
                Span::styled(why_entry.name.as_str(), styles::version_style()),
            ];
            if !why_entry.version.is_empty() {
                spans.push(Span::styled(
                    format!(" ({})", why_entry.version),
                    styles::muted_style(),
                ));
            }
            lines.push(Line::from(spans));
        }
    } else if !is_direct {
        lines.push(Line::default());
        lines.push(Line::from(Span::styled(
            "⚑ Transitive dependency (loading...)",
            styles::warning_style(),
        )));
    }

    // Reported at
    if !advisory.reported_at.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Reported:  ", styles::key_style()),
            Span::raw(&advisory.reported_at),
        ]));
    }

    // Link
    if !advisory.link.is_empty() {
        lines.push(Line::default());
        lines.push(Line::from(vec![
            Span::styled("Link:  ", styles::key_style()),
            Span::styled(&advisory.link, Style::default().fg(theme::COLOR_INFO)),
        ]));
    }

    lines
}

fn build_abandoned_detail_lines<'a>(pkg: &'a str, replaced_by: &'a str) -> Vec<Line<'a>> {
    let mut lines: Vec<Line> = vec![
        Line::from(Span::styled(pkg, styles::title_style())),
        Line::default(),
        Line::from(Span::styled("⚑ Abandoned Package", styles::warning_style())),
        Line::default(),
    ];

    if replaced_by.is_empty() {
        lines.push(Line::from(Span::styled(
            "No replacement suggested",
            styles::muted_style(),
        )));
    } else {
        lines.push(Line::from(vec![
            Span::styled("Replace with:  ", styles::key_style()),
            Span::styled(replaced_by, styles::version_style()),
        ]));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_audit_nil() {
        let mut p = AuditPanel::new();
        p.set_audit(None);
        assert_eq!(p.total_items(), 0);
    }
    #[test]
    fn set_audit_empty() {
        let mut p = AuditPanel::new();
        p.set_audit(Some(&AuditResult {
            advisories: Default::default(),
            abandoned: Default::default(),
        }));
        assert_eq!(p.total_items(), 0);
    }
    #[test]
    fn set_audit_with_advisories() {
        let mut p = AuditPanel::new();
        p.set_audit(Some(&AuditResult {
            advisories: [(
                "vendor/pkg".to_string(),
                vec![
                    Advisory {
                        advisory_id: "ADV-001".to_string(),
                        title: "XSS".to_string(),
                        cve: Some("CVE-2024-0001".to_string()),
                        ..Default::default()
                    },
                    Advisory {
                        advisory_id: "ADV-002".to_string(),
                        title: "CSRF".to_string(),
                        ..Default::default()
                    },
                ],
            )]
            .into_iter()
            .collect(),
            abandoned: Default::default(),
        }));
        assert_eq!(p.total_items(), 2);
    }
    #[test]
    fn set_audit_with_abandoned() {
        let mut p = AuditPanel::new();
        p.set_audit(Some(&AuditResult {
            advisories: Default::default(),
            abandoned: [
                ("old/pkg".to_string(), Some("new/pkg".to_string())),
                ("dead/pkg".to_string(), None),
            ]
            .into_iter()
            .collect(),
        }));
        assert_eq!(p.total_items(), 2);
    }
    #[test]
    fn set_audit_mixed() {
        let mut p = AuditPanel::new();
        p.set_audit(Some(&AuditResult {
            advisories: [(
                "vendor/pkg".to_string(),
                vec![Advisory {
                    advisory_id: "ADV-001".to_string(),
                    title: "Bug".to_string(),
                    ..Default::default()
                }],
            )]
            .into_iter()
            .collect(),
            abandoned: [("old/pkg".to_string(), None)].into_iter().collect(),
        }));
        assert_eq!(p.total_items(), 2);
    }
    #[test]
    fn set_audit_resets_state() {
        let mut p = AuditPanel::new();
        p.set_audit(Some(&AuditResult {
            advisories: [(
                "pkg".to_string(),
                vec![Advisory {
                    advisory_id: "ADV-001".to_string(),
                    title: "Bug".to_string(),
                    ..Default::default()
                }],
            )]
            .into_iter()
            .collect(),
            abandoned: Default::default(),
        }));
        p.handle_key(KeyEvent::new(
            KeyCode::Down,
            crossterm::event::KeyModifiers::NONE,
        ));
        p.set_audit(Some(&AuditResult {
            advisories: Default::default(),
            abandoned: Default::default(),
        }));
        assert_eq!(p.cursor, 0);
        assert_eq!(p.offset, 0);
    }
    #[test]
    fn navigate_down() {
        let mut p = AuditPanel::new();
        p.set_size(80, 40);
        p.set_audit(Some(&AuditResult {
            advisories: [
                (
                    "pkg1".to_string(),
                    vec![Advisory {
                        advisory_id: "ADV-001".to_string(),
                        title: "Bug1".to_string(),
                        ..Default::default()
                    }],
                ),
                (
                    "pkg2".to_string(),
                    vec![Advisory {
                        advisory_id: "ADV-002".to_string(),
                        title: "Bug2".to_string(),
                        ..Default::default()
                    }],
                ),
            ]
            .into_iter()
            .collect(),
            abandoned: Default::default(),
        }));
        assert_eq!(p.cursor, 0);
        p.handle_key(KeyEvent::new(
            KeyCode::Down,
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(p.cursor, 1);
    }
    #[test]
    fn navigate_up() {
        let mut p = AuditPanel::new();
        p.set_size(80, 40);
        p.set_audit(Some(&AuditResult {
            advisories: [(
                "pkg".to_string(),
                vec![
                    Advisory {
                        advisory_id: "ADV-001".to_string(),
                        title: "Bug1".to_string(),
                        ..Default::default()
                    },
                    Advisory {
                        advisory_id: "ADV-002".to_string(),
                        title: "Bug2".to_string(),
                        ..Default::default()
                    },
                ],
            )]
            .into_iter()
            .collect(),
            abandoned: Default::default(),
        }));
        p.handle_key(KeyEvent::new(
            KeyCode::Down,
            crossterm::event::KeyModifiers::NONE,
        ));
        p.handle_key(KeyEvent::new(
            KeyCode::Up,
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(p.cursor, 0);
    }
    #[test]
    fn navigate_up_at_top() {
        let mut p = AuditPanel::new();
        p.set_size(80, 40);
        p.set_audit(Some(&AuditResult {
            advisories: [(
                "pkg".to_string(),
                vec![Advisory {
                    advisory_id: "ADV-001".to_string(),
                    title: "Bug".to_string(),
                    ..Default::default()
                }],
            )]
            .into_iter()
            .collect(),
            abandoned: Default::default(),
        }));
        p.handle_key(KeyEvent::new(
            KeyCode::Up,
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(p.cursor, 0);
    }
    #[test]
    fn navigate_down_at_bottom() {
        let mut p = AuditPanel::new();
        p.set_size(80, 40);
        p.set_audit(Some(&AuditResult {
            advisories: [(
                "pkg".to_string(),
                vec![Advisory {
                    advisory_id: "ADV-001".to_string(),
                    title: "Bug".to_string(),
                    ..Default::default()
                }],
            )]
            .into_iter()
            .collect(),
            abandoned: Default::default(),
        }));
        p.handle_key(KeyEvent::new(
            KeyCode::Down,
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(p.cursor, 0);
    }
    #[test]
    fn navigate_with_jk() {
        let mut p = AuditPanel::new();
        p.set_size(80, 40);
        p.set_audit(Some(&AuditResult {
            advisories: [(
                "pkg".to_string(),
                vec![
                    Advisory {
                        advisory_id: "ADV-001".to_string(),
                        title: "Bug1".to_string(),
                        ..Default::default()
                    },
                    Advisory {
                        advisory_id: "ADV-002".to_string(),
                        title: "Bug2".to_string(),
                        ..Default::default()
                    },
                ],
            )]
            .into_iter()
            .collect(),
            abandoned: Default::default(),
        }));
        p.handle_key(KeyEvent::new(
            KeyCode::Char('j'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(p.cursor, 1);
        p.handle_key(KeyEvent::new(
            KeyCode::Char('k'),
            crossterm::event::KeyModifiers::NONE,
        ));
        assert_eq!(p.cursor, 0);
    }
    #[test]
    fn view_no_issues() {
        let mut p = AuditPanel::new();
        p.set_size(80, 40);
        assert!(!p.view(true).is_empty());
    }
    #[test]
    fn view_with_advisories() {
        let mut p = AuditPanel::new();
        p.set_size(80, 40);
        p.set_audit(Some(&AuditResult {
            advisories: [(
                "vendor/pkg".to_string(),
                vec![Advisory {
                    advisory_id: "ADV-001".to_string(),
                    title: "Critical XSS".to_string(),
                    cve: Some("CVE-2024-0001".to_string()),
                    ..Default::default()
                }],
            )]
            .into_iter()
            .collect(),
            abandoned: Default::default(),
        }));
        assert!(!p.view(true).is_empty());
    }
    #[test]
    fn view_with_abandoned() {
        let mut p = AuditPanel::new();
        p.set_size(80, 40);
        p.set_audit(Some(&AuditResult {
            advisories: Default::default(),
            abandoned: [("old/pkg".to_string(), Some("new/pkg".to_string()))]
                .into_iter()
                .collect(),
        }));
        assert!(!p.view(true).is_empty());
    }
    #[test]
    fn set_size_test() {
        let mut p = AuditPanel::new();
        p.set_size(100, 50);
        assert_eq!(p.width, 100);
        assert_eq!(p.height, 50);
    }
    #[test]
    fn update_versions_direct_dep() {
        let mut p = AuditPanel::new();
        p.set_audit(Some(&AuditResult {
            advisories: [(
                "vendor/pkg".to_string(),
                vec![Advisory {
                    advisory_id: "ADV-001".to_string(),
                    title: "Bug".to_string(),
                    ..Default::default()
                }],
            )]
            .into_iter()
            .collect(),
            abandoned: Default::default(),
        }));
        let packages = vec![Package {
            name: "vendor/pkg".to_string(),
            version: "1.0.0".to_string(),
            ..Default::default()
        }];
        let outdated = vec![OutdatedPackage {
            name: "vendor/pkg".to_string(),
            version: "1.0.0".to_string(),
            latest: "2.0.0".to_string(),
            ..Default::default()
        }];
        p.update_versions(&packages, &outdated);
        match p.selected_entry() {
            Some(SelectedAuditEntry::Advisory {
                installed_version,
                latest_version,
                is_direct,
                ..
            }) => {
                assert_eq!(installed_version, "1.0.0");
                assert_eq!(latest_version, "2.0.0");
                assert!(is_direct);
            }
            _ => panic!("expected advisory entry"),
        }
    }
    #[test]
    fn update_versions_transitive_dep() {
        let mut p = AuditPanel::new();
        p.set_audit(Some(&AuditResult {
            advisories: [(
                "transitive/pkg".to_string(),
                vec![Advisory {
                    advisory_id: "ADV-001".to_string(),
                    title: "Bug".to_string(),
                    ..Default::default()
                }],
            )]
            .into_iter()
            .collect(),
            abandoned: Default::default(),
        }));
        let packages = vec![Package {
            name: "other/pkg".to_string(),
            version: "1.0.0".to_string(),
            ..Default::default()
        }];
        p.update_versions(&packages, &[]);
        match p.selected_entry() {
            Some(SelectedAuditEntry::Advisory { is_direct, .. }) => {
                assert!(!is_direct);
            }
            _ => panic!("expected advisory entry"),
        }
    }
    #[test]
    fn transitive_advisory_packages_list() {
        let mut p = AuditPanel::new();
        p.set_audit(Some(&AuditResult {
            advisories: [
                (
                    "direct/pkg".to_string(),
                    vec![Advisory {
                        advisory_id: "ADV-001".to_string(),
                        title: "Bug".to_string(),
                        ..Default::default()
                    }],
                ),
                (
                    "transitive/pkg".to_string(),
                    vec![Advisory {
                        advisory_id: "ADV-002".to_string(),
                        title: "Bug2".to_string(),
                        ..Default::default()
                    }],
                ),
            ]
            .into_iter()
            .collect(),
            abandoned: Default::default(),
        }));
        let packages = vec![Package {
            name: "direct/pkg".to_string(),
            version: "1.0.0".to_string(),
            ..Default::default()
        }];
        p.update_versions(&packages, &[]);
        let transitive = p.transitive_advisory_packages();
        assert_eq!(transitive.len(), 1);
        assert_eq!(transitive[0], "transitive/pkg");
    }
    #[test]
    fn set_why_result_updates_entries() {
        let mut p = AuditPanel::new();
        p.set_audit(Some(&AuditResult {
            advisories: [(
                "transitive/pkg".to_string(),
                vec![Advisory {
                    advisory_id: "ADV-001".to_string(),
                    title: "Bug".to_string(),
                    ..Default::default()
                }],
            )]
            .into_iter()
            .collect(),
            abandoned: Default::default(),
        }));
        p.update_versions(&[], &[]);
        p.set_why_result(
            "transitive/pkg",
            vec![WhyEntry {
                name: "parent/pkg".to_string(),
                version: "3.0.0".to_string(),
                constraint: "requires transitive/pkg (^1.0)".to_string(),
            }],
        );
        match p.selected_entry() {
            Some(SelectedAuditEntry::Advisory {
                is_direct,
                required_by,
                ..
            }) => {
                assert!(!is_direct);
                assert_eq!(required_by.len(), 1);
                assert_eq!(required_by[0].name, "parent/pkg");
            }
            _ => panic!("expected advisory entry"),
        }
    }
    #[test]
    fn view_with_transitive_shows_via() {
        let mut p = AuditPanel::new();
        p.set_size(80, 40);
        p.set_audit(Some(&AuditResult {
            advisories: [(
                "transitive/pkg".to_string(),
                vec![Advisory {
                    advisory_id: "ADV-001".to_string(),
                    title: "Critical XSS".to_string(),
                    cve: Some("CVE-2024-0001".to_string()),
                    ..Default::default()
                }],
            )]
            .into_iter()
            .collect(),
            abandoned: Default::default(),
        }));
        p.update_versions(&[], &[]);
        p.set_why_result(
            "transitive/pkg",
            vec![WhyEntry {
                name: "parent/pkg".to_string(),
                version: "3.0.0".to_string(),
                constraint: String::new(),
            }],
        );
        let view = p.view(true);
        assert!(view.contains("transitive/pkg"));
    }
}
