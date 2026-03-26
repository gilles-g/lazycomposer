use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Widget};

use crate::composer::{ComposerJSON, FrameworkInfo};
use crate::ui::style::{styles, theme};

/// Renders the project info panel showing composer.json metadata.
pub fn render_project_panel(
    cj: &ComposerJSON,
    framework: Option<&FrameworkInfo>,
    area: Rect,
    buf: &mut Buffer,
    _focused: bool,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::COLOR_BORDER))
        .title(Span::styled(" Project ", styles::title_style()));
    let inner = block.inner(area);
    block.render(area, buf);

    let mut lines: Vec<Line> = Vec::new();

    // Name + license
    if !cj.name.is_empty() {
        let mut spans = vec![Span::styled(&cj.name, styles::title_style())];
        if !cj.license.is_empty() {
            spans.push(Span::raw("  "));
            spans.push(Span::styled(cj.license.join(", "), styles::muted_style()));
        }
        lines.push(Line::from(spans));
    }

    // Description
    if !cj.description.is_empty() {
        lines.push(Line::from(Span::styled(
            &cj.description,
            Style::default().fg(theme::COLOR_TEXT),
        )));
    }

    lines.push(Line::default());

    // Framework
    if let Some(FrameworkInfo::Symfony(sf)) = framework {
        lines.push(Line::from(vec![
            Span::styled("Framework:", styles::key_style()),
            Span::raw("  "),
            Span::styled("Symfony", Style::default().fg(theme::COLOR_PRIMARY)),
            if !sf.require.is_empty() {
                Span::styled(format!(" {}", sf.require), styles::muted_style())
            } else {
                Span::raw("")
            },
        ]));
    }

    // Stability
    if !cj.minimum_stability.is_empty() || cj.prefer_stable {
        let mut spans = vec![
            Span::styled("Stability:", styles::key_style()),
            Span::raw("  "),
        ];
        if !cj.minimum_stability.is_empty() {
            spans.push(Span::styled(
                &cj.minimum_stability,
                Style::default().fg(theme::COLOR_TEXT),
            ));
        } else {
            spans.push(Span::styled("stable", styles::muted_style()));
        }
        if cj.prefer_stable {
            spans.push(Span::styled(" (prefer-stable)", styles::muted_style()));
        }
        lines.push(Line::from(spans));
    }

    // Homepage
    if !cj.homepage.is_empty() {
        lines.push(styled_field(
            "Homepage:",
            &cj.homepage,
            Style::default().fg(theme::COLOR_INFO),
        ));
    }

    // Authors
    if !cj.authors.is_empty() {
        let names: Vec<&str> = cj.authors.iter().map(|a| a.name.as_str()).collect();
        let authors_str = if names.len() <= 3 {
            names.join(", ")
        } else {
            format!("{} +{}", names[..2].join(", "), names.len() - 2)
        };
        lines.push(styled_field(
            "Authors:",
            &authors_str,
            Style::default().fg(theme::COLOR_TEXT),
        ));
    }

    // Support
    if let Some(support) = &cj.support {
        let mut support_parts = Vec::new();
        if !support.issues.is_empty() {
            support_parts.push("issues");
        }
        if !support.docs.is_empty() {
            support_parts.push("docs");
        }
        if !support.source.is_empty() {
            support_parts.push("source");
        }
        if !support.email.is_empty() {
            support_parts.push("email");
        }
        if !support.forum.is_empty() {
            support_parts.push("forum");
        }
        if !support.wiki.is_empty() {
            support_parts.push("wiki");
        }
        if !support.chat.is_empty() {
            support_parts.push("chat");
        }
        if !support_parts.is_empty() {
            lines.push(styled_field(
                "Support:",
                &support_parts.join(", "),
                Style::default().fg(theme::COLOR_TEXT),
            ));
        }
    }

    lines.push(Line::default());

    // Dependency relations
    let relations = [
        ("Replace", cj.replace.len()),
        ("Conflict", cj.conflict.len()),
        ("Provide", cj.provide.len()),
        ("Suggest", cj.suggest.len()),
    ];
    let relation_spans: Vec<Span> = relations
        .iter()
        .filter(|(_, count)| *count > 0)
        .flat_map(|(label, count)| {
            vec![
                Span::styled(format!("{label} ({count})"), styles::key_style()),
                Span::raw("  "),
            ]
        })
        .collect();
    if !relation_spans.is_empty() {
        lines.push(Line::from(relation_spans));
    }

    // Replace details
    if !cj.replace.is_empty() {
        for (name, constraint) in &cj.replace {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(name.as_str(), Style::default().fg(theme::COLOR_TEXT)),
                Span::raw(" "),
                Span::styled(constraint.as_str(), styles::muted_style()),
            ]));
        }
    }

    // Conflict details
    if !cj.conflict.is_empty() {
        for (name, constraint) in &cj.conflict {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(name.as_str(), Style::default().fg(theme::COLOR_WARNING)),
                Span::raw(" "),
                Span::styled(constraint.as_str(), styles::muted_style()),
            ]));
        }
    }

    // Provide details
    if !cj.provide.is_empty() {
        for (name, constraint) in &cj.provide {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(name.as_str(), Style::default().fg(theme::COLOR_TEXT)),
                Span::raw(" "),
                Span::styled(constraint.as_str(), styles::muted_style()),
            ]));
        }
    }

    // Suggest details
    if !cj.suggest.is_empty() {
        for (name, desc) in &cj.suggest {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(name.as_str(), Style::default().fg(theme::COLOR_TEXT)),
                Span::raw(" "),
                Span::styled(desc.as_str(), styles::muted_style()),
            ]));
        }
    }

    lines.push(Line::default());

    // Scripts
    if !cj.scripts.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("Scripts ({})", cj.scripts.len()),
            styles::key_style(),
        )));
        let mut script_names: Vec<&str> = cj.scripts.keys().map(|k| k.as_str()).collect();
        script_names.sort();
        for name in &script_names {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(*name, Style::default().fg(theme::COLOR_TEXT)),
            ]));
        }
    }

    // Repositories
    if let Some(repos) = &cj.repositories {
        let count = match repos {
            serde_json::Value::Array(arr) => arr.len(),
            serde_json::Value::Object(obj) => obj.len(),
            _ => 0,
        };
        if count > 0 {
            lines.push(Line::default());
            lines.push(Line::from(Span::styled(
                format!("Repositories ({count})"),
                styles::key_style(),
            )));
            if let serde_json::Value::Array(arr) = repos {
                for repo in arr {
                    let repo_type = repo.get("type").and_then(|v| v.as_str()).unwrap_or("?");
                    let url = repo.get("url").and_then(|v| v.as_str()).unwrap_or("");
                    lines.push(Line::from(vec![
                        Span::raw("  "),
                        Span::styled(repo_type, styles::muted_style()),
                        Span::raw(" "),
                        Span::styled(url, Style::default().fg(theme::COLOR_INFO)),
                    ]));
                }
            }
        }
    }

    // Config platform
    if let Some(config) = &cj.config {
        if let Some(platform) = config.get("platform").and_then(|v| v.as_object()) {
            lines.push(Line::default());
            lines.push(Line::from(Span::styled("Platform", styles::key_style())));
            for (key, val) in platform {
                let version = val.as_str().unwrap_or("");
                lines.push(Line::from(vec![
                    Span::raw("  "),
                    Span::styled(key.as_str(), Style::default().fg(theme::COLOR_TEXT)),
                    Span::raw(" "),
                    Span::styled(version, styles::version_style()),
                ]));
            }
        }
    }

    let paragraph = Paragraph::new(lines);
    paragraph.render(inner, buf);
}

