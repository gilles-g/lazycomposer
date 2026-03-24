const STATUS_BAR_HEIGHT: u16 = 1;
const LEFT_RATIO: f64 = 0.50;
/// Height of a collapsed (inactive) panel: top border + bottom border + 1 line of content.
pub const COLLAPSED_PANEL_H: u16 = 3;

#[derive(Debug, Clone, Default)]
pub struct Layout {
    pub width: u16,
    pub height: u16,
    pub left_width: u16,
    pub right_width: u16,
    pub content_h: u16,
    pub status_bar_h: u16,
}

/// Calculates panel sizes from terminal dimensions.
pub fn compute_layout(width: u16, height: u16) -> Layout {
    let content_h = height.saturating_sub(STATUS_BAR_HEIGHT);

    let mut left_w = (width as f64 * LEFT_RATIO) as u16;
    if left_w < 20 {
        left_w = 20.min(width);
    }
    let right_w = width.saturating_sub(left_w);

    Layout {
        width,
        height,
        left_width: left_w,
        right_width: right_w,
        content_h,
        status_bar_h: STATUS_BAR_HEIGHT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_terminal() {
        let l = compute_layout(120, 40);
        assert_eq!(l.width, 120);
        assert_eq!(l.height, 40);
        assert_eq!(l.content_h, 39);
        assert_eq!(l.left_width, 60);
        assert_eq!(l.right_width, 60);
        assert_eq!(l.left_width + l.right_width, l.width);
    }

    #[test]
    fn narrow_terminal() {
        let l = compute_layout(40, 20);
        assert_eq!(l.left_width, 20);
        assert_eq!(l.right_width, 20);
    }

    #[test]
    fn content_height_calculation() {
        let tests = vec![
            ("standard", 50u16, 49u16),
            ("small", 10, 9),
            ("minimal", 3, 2),
        ];

        for (name, height, want_ch) in tests {
            let l = compute_layout(100, height);
            assert_eq!(l.content_h, want_ch, "case={name}");
        }
    }
}
