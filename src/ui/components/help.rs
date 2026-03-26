use crossterm::event::{KeyCode, KeyEvent};

use super::statusbar::Hint;

/// HelpSection groups related keybindings.
pub struct HelpSection {
    pub title: String,
    pub bindings: Vec<Hint>,
}

/// HelpPopup displays a modal listing all keyboard shortcuts.
pub struct HelpPopup {
    visible: bool,
}

impl Default for HelpPopup {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpPopup {
    pub fn new() -> Self {
        HelpPopup { visible: false }
    }

    pub fn show(&mut self) {
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Handles key input. ?, esc, q close the popup.
    pub fn handle_key(&mut self, key: KeyEvent) {
        if !self.visible {
            return;
        }

        match key.code {
            KeyCode::Char('?') | KeyCode::Esc | KeyCode::Char('q') => {
                self.visible = false;
            }
            _ => {}
        }
    }

    pub fn view(&self) -> String {
        if !self.visible {
            return String::new();
        }

        let sections = help_sections();
        let mut b = String::new();

        b.push_str("Keyboard Shortcuts\n\n");

        for (i, section) in sections.iter().enumerate() {
            if i > 0 {
                b.push('\n');
            }
            b.push_str(&section.title);
            b.push('\n');
            for binding in &section.bindings {
                b.push_str(&format!("  {:12} {}\n", binding.key, binding.desc));
            }
        }

        b.push_str("\nPress ? or esc to close");
        b
    }
}

fn help_sections() -> Vec<HelpSection> {
    vec![
        HelpSection {
            title: "Navigation".to_string(),
            bindings: vec![
                Hint {
                    key: "j/k / \u{2191}\u{2193}".to_string(),
                    desc: "navigate up/down".to_string(),
                },
                Hint {
                    key: "tab".to_string(),
                    desc: "next tab".to_string(),
                },
                Hint {
                    key: "shift+tab".to_string(),
                    desc: "previous tab / toggle focus".to_string(),
                },
            ],
        },
        HelpSection {
            title: "Packages".to_string(),
            bindings: vec![
                Hint {
                    key: "r".to_string(),
                    desc: "require a package".to_string(),
                },
                Hint {
                    key: "d".to_string(),
                    desc: "remove selected package".to_string(),
                },
                Hint {
                    key: "u".to_string(),
                    desc: "update selected package".to_string(),
                },
                Hint {
                    key: "s".to_string(),
                    desc: "show package details".to_string(),
                },
                Hint {
                    key: "/".to_string(),
                    desc: "search / filter".to_string(),
                },
            ],
        },
        HelpSection {
            title: "General".to_string(),
            bindings: vec![
                Hint {
                    key: "U".to_string(),
                    desc: "update all packages".to_string(),
                },
                Hint {
                    key: "?".to_string(),
                    desc: "show this help".to_string(),
                },
                Hint {
                    key: "q / ctrl+c".to_string(),
                    desc: "quit".to_string(),
                },
            ],
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn show_hide() {
        let mut h = HelpPopup::new();
        assert!(!h.is_visible());

        h.show();
        assert!(h.is_visible());

        h.hide();
        assert!(!h.is_visible());
    }

    #[test]
    fn close_with_esc() {
        let mut h = HelpPopup::new();
        h.show();
        h.handle_key(key(KeyCode::Esc));
        assert!(!h.is_visible());
    }

    #[test]
    fn close_with_question_mark() {
        let mut h = HelpPopup::new();
        h.show();
        h.handle_key(key(KeyCode::Char('?')));
        assert!(!h.is_visible());
    }

    #[test]
    fn close_with_q() {
        let mut h = HelpPopup::new();
        h.show();
        h.handle_key(key(KeyCode::Char('q')));
        assert!(!h.is_visible());
    }

    #[test]
    fn other_keys_ignored() {
        let mut h = HelpPopup::new();
        h.show();
        h.handle_key(key(KeyCode::Char('x')));
        assert!(h.is_visible());
    }

    #[test]
    fn view_hidden_empty() {
        let h = HelpPopup::new();
        assert!(h.view().is_empty());
    }

    #[test]
    fn view_visible_has_sections() {
        let mut h = HelpPopup::new();
        h.show();
        let view = h.view();
        assert!(view.contains("Keyboard Shortcuts"));
        assert!(view.contains("Navigation"));
        assert!(view.contains("Packages"));
        assert!(!view.contains("Outdated"));
        assert!(view.contains("General"));
        assert!(view.contains("show package details"));
        assert!(view.contains("Press ? or esc to close"));
    }

    #[test]
    fn help_sections_has_four() {
        assert_eq!(help_sections().len(), 3);
    }
}
