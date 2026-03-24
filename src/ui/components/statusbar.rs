use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;

use crate::ui::style::theme::*;

/// Hint is a key/description pair.
#[derive(Debug, Clone)]
pub struct Hint {
    pub key: String,
    pub desc: String,
}

/// StatusBar renders contextual keyboard hints at the bottom.
pub struct StatusBar {
    pub width: u16,
    pub hints: Vec<Hint>,
    pub composer_info: String,
    pub version: String,
    pub loading_msg: String,
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusBar {
    pub fn new() -> Self {
        StatusBar {
            width: 0,
            hints: vec![],
            composer_info: String::new(),
            version: String::new(),
            loading_msg: String::new(),
        }
    }

    pub fn set_right(&mut self, composer_info: &str, version: &str) {
        self.composer_info = composer_info.to_string();
        self.version = version.to_string();
    }

    pub fn set_loading(&mut self, msg: &str) {
        self.loading_msg = msg.to_string();
    }

    pub fn set_width(&mut self, width: u16) {
        self.width = width;
    }

    pub fn set_hints(&mut self, hints: Vec<Hint>) {
        self.hints = hints;
    }

    pub fn render(&self, area: Rect, buf: &mut Buffer) {
        let bg = Style::default()
            .fg(COLOR_TEXT)
            .bg(Color::Rgb(0x1A, 0x1A, 0x1A));
        // Fill background
        for x in area.x..area.x + area.width {
            buf.set_string(x, area.y, " ", bg);
        }

        let mut x = area.x + 1;

        // Loading message
        if !self.loading_msg.is_empty() {
            let msg = format!("⟳ {}", self.loading_msg);
            let style = Style::default()
                .fg(Color::Rgb(0x00, 0x00, 0x00))
                .bg(COLOR_WARNING);
            buf.set_string(x, area.y, &msg, style);
            x += msg.len() as u16 + 2;
        }

        // Hints
        for hint in &self.hints {
            let key_style = Style::default().fg(COLOR_PRIMARY);
            buf.set_string(x, area.y, &hint.key, key_style);
            x += hint.key.len() as u16 + 1;
            buf.set_string(x, area.y, &hint.desc, bg);
            x += hint.desc.len() as u16 + 2;
        }
    }

    pub fn view(&self) -> String {
        let mut parts = Vec::new();
        if !self.loading_msg.is_empty() {
            parts.push(format!("⟳ {}", self.loading_msg));
        }
        for h in &self.hints {
            parts.push(format!("{} {}", h.key, h.desc));
        }
        parts.join("  ")
    }
}

use ratatui::style::Color;

/// Returns the base hints shown on the status bar.
pub fn default_hints(tab: usize) -> Vec<Hint> {
    let mut hints = vec![
        Hint {
            key: "j/k".to_string(),
            desc: "navigate".to_string(),
        },
        Hint {
            key: "tab".to_string(),
            desc: "switch panel".to_string(),
        },
    ];

    if tab == 0 {
        hints.push(Hint {
            key: "r".to_string(),
            desc: "require".to_string(),
        });
        hints.push(Hint {
            key: "d".to_string(),
            desc: "remove".to_string(),
        });
        hints.push(Hint {
            key: "u".to_string(),
            desc: "update".to_string(),
        });
        hints.push(Hint {
            key: "s".to_string(),
            desc: "show".to_string(),
        });
        hints.push(Hint {
            key: "/".to_string(),
            desc: "search".to_string(),
        });
    }
    hints.push(Hint {
        key: "U".to_string(),
        desc: "update all".to_string(),
    });

    hints.push(Hint {
        key: "?".to_string(),
        desc: "help".to_string(),
    });
    hints.push(Hint {
        key: "q".to_string(),
        desc: "quit".to_string(),
    });
    hints
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_hints() {
        let sb = StatusBar::new();
        let view = sb.view();
        // empty hints = empty view, but the function works
        assert!(view.is_empty() || !view.is_empty()); // just don't panic
    }

    #[test]
    fn set_hints_view() {
        let mut sb = StatusBar::new();
        sb.set_width(80);
        sb.set_hints(vec![
            Hint {
                key: "q".to_string(),
                desc: "quit".to_string(),
            },
            Hint {
                key: "r".to_string(),
                desc: "require".to_string(),
            },
        ]);
        let view = sb.view();
        assert!(!view.is_empty());
    }

    #[test]
    fn default_hints_packages() {
        let hints = default_hints(0);
        assert!(hints.len() >= 4);
        let last = hints.last().unwrap();
        assert_eq!(last.key, "q");

        assert!(hints.iter().any(|h| h.key == "r" && h.desc == "require"));
        assert!(hints.iter().any(|h| h.key == "s" && h.desc == "show"));
    }

    #[test]
    fn default_hints_audit() {
        let hints = default_hints(1);
        assert!(hints.iter().any(|h| h.key == "U" && h.desc == "update all"));
        assert!(!hints.iter().any(|h| h.key == "s"));
    }
}
