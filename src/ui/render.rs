use ratatui::{
    prelude::*,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState,
    },
    Frame,
};

use crate::diff::LineChange;

use super::rebase::render_rebase_ui;
use super::syntax::highlight_line_changes;
use super::theme::Theme;
use super::types::*;

pub fn ui(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Apply the theme's root background to the entire frame
    let bg = Block::default().style(Style::default().bg(app.theme.bg_default));
    f.render_widget(bg, size);

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Min(0),    // Content
            Constraint::Length(1), // Help
        ])
        .split(size);

    render_header(f, app, main_chunks[0]);

    // Clamp scroll position so it cannot exceed content bounds
    if matches!(app.app_mode, AppMode::Diff) {
        clamp_scroll(app, main_chunks[1].height);
    }

    match app.app_mode {
        AppMode::Diff => match app.view_mode {
            ViewMode::SideBySide => {
                let content_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(20),
                        Constraint::Percentage(40),
                        Constraint::Percentage(40),
                    ])
                    .split(main_chunks[1]);

                render_file_list(f, app, content_chunks[0]);
                if !app.file_names.is_empty() {
                    render_side_by_side(f, app, content_chunks[1], content_chunks[2]);
                }
            }
            ViewMode::Unified => {
                let content_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
                    .split(main_chunks[1]);

                render_file_list(f, app, content_chunks[0]);
                if !app.file_names.is_empty() {
                    render_unified_diff(f, app, content_chunks[1]);
                }
            }
        },
        AppMode::Rebase => {
            render_rebase_ui(f, app, main_chunks[1]);
        }
    }

    render_help(f, app, main_chunks[2]);

    if app.show_rebase_modal {
        render_rebase_notification(f, app, size);
    }

    if app.show_help_modal {
        render_help_modal(f, app, size);
    }
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let view_mode = match app.view_mode {
        ViewMode::SideBySide => "Side-by-Side",
        ViewMode::Unified => "Unified",
    };
    let mode = match app.app_mode {
        AppMode::Diff => "DIFF",
        AppMode::Rebase => "REBASE",
    };
    let file_count = app.file_names.len();
    let current = if file_count > 0 {
        app.current_file_idx + 1
    } else {
        0
    };
    let current_file = app
        .file_names
        .get(app.current_file_idx)
        .map(|s| s.as_str())
        .unwrap_or("");

    let mut spans = vec![
        Span::styled(
            " giff ",
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ),
        Span::styled("\u{2502} ", Style::default().fg(t.border_dim)),
        Span::styled(
            format!("{} \u{2192} {}", app.left_label, app.right_label),
            Style::default().fg(t.fg_normal),
        ),
        Span::styled(" \u{2502} ", Style::default().fg(t.border_dim)),
        Span::styled(mode.to_owned(), Style::default().fg(t.accent)),
        Span::styled(" \u{2502} ", Style::default().fg(t.border_dim)),
        Span::styled(view_mode.to_owned(), Style::default().fg(t.fg_dim)),
    ];

    if !current_file.is_empty() {
        spans.push(Span::styled(
            " \u{2502} ",
            Style::default().fg(t.border_dim),
        ));
        spans.push(Span::styled(
            current_file.to_owned(),
            Style::default().fg(t.fg_bright),
        ));
    }

    spans.push(Span::styled(
        " \u{2502} ",
        Style::default().fg(t.border_dim),
    ));
    spans.push(Span::styled(
        format!("{}/{}", current, file_count),
        Style::default().fg(t.fg_dim),
    ));

    let header = Paragraph::new(Line::from(spans)).style(Style::default().bg(t.bg_header));
    f.render_widget(header, area);
}

