use crossterm::event::{KeyCode, KeyEvent};

/// Choice represents a single option in a ChoiceDialog.
#[derive(Debug, Clone)]
pub struct Choice {
    pub key: char,
    pub label: String,
}

/// ChoiceDialog shows a dialog with multiple keyed options.
pub struct ChoiceDialog {
    message: String,
    choices: Vec<Choice>,
    visible: bool,
    selected_key: Option<char>,
}

impl Default for ChoiceDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl ChoiceDialog {
    pub fn new() -> Self {
        ChoiceDialog {
            message: String::new(),
            choices: vec![],
            visible: false,
            selected_key: None,
        }
    }

    pub fn show(&mut self, message: &str, choices: Vec<Choice>) {
        self.message = message.to_string();
        self.choices = choices;
        self.visible = true;
        self.selected_key = None;
    }

    pub fn hide(&mut self) {
        self.visible = false;
        self.choices.clear();
        self.selected_key = None;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn selected_key(&self) -> Option<char> {
        self.selected_key
    }

    /// Handles key input. Returns Some(char) when a choice is selected, None otherwise.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<char> {
        if !self.visible {
            return None;
        }

        match key.code {
            KeyCode::Esc => {
                self.visible = false;
                self.selected_key = None;
                Some('\x1b') // ESC sentinel
            }
            KeyCode::Char(c) => {
                for ch in &self.choices {
                    if c == ch.key {
                        self.visible = false;
                        self.selected_key = Some(c);
                        return Some(c);
                    }
                }
                None
            }
            _ => None,
        }
    }

    pub fn view(&self) -> String {
        if !self.visible {
            return String::new();
        }

        let mut b = String::new();
        b.push_str(&self.message);
        b.push('\n');

        for ch in &self.choices {
            b.push_str(&format!("\n[{}] {}", ch.key, ch.label));
        }

        b.push_str("\n\n[esc] cancel");
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn sample_choices() -> Vec<Choice> {
        vec![
            Choice {
                key: 'u',
                label: "Update within constraints".to_string(),
            },
            Choice {
                key: 'U',
                label: "Upgrade to major".to_string(),
            },
        ]
    }

    #[test]
    fn show_hide() {
        let mut d = ChoiceDialog::new();
        assert!(!d.is_visible());

        d.show("Update pkg?", sample_choices());
        assert!(d.is_visible());

        d.hide();
        assert!(!d.is_visible());
    }

    #[test]
    fn select_choice() {
        let mut d = ChoiceDialog::new();
        d.show("Update pkg?", sample_choices());
        let result = d.handle_key(key(KeyCode::Char('u')));
        assert_eq!(result, Some('u'));
        assert!(!d.is_visible());
        assert_eq!(d.selected_key(), Some('u'));
    }

    #[test]
    fn cancel_with_esc() {
        let mut d = ChoiceDialog::new();
        d.show("Update pkg?", sample_choices());
        let result = d.handle_key(key(KeyCode::Esc));
        assert_eq!(result, Some('\x1b'));
        assert!(!d.is_visible());
        assert_eq!(d.selected_key(), None);
    }

    #[test]
    fn unknown_key_ignored() {
        let mut d = ChoiceDialog::new();
        d.show("Update pkg?", sample_choices());
        let result = d.handle_key(key(KeyCode::Char('x')));
        assert!(result.is_none());
        assert!(d.is_visible());
    }

    #[test]
    fn handle_key_when_hidden() {
        let mut d = ChoiceDialog::new();
        let result = d.handle_key(key(KeyCode::Char('u')));
        assert!(result.is_none());
    }

    #[test]
    fn view_hidden_empty() {
        let d = ChoiceDialog::new();
        assert!(d.view().is_empty());
    }

    #[test]
    fn view_visible_contains_choices() {
        let mut d = ChoiceDialog::new();
        d.show("Update pkg?", sample_choices());
        let view = d.view();
        assert!(view.contains("Update pkg?"));
        assert!(view.contains("[u]"));
        assert!(view.contains("[U]"));
        assert!(view.contains("[esc] cancel"));
    }
}
