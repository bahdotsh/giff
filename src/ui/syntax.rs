use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::sync::LazyLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{self, ThemeSet};
use syntect::parsing::SyntaxSet;

pub static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
pub static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

// Diff line background tints
pub const BG_ADDED: Color = Color::Rgb(15, 40, 15);
pub const BG_REMOVED: Color = Color::Rgb(45, 15, 15);
const FG_LINE_NUM: Color = Color::Rgb(75, 80, 95);
const FG_ADDED_MARKER: Color = Color::Rgb(80, 210, 105);
const FG_REMOVED_MARKER: Color = Color::Rgb(235, 85, 85);

fn to_ratatui_color(c: highlighting::Color) -> Color {
    Color::Rgb(c.r, c.g, c.b)
}

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

fn highlight_code_with_bg(
    code: &str,
    highlighter: &mut HighlightLines,
    bg: Color,
) -> Vec<Span<'static>> {
    match highlighter.highlight_line(code, &SYNTAX_SET) {
        Ok(ranges) => ranges
            .into_iter()
            .map(|(style, text)| {
                Span::styled(
                    text.to_owned(),
                    Style::default()
                        .fg(to_ratatui_color(style.foreground))
                        .bg(bg),
                )
            })
            .collect(),
        Err(_) => vec![Span::styled(code.to_owned(), Style::default().bg(bg))],
    }
}

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
                    .map(|(num, line)| Line::from(Span::raw(format!("{:4}   {}", num, line))))
                    .collect()
            }
        },
    };
    let mut highlighter = HighlightLines::new(syntax, theme);

    lines
        .iter()
        .map(|(line_num, line)| {
            if let Some(rest) = line.strip_prefix('-') {
                let mut spans = vec![
                    Span::styled(
                        format!("{:4} ", line_num),
                        Style::default().fg(FG_LINE_NUM).bg(BG_REMOVED),
                    ),
                    Span::styled(
                        "- ",
                        Style::default()
                            .fg(FG_REMOVED_MARKER)
                            .bg(BG_REMOVED)
                            .add_modifier(Modifier::BOLD),
                    ),
                ];
                spans.extend(highlight_code_with_bg(rest, &mut highlighter, BG_REMOVED));
                Line::from(spans)
            } else if let Some(rest) = line.strip_prefix('+') {
                let mut spans = vec![
                    Span::styled(
                        format!("{:4} ", line_num),
                        Style::default().fg(FG_LINE_NUM).bg(BG_ADDED),
                    ),
                    Span::styled(
                        "+ ",
                        Style::default()
                            .fg(FG_ADDED_MARKER)
                            .bg(BG_ADDED)
                            .add_modifier(Modifier::BOLD),
                    ),
                ];
                spans.extend(highlight_code_with_bg(rest, &mut highlighter, BG_ADDED));
                Line::from(spans)
            } else {
                let mut spans = vec![
                    Span::styled(format!("{:4} ", line_num), Style::default().fg(FG_LINE_NUM)),
                    Span::styled("  ", Style::default()),
                ];
                spans.extend(highlight_code(line.as_str(), &mut highlighter));
                Line::from(spans)
            }
        })
        .collect()
}
