const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// LoadingSpinner displays an animated spinner with a label.
pub struct LoadingSpinner {
    active: bool,
    label: String,
    frame: usize,
}

impl Default for LoadingSpinner {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadingSpinner {
    pub fn new() -> Self {
        LoadingSpinner {
            active: false,
            label: String::new(),
            frame: 0,
        }
    }

    pub fn start(&mut self, label: &str) {
        self.active = true;
        self.label = label.to_string();
        self.frame = 0;
    }

    pub fn stop(&mut self) {
        self.active = false;
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn tick(&mut self) {
        if self.active {
            self.frame = (self.frame + 1) % SPINNER_FRAMES.len();
        }
    }

    pub fn view(&self) -> String {
        if !self.active {
            return String::new();
        }
        format!("{} {}", SPINNER_FRAMES[self.frame], self.label)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn start_stop() {
        let mut s = LoadingSpinner::new();
        assert!(!s.is_active());

        s.start("Loading...");
        assert!(s.is_active());

        s.stop();
        assert!(!s.is_active());
    }

    #[test]
    fn view_when_inactive() {
        let s = LoadingSpinner::new();
        assert!(s.view().is_empty());
    }

    #[test]
    fn tick_when_inactive() {
        let mut s = LoadingSpinner::new();
        s.tick(); // should not panic
        assert!(!s.is_active());
    }
}
