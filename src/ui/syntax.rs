use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use std::sync::LazyLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{self, ThemeSet};
use syntect::parsing::SyntaxSet;

pub static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
pub static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

/// Convert a syntect color to a ratatui Color
fn to_ratatui_color(c: highlighting::Color) -> Color {
    Color::Rgb(c.r, c.g, c.b)
}

/// Highlight a single line of code using syntect, returning styled spans
pub fn highlight_code(code: &str, highlighter: &mut HighlightLines) -> Vec<Span<'static>> {
    match highlighter.highlight_line(code, &SYNTAX_SET) {
        Ok(ranges) => ranges
            .into_iter()
            .map(|(style, text)| {
                Span::styled(
                    text.to_owned(),
                    Style::default().fg(to_ratatui_color(style.foreground)),
                )
            })
            .collect(),
        Err(_) => vec![Span::raw(code.to_owned())],
    }
}

/// Convert line changes to syntax highlighted spans using syntect
pub fn highlight_line_changes(lines: &[(usize, String)], filename: &str) -> Vec<Line<'static>> {
    let syntax = SYNTAX_SET
        .find_syntax_for_file(filename)
        .ok()
        .flatten()
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());
    let theme = match THEME_SET.themes.get("base16-ocean.dark") {
        Some(t) => t,
        None => match THEME_SET.themes.values().next() {
            Some(t) => t,
            None => {
                return lines
                    .iter()
                    .map(|(num, line)| Line::from(Span::raw(format!("{:4}  {}", num, line))))
                    .collect()
            }
        },
    };
    let mut highlighter = HighlightLines::new(syntax, theme);

    lines
        .iter()
        .map(|(line_num, line)| {
            let (prefix, color, code) = if let Some(rest) = line.strip_prefix('-') {
                (format!("{:4} -", line_num), Color::Red, rest)
            } else if let Some(rest) = line.strip_prefix('+') {
                (format!("{:4} +", line_num), Color::Green, rest)
            } else {
                (format!("{:4} ", line_num), Color::White, line.as_str())
            };

            let mut spans = vec![Span::styled(prefix, Style::default().fg(color))];
            spans.extend(highlight_code(code, &mut highlighter));
            Line::from(spans)
        })
        .collect()
}
