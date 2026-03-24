use crossterm::event::{KeyCode, KeyEvent};

use crate::ui::messages::Action;

/// InputBox is a text input overlay.
pub struct InputBox {
    value: String,
    label: String,
    placeholder: String,
    visible: bool,
}

impl Default for InputBox {
    fn default() -> Self {
        Self::new()
    }
}

impl InputBox {
    pub fn new() -> Self {
        InputBox {
            value: String::new(),
            label: String::new(),
            placeholder: "vendor/package".to_string(),
            visible: false,
        }
    }

    pub fn show(&mut self, label: &str, placeholder: &str) {
        self.label = label.to_string();
        self.placeholder = placeholder.to_string();
        self.value.clear();
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    /// Handles key input. Returns Some(Action) when the input resolves.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if !self.visible {
            return None;
        }

        match key.code {
            KeyCode::Enter => {
                let value = self.value.clone();
                self.hide();
                Some(Action::InputSubmit(value))
            }
            KeyCode::Esc => {
                self.hide();
                Some(Action::InputCancel)
            }
            KeyCode::Char(c) => {
                self.value.push(c);
                None
            }
            KeyCode::Backspace => {
                self.value.pop();
                None
            }
            _ => None,
        }
    }

    pub fn view(&self) -> String {
        if !self.visible {
            return String::new();
        }
        format!("{}\n\n{}", self.label, self.value)
    }
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
        let mut b = InputBox::new();
        assert!(!b.is_visible());

        b.show("Require", "vendor/package");
        assert!(b.is_visible());

        b.hide();
        assert!(!b.is_visible());
    }

    #[test]
    fn submit_returns_action() {
        let mut b = InputBox::new();
        b.show("Require", "vendor/package");
        b.handle_key(key(KeyCode::Char('a')));
        let result = b.handle_key(key(KeyCode::Enter));
        match result {
            Some(Action::InputSubmit(v)) => assert_eq!(v, "a"),
            other => panic!("expected InputSubmit, got {other:?}"),
        }
        assert!(!b.is_visible());
    }

    #[test]
    fn cancel_returns_action() {
        let mut b = InputBox::new();
        b.show("Require", "vendor/package");
        let result = b.handle_key(key(KeyCode::Esc));
        match result {
            Some(Action::InputCancel) => {}
            other => panic!("expected InputCancel, got {other:?}"),
        }
        assert!(!b.is_visible());
    }

    #[test]
    fn handle_key_when_hidden() {
        let mut b = InputBox::new();
        let result = b.handle_key(key(KeyCode::Enter));
        assert!(result.is_none());
    }

    #[test]
    fn view_hidden_empty() {
        let b = InputBox::new();
        assert!(b.view().is_empty());

        let mut b2 = InputBox::new();
        b2.show("Require Package", "vendor/pkg");
        assert!(!b2.view().is_empty());
    }
}