pub fn render_file_list(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let is_focused = matches!(app.focused_pane, Pane::FileList);
    let border_color = if is_focused {
        t.border_focused
    } else {
        t.border_dim
    };
    let title_style = if is_focused {
        Style::default().fg(t.accent).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(t.fg_dim)
    };

    let block = Block::default()
        .title(Span::styled(" Files ", title_style))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    if app.file_names.is_empty() {
        let empty = Paragraph::new(Span::styled("  No changes", Style::default().fg(t.fg_dim)))
            .block(block);
        f.render_widget(empty, area);
        return;
    }

    // Available width inside the block: area width minus borders (2) minus highlight symbol width (2)
    let inner_width = area.width.saturating_sub(4) as usize;

    let items: Vec<ListItem> = app
        .file_names
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let (adds, dels) = count_file_changes(app, file);
            let is_current = i == app.current_file_idx;
            let name_style = if is_current {
                Style::default()
                    .fg(t.fg_bright)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(t.fg_normal)
            };

            // Calculate how much space the stats suffix needs (no allocations)
            let stats_width = if adds > 0 || dels > 0 {
                let mut w = 1; // leading space
                if adds > 0 {
                    w += 1 + digit_count(adds); // "+" + digits
                }
                if adds > 0 && dels > 0 {
                    w += 1; // space between
                }
                if dels > 0 {
                    w += 1 + digit_count(dels); // "-" + digits
                }
                w
            } else {
                0
            };

            let max_name_width = inner_width.saturating_sub(stats_width);
            let char_count = file.chars().count();
            let display_name = if char_count > max_name_width {
                if max_name_width <= 1 {
                    "\u{2026}".to_string()
                } else {
                    // Keep the tail — the filename is more useful than the directory prefix
                    let skip = char_count - (max_name_width - 1);
                    let truncated: String = file.chars().skip(skip).collect();
                    format!("\u{2026}{}", truncated)
                }
            } else {
                file.clone()
            };

            let mut spans = vec![Span::styled(display_name, name_style)];
            if adds > 0 || dels > 0 {
                spans.push(Span::styled(" ", Style::default()));
                if adds > 0 {
                    spans.push(Span::styled(
                        format!("+{}", adds),
                        Style::default().fg(t.fg_added),
                    ));
                }
                if adds > 0 && dels > 0 {
                    spans.push(Span::styled(" ", Style::default()));
                }
                if dels > 0 {
                    spans.push(Span::styled(
                        format!("-{}", dels),
                        Style::default().fg(t.fg_removed),
                    ));
                }
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    let list = List::new(items)
        .block(block)
        .highlight_style(Style::default().bg(t.bg_selection))
        .highlight_symbol("\u{258c} ");

    f.render_stateful_widget(
        list,
        area,
        &mut ratatui::widgets::ListState::default().with_selected(Some(app.current_file_idx)),
    );
}

#[allow(clippy::too_many_arguments)]
fn render_diff_pane(
    f: &mut Frame,
    title: &str,
    lines: &[(usize, String)],
    filename: &str,
    scroll: usize,
    is_focused: bool,
    area: Rect,
    theme: &Theme,
) {
    let border_color = if is_focused {
        theme.border_focused
    } else {
        theme.border_dim
    };
    let title_style = if is_focused {
        Style::default()
            .fg(theme.accent)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.fg_dim)
    };

    let highlighted = highlight_line_changes(lines, filename, theme);
    let total_lines = highlighted.len();
    let content = Text::from(highlighted);
    let visible_height = area.height.saturating_sub(2) as usize;

    let title_text = if total_lines > visible_height {
        let max_scroll = total_lines.saturating_sub(visible_height);
        let pos = scroll.min(max_scroll);
        let pct = if max_scroll > 0 {
            (pos * 100) / max_scroll
        } else {
            0
        };
        format!(" {} ({}%) ", title, pct)
    } else {
        format!(" {} ", title)
    };

    let block = Block::default()
        .title(Span::styled(title_text, title_style))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color));

    // ratatui Paragraph::scroll() accepts (u16, u16); clamp for content >65k lines.
    let scroll_u16 = scroll.min(u16::MAX as usize) as u16;
    let paragraph = Paragraph::new(content).block(block).scroll((scroll_u16, 0));
    f.render_widget(paragraph, area);

    // Scrollbar
    if total_lines > visible_height {
        let scrollbar_area = Rect::new(
            area.x,
            area.y + 1,
            area.width,
            area.height.saturating_sub(2),
        );
        let max_scroll = total_lines.saturating_sub(visible_height);
        let mut scrollbar_state = ScrollbarState::new(max_scroll).position(scroll.min(max_scroll));
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            scrollbar_area,
            &mut scrollbar_state,
        );
    }
}

