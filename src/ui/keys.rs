use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// KeyBinding holds a set of keys and their help text.
#[derive(Debug, Clone)]
pub struct KeyBinding {
    pub keys: Vec<KeyEvent>,
    pub help_key: String,
    pub help_desc: String,
}

impl KeyBinding {
    pub fn matches(&self, key: &KeyEvent) -> bool {
        self.keys.iter().any(|k| k == key)
    }
}

/// KeyMap contains all key bindings.
#[derive(Debug, Clone)]
pub struct KeyMap {
    pub quit: KeyBinding,
    pub help: KeyBinding,
    pub tab1: KeyBinding,
    pub tab2: KeyBinding,
    pub next_tab: KeyBinding,
    pub prev_tab: KeyBinding,
    pub up: KeyBinding,
    pub down: KeyBinding,
    pub enter: KeyBinding,
    pub search: KeyBinding,
    pub require: KeyBinding,
    pub remove: KeyBinding,
    pub update: KeyBinding,
    pub update_all: KeyBinding,
    pub escape: KeyBinding,
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn key_ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

fn key_shift(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::SHIFT)
}

/// Returns the default key bindings.
pub fn default_key_map() -> KeyMap {
    KeyMap {
        quit: KeyBinding {
            keys: vec![key(KeyCode::Char('q')), key_ctrl(KeyCode::Char('c'))],
            help_key: "q".to_string(),
            help_desc: "quit".to_string(),
        },
        help: KeyBinding {
            keys: vec![key(KeyCode::Char('?'))],
            help_key: "?".to_string(),
            help_desc: "help".to_string(),
        },
        tab1: KeyBinding {
            keys: vec![key(KeyCode::Char('1'))],
            help_key: "1".to_string(),
            help_desc: "packages".to_string(),
        },
        tab2: KeyBinding {
            keys: vec![key(KeyCode::Char('2'))],
            help_key: "2".to_string(),
            help_desc: "audit".to_string(),
        },
        next_tab: KeyBinding {
            keys: vec![key(KeyCode::Tab)],
            help_key: "tab".to_string(),
            help_desc: "next tab".to_string(),
        },
        prev_tab: KeyBinding {
            keys: vec![key(KeyCode::BackTab)],
            help_key: "shift+tab".to_string(),
            help_desc: "prev tab".to_string(),
        },
        up: KeyBinding {
            keys: vec![key(KeyCode::Up), key(KeyCode::Char('k'))],
            help_key: "k/↑".to_string(),
            help_desc: "up".to_string(),
        },
        down: KeyBinding {
            keys: vec![key(KeyCode::Down), key(KeyCode::Char('j'))],
            help_key: "j/↓".to_string(),
            help_desc: "down".to_string(),
        },
        enter: KeyBinding {
            keys: vec![key(KeyCode::Enter)],
            help_key: "enter".to_string(),
            help_desc: "select".to_string(),
        },
        search: KeyBinding {
            keys: vec![key(KeyCode::Char('/'))],
            help_key: "/".to_string(),
            help_desc: "search".to_string(),
        },
        require: KeyBinding {
            keys: vec![key(KeyCode::Char('r'))],
            help_key: "r".to_string(),
            help_desc: "require".to_string(),
        },
        remove: KeyBinding {
            keys: vec![key(KeyCode::Char('d'))],
            help_key: "d".to_string(),
            help_desc: "remove".to_string(),
        },
        update: KeyBinding {
            keys: vec![key(KeyCode::Char('u'))],
            help_key: "u".to_string(),
            help_desc: "update".to_string(),
        },
        update_all: KeyBinding {
            keys: vec![key_shift(KeyCode::Char('U'))],
            help_key: "U".to_string(),
            help_desc: "update all".to_string(),
        },
        escape: KeyBinding {
            keys: vec![key(KeyCode::Esc)],
            help_key: "esc".to_string(),
            help_desc: "cancel".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_key_map_all_bindings_set() {
        let km = default_key_map();

        let bindings: Vec<(&str, &KeyBinding)> = vec![
            ("Quit", &km.quit),
            ("Help", &km.help),
            ("Tab1", &km.tab1),
            ("Tab2", &km.tab2),
            ("NextTab", &km.next_tab),
            ("PrevTab", &km.prev_tab),
            ("Up", &km.up),
            ("Down", &km.down),
            ("Enter", &km.enter),
            ("Search", &km.search),
            ("Require", &km.require),
            ("Remove", &km.remove),
            ("Update", &km.update),
            ("UpdateAll", &km.update_all),
            ("Escape", &km.escape),
        ];

        for (name, binding) in bindings {
            assert!(
                !binding.keys.is_empty(),
                "binding {name} should have at least one key"
            );
        }
    }

    #[test]
    fn default_key_map_quit_keys() {
        let km = default_key_map();
        assert!(km.quit.matches(&key(KeyCode::Char('q'))));
        assert!(km.quit.matches(&key_ctrl(KeyCode::Char('c'))));
    }
}
