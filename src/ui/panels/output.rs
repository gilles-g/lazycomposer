use ansi_to_tui::IntoText;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget, Wrap};

use crate::ui::style::{styles, theme};

/// OutputPanel streams command output with auto-scroll.
pub struct OutputPanel {
    pub lines: Vec<String>,
    title: String,
    pub width: u16,
    pub height: u16,
    visible: bool,
    scroll: usize,
}

impl Default for OutputPanel {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputPanel {
    pub fn new() -> Self {
        OutputPanel {
            lines: vec![],
            title: String::new(),
            width: 80,
            height: 20,
            visible: false,
            scroll: 0,
        }
    }

    pub fn show(&mut self, title: &str) {
        self.visible = true;
        self.title = title.to_string();
        self.lines.clear();
        self.scroll = 0;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn append_line(&mut self, line: &str) {
        self.lines.push(line.to_string());
        let visible_lines = self.height.saturating_sub(4) as usize;
        if self.lines.len() > visible_lines {
            self.scroll = self.lines.len() - visible_lines;
        }
    }

    pub fn set_size(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
    }

    /// Render the output panel with colors.
    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        if !self.visible {
            return;
        }

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme::COLOR_BORDER_FOCUS))
            .title(Span::styled(
                format!(" {} ", self.title),
                styles::title_style(),
            ));
        let inner = block.inner(area);
        block.render(area, buf);

        let full_output = self.lines.join("\n");
        let styled_lines: Vec<Line> = match full_output.as_bytes().into_text() {
            Ok(text) => text.lines,
            Err(_) => self
                .lines
                .iter()
                .map(|line| Line::from(Span::styled(line.as_str(), styles::output_style())))
                .collect(),
        };

        let paragraph = Paragraph::new(styled_lines)
            .scroll((self.scroll as u16, 0))
            .wrap(Wrap { trim: false });
        paragraph.render(inner, buf);
    }

    pub fn view(&self) -> String {
        if !self.visible {
            return String::new();
        }
        let mut b = format!(" {} \n", self.title);
        for line in &self.lines {
            b.push_str(line);
            b.push('\n');
        }
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn show_hide() {
        let mut p = OutputPanel::new();
        assert!(!p.is_visible());
        p.show("composer update");
        assert!(p.is_visible());
        p.hide();
        assert!(!p.is_visible());
    }
    #[test]
    fn append_line() {
        let mut p = OutputPanel::new();
        p.set_size(80, 40);
        p.show("test");
        p.append_line("line 1");
        p.append_line("line 2");
        assert_eq!(p.lines.len(), 2);
    }
    #[test]
    fn show_resets_content() {
        let mut p = OutputPanel::new();
        p.set_size(80, 40);
        p.show("first command");
        p.append_line("old output");
        p.show("second command");
        assert_eq!(p.lines.len(), 0);
    }
    #[test]
    fn view_hidden() {
        let p = OutputPanel::new();
        assert!(p.view().is_empty());
    }
    #[test]
    fn view_visible() {
        let mut p = OutputPanel::new();
        p.set_size(80, 40);
        p.show("composer require vendor/pkg");
        p.append_line("Loading...");
        assert!(!p.view().is_empty());
    }
    #[test]
    fn set_size_test() {
        let mut p = OutputPanel::new();
        p.set_size(100, 50);
        assert_eq!(p.width, 100);
        assert_eq!(p.height, 50);
    }
}
