const STATUS_BAR_HEIGHT: u16 = 1;
pub const DEFAULT_LEFT_RATIO: f64 = 0.50;
pub const DEFAULT_LEFT_PANEL_RATIO: f64 = 0.70;
const MIN_RATIO: f64 = 0.20;
const MAX_RATIO: f64 = 0.80;
const RATIO_STEP: f64 = 0.02;
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
pub fn compute_layout(width: u16, height: u16, left_ratio: f64) -> Layout {
    let content_h = height.saturating_sub(STATUS_BAR_HEIGHT);

    let mut left_w = (width as f64 * left_ratio) as u16;
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

/// Clamps and adjusts a ratio by the standard step.
pub fn adjust_ratio(current: f64, increase: bool) -> f64 {
    let new = if increase {
        current + RATIO_STEP
    } else {
        current - RATIO_STEP
    };
    new.clamp(MIN_RATIO, MAX_RATIO)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normal_terminal() {
        let l = compute_layout(120, 40, DEFAULT_LEFT_RATIO);
        assert_eq!(l.width, 120);
        assert_eq!(l.height, 40);
        assert_eq!(l.content_h, 39);
        assert_eq!(l.left_width, 60);
        assert_eq!(l.right_width, 60);
        assert_eq!(l.left_width + l.right_width, l.width);
    }

    #[test]
    fn narrow_terminal() {
        let l = compute_layout(40, 20, DEFAULT_LEFT_RATIO);
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
            let l = compute_layout(100, height, DEFAULT_LEFT_RATIO);
            assert_eq!(l.content_h, want_ch, "case={name}");
        }
    }

    #[test]
    fn custom_left_ratio() {
        let l = compute_layout(100, 40, 0.30);
        assert_eq!(l.left_width, 30);
        assert_eq!(l.right_width, 70);
    }

    #[test]
    fn adjust_ratio_increase() {
        let r = adjust_ratio(0.50, true);
        assert!((r - 0.52).abs() < 0.001);
    }

    #[test]
    fn adjust_ratio_decrease() {
        let r = adjust_ratio(0.50, false);
        assert!((r - 0.48).abs() < 0.001);
    }

    #[test]
    fn adjust_ratio_clamps() {
        assert!((adjust_ratio(0.20, false) - MIN_RATIO).abs() < 0.001);
        assert!((adjust_ratio(0.80, true) - MAX_RATIO).abs() < 0.001);
    }
}