fn styled_field<'a>(label: &'a str, value: &str, value_style: Style) -> Line<'a> {
    Line::from(vec![
        Span::styled(label, styles::key_style()),
        Span::raw("  "),
        Span::styled(value.to_string(), value_style),
    ])
}

/// String-based project view (for tests).
pub fn project_view(cj: &ComposerJSON) -> String {
    let mut b = String::new();

    if !cj.name.is_empty() {
        b.push_str(&cj.name);
        if !cj.license.is_empty() {
            b.push_str(&format!("  {}", cj.license.join(", ")));
        }
        b.push('\n');
    }
    if !cj.description.is_empty() {
        b.push_str(&format!("{}\n", cj.description));
    }
    if !cj.minimum_stability.is_empty() {
        b.push_str(&format!("Stability:  {}", cj.minimum_stability));
        if cj.prefer_stable {
            b.push_str(" (prefer-stable)");
        }
        b.push('\n');
    }
    if !cj.homepage.is_empty() {
        b.push_str(&format!("Homepage:  {}\n", cj.homepage));
    }
    if !cj.authors.is_empty() {
        let names: Vec<&str> = cj.authors.iter().map(|a| a.name.as_str()).collect();
        b.push_str(&format!("Authors:  {}\n", names.join(", ")));
    }
    if !cj.scripts.is_empty() {
        let mut keys: Vec<&str> = cj.scripts.keys().map(|k| k.as_str()).collect();
        keys.sort();
        b.push_str(&format!("Scripts:  {}\n", keys.join(", ")));
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composer::{Author, Support};

    #[test]
    fn project_view_full() {
        let cj = ComposerJSON {
            name: "test/project".to_string(),
            description: "A test project".to_string(),
            license: vec!["MIT".to_string()],
            homepage: "https://example.com".to_string(),
            minimum_stability: "stable".to_string(),
            prefer_stable: true,
            authors: vec![
                Author {
                    name: "John".to_string(),
                    ..Default::default()
                },
                Author {
                    name: "Jane".to_string(),
                    ..Default::default()
                },
            ],
            scripts: [(
                "test".to_string(),
                serde_json::Value::String("phpunit".to_string()),
            )]
            .into_iter()
            .collect(),
            ..Default::default()
        };
        let view = project_view(&cj);
        assert!(view.contains("test/project  MIT"));
        assert!(view.contains("A test project"));
        assert!(view.contains("Stability:  stable (prefer-stable)"));
        assert!(view.contains("Homepage:  https://example.com"));
        assert!(view.contains("Authors:  John, Jane"));
        assert!(view.contains("Scripts:  test"));
    }

    #[test]
    fn project_view_minimal() {
        let cj = ComposerJSON {
            name: "a/b".to_string(),
            ..Default::default()
        };
        let view = project_view(&cj);
        assert!(view.contains("a/b"));
        assert!(!view.contains("Stability"));
        assert!(!view.contains("Scripts"));
    }

    #[test]
    fn project_view_with_support() {
        let cj = ComposerJSON {
            name: "test/pkg".to_string(),
            support: Some(Support {
                issues: "https://github.com/test/issues".to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };
        let view = project_view(&cj);
        assert!(view.contains("test/pkg"));
    }
}
