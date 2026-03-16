use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::sync::LazyLock;
use syntect::easy::HighlightLines;
use syntect::highlighting::{self, ThemeSet};
use syntect::parsing::SyntaxSet;

use super::theme::Theme;

pub static SYNTAX_SET: LazyLock<SyntaxSet> = LazyLock::new(SyntaxSet::load_defaults_newlines);
pub static THEME_SET: LazyLock<ThemeSet> = LazyLock::new(ThemeSet::load_defaults);

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

pub fn highlight_line_changes(
    lines: &[(usize, String)],
    filename: &str,
    theme: &Theme,
) -> Vec<Line<'static>> {
    let syntax = SYNTAX_SET
        .find_syntax_for_file(filename)
        .ok()
        .flatten()
        .unwrap_or_else(|| SYNTAX_SET.find_syntax_plain_text());
    let syn_theme = match THEME_SET.themes.get(&theme.syntax_theme) {
        Some(t) => t,
        None => match THEME_SET.themes.values().next() {
            Some(t) => t,
            None => {
                return lines
                    .iter()
                    .map(|(num, line)| {
                        if *num == 0 {
                            return Line::from(Span::raw(""));
                        }
                        Line::from(Span::raw(format!("{:4}   {}", num, line)))
                    })
                    .collect()
            }
        },
    };
    let mut highlighter = HighlightLines::new(syntax, syn_theme);

    let fg_line_num = theme.fg_line_num;
    let bg_removed = theme.bg_removed;
    let bg_added = theme.bg_added;
    let fg_removed_marker = theme.fg_removed_marker;
    let fg_added_marker = theme.fg_added_marker;

    lines
        .iter()
        .map(|(line_num, line)| {
            // Gap/placeholder line for side-by-side alignment
            if *line_num == 0 {
                return Line::from(Span::raw(""));
            }
            if let Some(rest) = line.strip_prefix('-') {
                let mut spans = vec![
                    Span::styled(
                        format!("{:4} ", line_num),
                        Style::default().fg(fg_line_num).bg(bg_removed),
                    ),
                    Span::styled(
                        "- ",
                        Style::default()
                            .fg(fg_removed_marker)
                            .bg(bg_removed)
                            .add_modifier(Modifier::BOLD),
                    ),
                ];
                spans.extend(highlight_code_with_bg(rest, &mut highlighter, bg_removed));
                Line::from(spans)
            } else if let Some(rest) = line.strip_prefix('+') {
                let mut spans = vec![
                    Span::styled(
                        format!("{:4} ", line_num),
                        Style::default().fg(fg_line_num).bg(bg_added),
                    ),
                    Span::styled(
                        "+ ",
                        Style::default()
                            .fg(fg_added_marker)
                            .bg(bg_added)
                            .add_modifier(Modifier::BOLD),
                    ),
                ];
                spans.extend(highlight_code_with_bg(rest, &mut highlighter, bg_added));
                Line::from(spans)
            } else {
                let mut spans = vec![
                    Span::styled(format!("{:4} ", line_num), Style::default().fg(fg_line_num)),
                    Span::styled("  ", Style::default()),
                ];
                spans.extend(highlight_code(line.as_str(), &mut highlighter));
                Line::from(spans)
            }
        })
        .collect()
}