/// Produce aligned line vectors for side-by-side display.
/// Gap lines are represented as `(0, String::new())`.
pub(super) fn align_lines(
    base_lines: &[LineChange],
    head_lines: &[LineChange],
) -> (Vec<LineChange>, Vec<LineChange>) {
    let mut aligned_base = Vec::new();
    let mut aligned_head = Vec::new();
    let mut bi = 0;
    let mut hi = 0;

    while bi < base_lines.len() || hi < head_lines.len() {
        let b_is_change = bi < base_lines.len() && base_lines[bi].1.starts_with('-');
        let h_is_change = hi < head_lines.len() && head_lines[hi].1.starts_with('+');

        if b_is_change || h_is_change {
            // Collect consecutive change lines from each side
            let mut b_chunk = Vec::new();
            let mut h_chunk = Vec::new();

            while bi < base_lines.len() && base_lines[bi].1.starts_with('-') {
                b_chunk.push(base_lines[bi].clone());
                bi += 1;
            }
            while hi < head_lines.len() && head_lines[hi].1.starts_with('+') {
                h_chunk.push(head_lines[hi].clone());
                hi += 1;
            }

            // Pair change lines, padding the shorter side with gaps
            let max_len = b_chunk.len().max(h_chunk.len());
            for i in 0..max_len {
                aligned_base.push(b_chunk.get(i).cloned().unwrap_or((0, String::new())));
                aligned_head.push(h_chunk.get(i).cloned().unwrap_or((0, String::new())));
            }
        } else if bi < base_lines.len() && hi < head_lines.len() {
            // Both are context lines
            aligned_base.push(base_lines[bi].clone());
            aligned_head.push(head_lines[hi].clone());
            bi += 1;
            hi += 1;
        } else if bi < base_lines.len() {
            aligned_base.push(base_lines[bi].clone());
            aligned_head.push((0, String::new()));
            bi += 1;
        } else {
            aligned_base.push((0, String::new()));
            aligned_head.push(head_lines[hi].clone());
            hi += 1;
        }
    }

    (aligned_base, aligned_head)
}

/// Compute the number of aligned lines without allocating full vectors.
pub(super) fn aligned_line_count(base_lines: &[LineChange], head_lines: &[LineChange]) -> usize {
    let mut count = 0;
    let mut bi = 0;
    let mut hi = 0;

    while bi < base_lines.len() || hi < head_lines.len() {
        let b_is_change = bi < base_lines.len() && base_lines[bi].1.starts_with('-');
        let h_is_change = hi < head_lines.len() && head_lines[hi].1.starts_with('+');

        if b_is_change || h_is_change {
            let mut b_count = 0;
            let mut h_count = 0;
            while bi < base_lines.len() && base_lines[bi].1.starts_with('-') {
                b_count += 1;
                bi += 1;
            }
            while hi < head_lines.len() && head_lines[hi].1.starts_with('+') {
                h_count += 1;
                hi += 1;
            }
            count += b_count.max(h_count);
        } else {
            if bi < base_lines.len() {
                bi += 1;
            }
            if hi < head_lines.len() {
                hi += 1;
            }
            count += 1;
        }
    }

    count
}

/// Compute the number of unified diff lines without allocating.
pub(super) fn unified_line_count(base_lines: &[LineChange], head_lines: &[LineChange]) -> usize {
    let mut count = 0;
    let mut bi = 0;
    let mut hi = 0;

    while bi < base_lines.len() || hi < head_lines.len() {
        let b_is_change = bi < base_lines.len() && base_lines[bi].1.starts_with('-');
        let h_is_change = hi < head_lines.len() && head_lines[hi].1.starts_with('+');

        if b_is_change || h_is_change {
            while bi < base_lines.len() && base_lines[bi].1.starts_with('-') {
                count += 1;
                bi += 1;
            }
            while hi < head_lines.len() && head_lines[hi].1.starts_with('+') {
                count += 1;
                hi += 1;
            }
        } else {
            if bi < base_lines.len() {
                bi += 1;
            }
            if hi < head_lines.len() {
                hi += 1;
            }
            count += 1;
        }
    }

    count
}

