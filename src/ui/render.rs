use ratatui::{
    prelude::*,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use super::rebase::render_rebase_ui;
use super::syntax::highlight_line_changes;
use super::types::*;

pub fn ui(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Create main layout with 3 parts: header, content, footer
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Help
        ])
        .split(size);

    // Render header with title and controls
    render_header(f, app, main_chunks[0]);

    // Main content depends on the app mode
    match app.app_mode {
        AppMode::Diff => {
            // Render diff mode content
            match app.view_mode {
                ViewMode::SideBySide => {
                    let content_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Percentage(20), // File list
                            Constraint::Percentage(40), // Base content
                            Constraint::Percentage(40), // Head content
                        ])
                        .split(main_chunks[1]);

                    // Render file list
                    render_file_list(f, app, content_chunks[0]);

                    // Only render content if files exist
                    if !app.file_names.is_empty() {
                        // Render base content
                        render_base_content(f, app, content_chunks[1]);

                        // Render head content
                        render_head_content(f, app, content_chunks[2]);
                    }
                }
                ViewMode::Unified => {
                    let content_chunks = Layout::default()
                        .direction(Direction::Horizontal)
                        .constraints([
                            Constraint::Percentage(20), // File list
                            Constraint::Percentage(80), // Unified content
                        ])
                        .split(main_chunks[1]);

                    // Render file list
                    render_file_list(f, app, content_chunks[0]);

                    // Only render content if files exist
                    if !app.file_names.is_empty() {
                        // Render unified diff
                        render_unified_diff(f, app, content_chunks[1]);
                    }
                }
            }
        }
        AppMode::Rebase => {
            // Render rebase mode content
            render_rebase_ui(f, app, main_chunks[1]);
        }
    }

    // Render help footer
    render_help(f, app, main_chunks[2]);

    // Render rebase notification if needed
    if app.show_rebase_modal {
        render_rebase_notification(f, app, size);
    }
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let view_mode_text = match app.view_mode {
        ViewMode::SideBySide => "Side-by-Side",
        ViewMode::Unified => "Unified",
    };
    let title = format!(
        " giff - Comparing {} to {} [{}] ",
        app.left_label, app.right_label, view_mode_text
    );
    let header = Paragraph::new(title)
        .style(Style::default().fg(Color::White).bg(Color::Blue))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, area);
}

pub fn render_file_list(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .file_names
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let content = Line::from(Span::styled(
                file.clone(),
                Style::default().add_modifier(if i == app.current_file_idx {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
            ));
            ListItem::new(content)
        })
        .collect();

    let files_list = List::new(items)
        .block(Block::default().title("Files").borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol(">> ");

    // Use different style if FileList is focused
    let files_list = match app.focused_pane {
        Pane::FileList => files_list.block(
            Block::default()
                .title("Files")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        ),
        _ => files_list,
    };

    f.render_stateful_widget(
        files_list,
        area,
        &mut ratatui::widgets::ListState::default().with_selected(Some(app.current_file_idx)),
    );
}

fn render_base_content(f: &mut Frame, app: &App, area: Rect) {
    let current_file = if let Some(file) = app.file_names.get(app.current_file_idx) {
        file
    } else {
        return; // No file selected
    };

    let (base_lines, _) = if let Some(changes) = app.file_changes.get(current_file) {
        changes
    } else {
        return; // File not found in changes
    };

    let scroll = app.scroll_positions.get(current_file).unwrap_or(&0);

    // Use syntax highlighting
    let highlighted_content = highlight_line_changes(base_lines, current_file);
    let content = Text::from(highlighted_content);

    let base_paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(format!("{} ({})", app.left_label, current_file))
                .borders(Borders::ALL),
        )
        .scroll((*scroll, 0));

    // Use different style if DiffContent is focused
    let base_paragraph = match app.focused_pane {
        Pane::DiffContent => base_paragraph.block(
            Block::default()
                .title(format!("{} ({})", app.left_label, current_file))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        ),
        _ => base_paragraph,
    };

    f.render_widget(base_paragraph, area);
}

fn render_head_content(f: &mut Frame, app: &App, area: Rect) {
    let current_file = if let Some(file) = app.file_names.get(app.current_file_idx) {
        file
    } else {
        return; // No file selected
    };

    let (_, head_lines) = if let Some(changes) = app.file_changes.get(current_file) {
        changes
    } else {
        return; // File not found in changes
    };

    let scroll = app.scroll_positions.get(current_file).unwrap_or(&0);

    // Use syntax highlighting
    let highlighted_content = highlight_line_changes(head_lines, current_file);
    let content = Text::from(highlighted_content);

    let head_paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(format!("{} ({})", app.right_label, current_file))
                .borders(Borders::ALL),
        )
        .scroll((*scroll, 0));

    // Use different style if DiffContent is focused
    let head_paragraph = match app.focused_pane {
        Pane::DiffContent => head_paragraph.block(
            Block::default()
                .title(format!("{} ({})", app.right_label, current_file))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        ),
        _ => head_paragraph,
    };

    f.render_widget(head_paragraph, area);
}

