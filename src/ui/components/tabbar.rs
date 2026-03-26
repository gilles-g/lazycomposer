use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};

use crate::ui::style::theme;

const TAB_LABELS: &[&str] = &["Packages", "Audit", "Project"];

/// Height of the tab bar.
pub const TAB_BAR_H: u16 = 1;

/// Renders the tab bar at the given area.
pub fn render_tab_bar(active: usize, area: Rect, buf: &mut Buffer) {
    let bg = Style::default().fg(theme::COLOR_MUTED);
    // Fill background
    for x in area.x..area.x + area.width {
        buf.set_string(x, area.y, " ", bg);
    }

    let mut x = area.x + 1;
    for (i, label) in TAB_LABELS.iter().enumerate() {
        let is_active = i == active;
        if is_active {
            let style = Style::default()
                .fg(theme::COLOR_PRIMARY)
                .add_modifier(Modifier::BOLD);
            buf.set_string(x, area.y, "[", style);
            x += 1;
            buf.set_string(x, area.y, label, style);
            x += label.len() as u16;
            buf.set_string(x, area.y, "]", style);
            x += 1;
        } else {
            let style = Style::default().fg(theme::COLOR_MUTED);
            buf.set_string(x, area.y, " ", style);
            x += 1;
            buf.set_string(x, area.y, label, style);
            x += label.len() as u16;
            buf.set_string(x, area.y, " ", style);
            x += 1;
        }
        x += 1; // spacing
    }
}

/// Returns the tab index at the given column position, or None if outside any tab.
pub fn tab_index_at(col: u16) -> Option<usize> {
    let mut x: u16 = 1; // starts at area.x(0) + 1
    for (i, label) in TAB_LABELS.iter().enumerate() {
        let tab_width = 1 + label.len() as u16 + 1; // bracket/space + label + bracket/space
        if col >= x && col < x + tab_width {
            return Some(i);
        }
        x += tab_width + 1; // +1 for spacing
    }
    None
}

/// String-based tab bar view (for tests).
pub fn tab_bar_view(active: usize) -> String {
    TAB_LABELS
        .iter()
        .enumerate()
        .map(|(i, label)| {
            if i == active {
                format!("[{label}]")
            } else {
                format!(" {label} ")
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tab_bar_view_packages() {
        let view = tab_bar_view(0);
        assert!(view.contains("[Packages]"));
        assert!(view.contains(" Audit "));
        assert!(view.contains(" Project "));
    }

    #[test]
    fn tab_bar_view_audit() {
        let view = tab_bar_view(1);
        assert!(view.contains(" Packages "));
        assert!(view.contains("[Audit]"));
    }

    #[test]
    fn tab_bar_view_project() {
        let view = tab_bar_view(2);
        assert!(view.contains("[Project]"));
    }
}
