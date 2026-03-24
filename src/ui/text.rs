use ratatui::style::Style;
use ratatui::text::{Line, Span};

use super::style::styles;

/// Render a labeled field with automatic word-wrapping.
/// The first line shows "Label:  value...", continuation lines are indented
/// to align with the value column.
pub fn wrap_field<'a>(
    label: &'a str,
    value: &'a str,
    value_style: Style,
    width: u16,
) -> Vec<Line<'a>> {
    let label_width = label.len() + 2; // label + "  "
    let available = (width as usize).saturating_sub(label_width);

    if available == 0 || value.len() <= available {
        return vec![Line::from(vec![
            Span::styled(label, styles::key_style()),
            Span::raw("  "),
            Span::styled(value, value_style),
        ])];
    }

    let mut lines = Vec::new();
    let mut remaining = value;
    let mut first = true;

    while !remaining.is_empty() {
        let chunk_end = if remaining.len() <= available {
            remaining.len()
        } else {
            remaining[..available]
                .rfind(' ')
                .map_or(available, |pos| pos)
        };

        let chunk = &remaining[..chunk_end];
        remaining = if chunk_end < remaining.len() {
            remaining[chunk_end..].trim_start()
        } else {
            ""
        };

        if first {
            lines.push(Line::from(vec![
                Span::styled(label, styles::key_style()),
                Span::raw("  "),
                Span::styled(chunk, value_style),
            ]));
            first = false;
        } else {
            lines.push(Line::from(vec![
                Span::raw(" ".repeat(label_width)),
                Span::styled(chunk, value_style),
            ]));
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_value_no_wrap() {
        let lines = wrap_field("Label:", "short", Style::default(), 80);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn long_value_wraps() {
        let lines = wrap_field(
            "Desc:",
            "This is a very long description that should wrap onto multiple lines",
            Style::default(),
            30,
        );
        assert!(lines.len() > 1);
    }

    #[test]
    fn zero_width_no_panic() {
        let lines = wrap_field("Label:", "value", Style::default(), 0);
        assert_eq!(lines.len(), 1);
    }

    #[test]
    fn empty_value() {
        let lines = wrap_field("Label:", "", Style::default(), 80);
        assert_eq!(lines.len(), 1);
    }
}