fn render_unified_diff(f: &mut Frame, app: &App, area: Rect) {
    let current_file = if let Some(file) = app.file_names.get(app.current_file_idx) {
        file
    } else {
        return; // No file selected
    };

    let (base_lines, head_lines) = if let Some(changes) = app.file_changes.get(current_file) {
        changes
    } else {
        return; // File not found in changes
    };

    let scroll = app.scroll_positions.get(current_file).unwrap_or(&0);

    // Create a vector to store the unified lines for syntax highlighting
    let mut unified_lines: Vec<(usize, String)> = Vec::new();

    // Collect all line numbers from both sides
    let mut all_lines: Vec<(usize, bool)> = Vec::new(); // (line_number, is_head)
    for (num, _) in base_lines {
        all_lines.push((*num, false));
    }
    for (num, _) in head_lines {
        all_lines.push((*num, true));
    }

    // Sort by line number
    all_lines.sort_by_key(|(num, _)| *num);

    // Process lines
    let mut processed_lines = Vec::new();
    for (num, is_head) in all_lines {
        if is_head {
            // Find this line in head_lines
            if let Some((_, line)) = head_lines.iter().find(|(line_num, _)| *line_num == num) {
                if !line.starts_with('-') && !processed_lines.contains(&num) {
                    unified_lines.push((num, line.clone()));
                    processed_lines.push(num);
                }
            }
        } else {
            // Find this line in base_lines
            if let Some((_, line)) = base_lines.iter().find(|(line_num, _)| *line_num == num) {
                if !line.starts_with('+') && !processed_lines.contains(&num) {
                    unified_lines.push((num, line.clone()));
                    processed_lines.push(num);
                }
            }
        }
    }

    // Apply syntax highlighting to unified diff
    let highlighted_content = highlight_line_changes(&unified_lines, current_file);
    let content = Text::from(highlighted_content);

    // Use different style if DiffContent is focused
    let unified_paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(format!(
                    "Unified Diff: {} vs {} ({})",
                    app.left_label, app.right_label, current_file
                ))
                .borders(Borders::ALL),
        )
        .scroll((*scroll, 0));

    f.render_widget(unified_paragraph, area);
}

fn render_rebase_notification(f: &mut Frame, app: &App, area: Rect) {
    if let Some(notification) = &app.rebase_notification {
        // Calculate modal size
        let mut max_line_length = 0;
        let mut line_count = 0;
        for line in notification.lines() {
            max_line_length = max_line_length.max(line.len());
            line_count += 1;
        }
        let modal_width = (max_line_length as u16 + 4).min(80);
        let modal_height = (line_count as u16 + 6).min(20);

        let modal_area = centered_rect(modal_width, modal_height, area);

        // Render background
        let background = Block::default()
            .style(Style::default().bg(Color::Black))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title("Rebase Recommended");

        f.render_widget(Clear, modal_area); // Clear the area
        f.render_widget(&background, modal_area);

        let inner_area = background.inner(modal_area);

        // Split into message area and buttons
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(line_count as u16 + 2),
                Constraint::Length(3),
            ])
            .split(inner_area);

        // Render notification message
        let message = Paragraph::new(notification.clone())
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });

        f.render_widget(message, chunks[0]);

        // Render buttons
        let buttons = Paragraph::new("Press 'r' to rebase now   Press 'i' to ignore")
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center);

        f.render_widget(buttons, chunks[1]);
    }
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
    // Show status message if present, otherwise show help text
    if let Some(msg) = &app.status_message {
        let is_error = msg.starts_with("Error");
        let color = if is_error { Color::Red } else { Color::Green };
        let status = Paragraph::new(msg.as_str())
            .style(Style::default().fg(color).add_modifier(Modifier::BOLD))
            .alignment(Alignment::Center)
            .block(Block::default().borders(Borders::ALL));
        f.render_widget(status, area);
        return;
    }

    let help_text = match app.app_mode {
        AppMode::Diff => "Esc/q: Quit | j/k: Navigate | Tab: Change focus | h/l: Switch panes | u: Toggle view | r: Enter rebase mode",
        AppMode::Rebase => "Esc/q: Cancel | j/k: Navigate changes | a: Accept change | x: Reject change | c: Commit changes",
    };

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, area);
}