fn render_side_by_side(f: &mut Frame, app: &App, base_area: Rect, head_area: Rect) {
    let current_file = match app.file_names.get(app.current_file_idx) {
        Some(f) => f,
        None => return,
    };
    let (base_lines, head_lines) = match app.file_changes.get(current_file) {
        Some(c) => c,
        None => return,
    };
    let scroll = *app.scroll_positions.get(current_file).unwrap_or(&0);
    let is_focused = matches!(app.focused_pane, Pane::DiffContent);

    let (aligned_base, aligned_head) = align_lines(base_lines, head_lines);

    render_diff_pane(
        f,
        app.left_label,
        &aligned_base,
        current_file,
        scroll,
        is_focused,
        base_area,
        &app.theme,
    );
    render_diff_pane(
        f,
        app.right_label,
        &aligned_head,
        current_file,
        scroll,
        is_focused,
        head_area,
        &app.theme,
    );
}

/// Build unified diff lines by walking both lists in order.
/// Context lines appear once; change blocks show removals then additions.
pub(super) fn build_unified_lines(
    base_lines: &[LineChange],
    head_lines: &[LineChange],
) -> Vec<LineChange> {
    let mut unified = Vec::new();
    let mut bi = 0;
    let mut hi = 0;

    while bi < base_lines.len() || hi < head_lines.len() {
        let b_is_change = bi < base_lines.len() && base_lines[bi].1.starts_with('-');
        let h_is_change = hi < head_lines.len() && head_lines[hi].1.starts_with('+');

        if b_is_change || h_is_change {
            // Change block: all removals first, then all additions
            while bi < base_lines.len() && base_lines[bi].1.starts_with('-') {
                unified.push(base_lines[bi].clone());
                bi += 1;
            }
            while hi < head_lines.len() && head_lines[hi].1.starts_with('+') {
                unified.push(head_lines[hi].clone());
                hi += 1;
            }
        } else {
            // Context line — take from base (preferred), or head if base exhausted
            if bi < base_lines.len() {
                unified.push(base_lines[bi].clone());
                bi += 1;
                if hi < head_lines.len() {
                    hi += 1;
                }
            } else if hi < head_lines.len() {
                unified.push(head_lines[hi].clone());
                hi += 1;
            }
        }
    }

    unified
}

fn render_unified_diff(f: &mut Frame, app: &App, area: Rect) {
    let current_file = match app.file_names.get(app.current_file_idx) {
        Some(f) => f,
        None => return,
    };
    let (base_lines, head_lines) = match app.file_changes.get(current_file) {
        Some(c) => c,
        None => return,
    };
    let scroll = *app.scroll_positions.get(current_file).unwrap_or(&0);
    let is_focused = matches!(app.focused_pane, Pane::DiffContent);

    let unified_lines = build_unified_lines(base_lines, head_lines);

    let title = format!("{} vs {}", app.left_label, app.right_label);
    render_diff_pane(
        f,
        &title,
        &unified_lines,
        current_file,
        scroll,
        is_focused,
        area,
        &app.theme,
    );
}

fn render_rebase_notification(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    if let Some(notification) = &app.rebase_notification {
        let mut max_line_length = 0;
        let mut line_count = 0;
        for line in notification.lines() {
            max_line_length = max_line_length.max(line.len());
            line_count += 1;
        }
        let modal_width = (max_line_length as u16 + 6).min(70);
        let modal_height = (line_count as u16 + 6).min(16);
        let modal_area = centered_rect(modal_width, modal_height, area);

        // Dim the background behind the modal
        let dim_bg = Block::default().style(Style::default().bg(t.bg_modal_dim));
        f.render_widget(dim_bg, area);

        let background = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(t.accent))
            .style(Style::default().bg(t.bg_modal))
            .title(Span::styled(
                " Rebase Recommended ",
                Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
            ));

        f.render_widget(Clear, modal_area);
        f.render_widget(&background, modal_area);

        let inner_area = background.inner(modal_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(line_count as u16 + 2),
                Constraint::Length(3),
            ])
            .split(inner_area);

        let message = Paragraph::new(notification.clone())
            .style(Style::default().fg(t.fg_normal))
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });
        f.render_widget(message, chunks[0]);

        let button_spans = vec![
            Span::styled(" r ", Style::default().fg(t.fg_badge).bg(t.fg_key)),
            Span::styled(" Rebase now  ", Style::default().fg(t.fg_normal)),
            Span::styled(" i ", Style::default().fg(t.fg_badge).bg(t.fg_dim)),
            Span::styled(" Ignore", Style::default().fg(t.fg_normal)),
        ];
        let buttons = Paragraph::new(Line::from(button_spans)).alignment(Alignment::Center);
        f.render_widget(buttons, chunks[1]);
    }
}

