use crossterm::event::{KeyCode, KeyEvent};

use crate::ui::messages::Action;

/// ConfirmDialog shows a yes/no confirmation prompt.
pub struct ConfirmDialog {
    message: String,
    visible: bool,
    pub confirmed: bool,
}

impl Default for ConfirmDialog {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfirmDialog {
    pub fn new() -> Self {
        ConfirmDialog {
            message: String::new(),
            visible: false,
            confirmed: false,
        }
    }

    pub fn show(&mut self, message: &str) {
        self.message = message.to_string();
        self.visible = true;
        self.confirmed = false;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    /// Handles key input. Returns Some(Action) when the dialog resolves.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<Action> {
        if !self.visible {
            return None;
        }

        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                self.visible = false;
                self.confirmed = true;
                Some(Action::None) // caller checks confirmed flag
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.visible = false;
                self.confirmed = false;
                Some(Action::None)
            }
            _ => None,
        }
    }

    pub fn view(&self) -> String {
        if !self.visible {
            return String::new();
        }
        format!("{}\n\n[y]es / [n]o", self.message)
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
        let mut d = ConfirmDialog::new();
        assert!(!d.is_visible());

        d.show("Delete?");
        assert!(d.is_visible());

        d.hide();
        assert!(!d.is_visible());
    }

    #[test]
    fn confirm_with_y() {
        let mut d = ConfirmDialog::new();
        d.show("Delete?");
        d.handle_key(key(KeyCode::Char('y')));
        assert!(d.confirmed);
        assert!(!d.is_visible());
    }

    #[test]
    fn confirm_with_enter() {
        let mut d = ConfirmDialog::new();
        d.show("Delete?");
        d.handle_key(key(KeyCode::Enter));
        assert!(d.confirmed);
    }

    #[test]
    fn cancel_with_n() {
        let mut d = ConfirmDialog::new();
        d.show("Delete?");
        d.handle_key(key(KeyCode::Char('n')));
        assert!(!d.confirmed);
        assert!(!d.is_visible());
    }

    #[test]
    fn cancel_with_esc() {
        let mut d = ConfirmDialog::new();
        d.show("Delete?");
        d.handle_key(key(KeyCode::Esc));
        assert!(!d.is_visible());
    }

    #[test]
    fn handle_key_when_hidden() {
        let mut d = ConfirmDialog::new();
        let result = d.handle_key(key(KeyCode::Char('y')));
        assert!(result.is_none());
    }

    #[test]
    fn view_hidden_empty() {
        let d = ConfirmDialog::new();
        assert!(d.view().is_empty());
    }

    #[test]
    fn view_visible_not_empty() {
        let mut d = ConfirmDialog::new();
        d.show("Delete file?");
        assert!(!d.view().is_empty());
    }
}
