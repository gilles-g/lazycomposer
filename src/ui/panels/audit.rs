use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::composer::{Advisory, AuditResult};
use crate::ui::style::{styles, theme};

struct AdvisoryEntry {
    pkg: String,
    advisory: Advisory,
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
                lines.push(Line::from(vec![
                    cursor,
                    Span::styled(cve.to_string(), styles::error_style()),
                    Span::raw(" "),
                    Span::raw(&entry.pkg),
                ]));
                lines.push(Line::from(vec![
                    Span::raw("    "),
                    Span::raw(&entry.advisory.title),
                ]));
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
                b.push_str(&format!("{cursor}{cve} {}\n", entry.pkg));
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
}