fn render_help_modal(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    let is_rebase = matches!(app.app_mode, AppMode::Rebase);

    let modal_width = 56u16;
    let modal_height = 29u16;
    let modal_area = centered_rect(modal_width, modal_height, area);

    // Dim the background behind the modal
    let dim_bg = Block::default().style(Style::default().bg(t.bg_modal_dim));
    f.render_widget(dim_bg, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(t.border_modal))
        .style(Style::default().bg(t.bg_modal))
        .title(Span::styled(
            " Keybindings ",
            Style::default().fg(t.accent).add_modifier(Modifier::BOLD),
        ))
        .title_bottom(Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(" ? ", Style::default().fg(t.fg_badge).bg(t.fg_key)),
            Span::styled(" ", Style::default()),
            Span::styled(" Esc ", Style::default().fg(t.fg_badge).bg(t.fg_key)),
            Span::styled(" to close ", Style::default().fg(t.fg_dim)),
        ]));

    f.render_widget(Clear, modal_area);
    f.render_widget(&block, modal_area);

    let inner = block.inner(modal_area);
    let inner_width = inner.width as usize;

    let accent = t.accent;
    let fg_normal = t.fg_normal;
    let fg_bright = t.fg_bright;
    let bg_key_badge = t.bg_key_badge;
    let fg_separator = t.fg_separator;

    let section = |title: &str| -> Line<'static> {
        Line::from(vec![
            Span::styled("  \u{25cf} ", Style::default().fg(accent)),
            Span::styled(
                title.to_owned(),
                Style::default().fg(accent).add_modifier(Modifier::BOLD),
            ),
        ])
    };

    let sep = |w: usize| -> Line<'static> {
        Line::from(Span::styled(
            "\u{2500}".repeat(w),
            Style::default().fg(fg_separator),
        ))
    };

    let row = |key: &str, desc: &str| -> Line<'static> {
        Line::from(vec![
            Span::styled("    ", Style::default()),
            Span::styled(
                format!(" {:^8} ", key),
                Style::default().fg(fg_bright).bg(bg_key_badge),
            ),
            Span::styled("  ", Style::default()),
            Span::styled(desc.to_owned(), Style::default().fg(fg_normal)),
        ])
    };

    let empty = || -> Line<'static> { Line::from("") };

    let mut lines: Vec<Line<'static>> = vec![
        empty(),
        section("Navigation"),
        empty(),
        row("j / \u{2193}", "Move down / next item"),
        row("k / \u{2191}", "Move up / previous item"),
        row("PgDn", "Page down"),
        row("PgUp", "Page up"),
        row("Home", "Go to first"),
        row("End", "Go to last"),
        sep(inner_width),
    ];

    if is_rebase {
        lines.extend(vec![
            empty(),
            section("Rebase"),
            empty(),
            row("a", "Accept current change"),
            row("x", "Reject current change"),
            row("n", "Next file with changes"),
            row("p", "Previous file with changes"),
            row("c", "Commit accepted changes"),
            row("Esc", "Back to diff mode"),
            sep(inner_width),
            empty(),
            section("General"),
            empty(),
            row("?", "Toggle this help"),
        ]);
    } else {
        lines.extend(vec![
            empty(),
            section("Diff View"),
            empty(),
            row("Tab", "Toggle focus (files / diff)"),
            row("h / \u{2190}", "Focus file list"),
            row("l / \u{2192}", "Focus diff content"),
            row("u", "Toggle unified / side-by-side"),
            row("t", "Toggle dark / light theme"),
            row("r", "Enter rebase mode"),
            sep(inner_width),
            empty(),
            section("General"),
            empty(),
            row("q / Esc", "Quit"),
            row("?", "Toggle this help"),
        ]);
    }

    let text = Text::from(lines);
    let paragraph = Paragraph::new(text).style(Style::default().bg(t.bg_modal));
    f.render_widget(paragraph, inner);
}

fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    if r.width == 0 || r.height == 0 {
        return r;
    }

    let height = height.min(r.height);
    let width = width.min(r.width);

    let vert_margin = 100u16.saturating_sub(height * 100 / r.height) / 2;
    let horiz_margin = 100u16.saturating_sub(width * 100 / r.width) / 2;

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(vert_margin),
            Constraint::Length(height),
            Constraint::Percentage(vert_margin),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(horiz_margin),
            Constraint::Length(width),
            Constraint::Percentage(horiz_margin),
        ])
        .split(popup_layout[1])[1]
}

fn render_help(f: &mut Frame, app: &App, area: Rect) {
    let t = &app.theme;
    if let Some(msg) = &app.status_message {
        let is_error = msg.starts_with("Error");
        let color = if is_error { t.fg_removed } else { t.fg_added };
        let help = Paragraph::new(Line::from(Span::styled(
            format!(" {}", msg),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        )))
        .style(Style::default().bg(t.bg_header));
        f.render_widget(help, area);
        return;
    }

    let pairs: &[(&str, &str)] = match app.app_mode {
        AppMode::Diff => &[
            ("q", "Quit"),
            ("j/k", "Navigate"),
            ("Tab", "Focus"),
            ("h/l", "Panes"),
            ("u", "View"),
            ("t", "Theme"),
            ("PgUp/Dn", "Page"),
            ("r", "Rebase"),
            ("?", "Help"),
        ],
        AppMode::Rebase => &[
            ("Esc", "Back"),
            ("j/k", "Navigate"),
            ("a", "Accept"),
            ("x", "Reject"),
            ("n/p", "Files"),
            ("c", "Commit"),
            ("?", "Help"),
        ],
    };

    let mut spans: Vec<Span> = vec![Span::styled(" ", Style::default())];
    for (i, (key, desc)) in pairs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled("  ", Style::default().fg(t.border_dim)));
        }
        spans.push(Span::styled(
            (*key).to_owned(),
            Style::default().fg(t.fg_key),
        ));
        spans.push(Span::styled(
            format!(" {}", desc),
            Style::default().fg(t.fg_dim),
        ));
    }

    let help = Paragraph::new(Line::from(spans)).style(Style::default().bg(t.bg_header));
    f.render_widget(help, area);
}

fn clamp_scroll(app: &mut App, content_area_height: u16) {
    let file = match app.file_names.get(app.current_file_idx) {
        Some(f) => f,
        None => return,
    };
    let (base, head) = match app.file_changes.get(file) {
        Some(c) => c,
        None => return,
    };

    let content_len = match app.view_mode {
        ViewMode::SideBySide => aligned_line_count(base, head),
        ViewMode::Unified => unified_line_count(base, head),
    };

    let visible = content_area_height.saturating_sub(2) as usize;
    if content_len <= visible {
        app.scroll_positions.insert(file.clone(), 0);
        return;
    }
    let max_scroll = content_len - visible;
    let scroll = app.scroll_positions.get(file).copied().unwrap_or(0);
    if scroll > max_scroll {
        app.scroll_positions.insert(file.clone(), max_scroll);
    }
}

fn digit_count(n: usize) -> usize {
    if n == 0 {
        return 1;
    }
    let mut count = 0;
    let mut v = n;
    while v > 0 {
        count += 1;
        v /= 10;
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_digit_count() {
        assert_eq!(digit_count(0), 1);
        assert_eq!(digit_count(1), 1);
        assert_eq!(digit_count(9), 1);
        assert_eq!(digit_count(10), 2);
        assert_eq!(digit_count(99), 2);
        assert_eq!(digit_count(100), 3);
        assert_eq!(digit_count(999), 3);
        assert_eq!(digit_count(1000), 4);
        assert_eq!(digit_count(usize::MAX), usize::MAX.to_string().len());
    }
}

fn count_file_changes(app: &App, file: &str) -> (usize, usize) {
    if let Some((base, head)) = app.file_changes.get(file) {
        let dels = base.iter().filter(|(_, l)| l.starts_with('-')).count();
        let adds = head.iter().filter(|(_, l)| l.starts_with('+')).count();
        (adds, dels)
    } else {
        (0, 0)
    }
}
