use ratatui::{
    prelude::*,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::collections::{HashMap, HashSet};

use super::render::render_file_list;
use super::types::*;

pub fn prepare_rebase_changes(app: &mut App) {
    app.rebase_changes.clear();

    for file_name in &app.file_names {
        if let Some((base_lines, head_lines)) = app.file_changes.get(file_name) {
            let mut changes = Vec::new();

            // Helper function to extract context (3 lines before and after)
            let get_context = |lines: &[(usize, String)], line_num: usize| -> Vec<String> {
                let mut context = Vec::new();
                let start = if line_num > 3 { line_num - 3 } else { 1 };

                // Context lines before the change
                for i in start..line_num {
                    if let Some((_, line)) = lines.iter().find(|(num, _)| *num == i) {
                        context.push(format!("{}: {}", i, line));
                    }
                }

                // Context lines after the change
                for i in line_num + 1..=line_num + 3 {
                    if let Some((_, line)) = lines.iter().find(|(num, _)| *num == i) {
                        context.push(format!("{}: {}", i, line));
                    }
                }

                context
            };

            // First, find corresponding deleted/added lines to pair them
            let mut paired_changes = HashMap::new();

            // Map line numbers to their content for easier matching
            let mut base_map = HashMap::new();
            for (line_num, line) in base_lines {
                if line.starts_with('-') {
                    base_map.insert(*line_num, line.clone());
                }
            }

            let mut head_map = HashMap::new();
            for (line_num, line) in head_lines {
                if line.starts_with('+') {
                    head_map.insert(*line_num, line.clone());
                }
            }

            // Pair deleted lines with nearby added lines (closest match first).
            // Each head line can only be paired once.
            let mut used_head_nums: HashSet<usize> = HashSet::new();
            let mut sorted_base_nums: Vec<usize> = base_map.keys().copied().collect();
            sorted_base_nums.sort();

            for base_num in sorted_base_nums {
                let mut best_head_num = None;
                let mut best_distance = 5isize;

                for head_num in head_map.keys() {
                    if used_head_nums.contains(head_num) {
                        continue;
                    }
                    let distance = (*head_num as isize - base_num as isize).abs();
                    if distance < best_distance {
                        best_distance = distance;
                        best_head_num = Some(*head_num);
                    }
                }

                if let Some(head_num) = best_head_num {
                    paired_changes.insert(base_num, head_num);
                    used_head_nums.insert(head_num);
                }
            }

            // Build head→base insertion position mapping by aligning
            // context lines between the two sides of the diff.
            let mut base_insert_positions: HashMap<usize, usize> = HashMap::new();
            {
                let mut bi = 0;
                let mut last_base_pos = 0usize;

                for (h_num, h_line) in head_lines {
                    if h_line.starts_with('+') {
                        // Addition: insert after the last aligned base position
                        base_insert_positions.insert(*h_num, last_base_pos + 1);
                    } else {
                        // Context line: skip past any '-' lines in base
                        while bi < base_lines.len() && base_lines[bi].1.starts_with('-') {
                            bi += 1;
                        }
                        if bi < base_lines.len() {
                            last_base_pos = base_lines[bi].0;
                            bi += 1;
                        }
                    }
                }
            }

            // Add removed lines from base with their paired added lines
            for (line_num, line) in base_lines {
                if line.starts_with('-') {
                    let context = get_context(base_lines, *line_num);

                    // Check if this line has a paired addition
                    let paired_head_num = paired_changes.get(line_num);
                    let paired_content = paired_head_num
                        .and_then(|head_num| head_map.get(head_num))
                        .cloned();

                    changes.push(Change {
                        line_num: *line_num,
                        content: line.clone(),
                        paired_content,
                        state: ChangeState::Unselected,
                        is_base: true,
                        context,
                        base_insert_pos: None,
                    });
                }
            }

            // Add added lines from head that weren't paired
            for (line_num, line) in head_lines {
                if line.starts_with('+') && !paired_changes.values().any(|num| num == line_num) {
                    let context = get_context(head_lines, *line_num);
                    let base_pos = base_insert_positions.get(line_num).copied();
                    changes.push(Change {
                        line_num: *line_num,
                        content: line.clone(),
                        paired_content: None,
                        state: ChangeState::Unselected,
                        is_base: false,
                        context,
                        base_insert_pos: base_pos,
                    });
                }
            }

            // Sort by line number
            changes.sort_by_key(|change| change.line_num);

            app.rebase_changes.insert(file_name.clone(), changes);
        }
    }

    app.current_change_idx = 0;
}

pub fn render_rebase_ui(f: &mut Frame, app: &App, area: Rect) {
    // First, clear the background by rendering a filled block
    let clear_block = Block::default()
        .style(Style::default().bg(Color::Black)) // Use the terminal's default background
        .borders(Borders::NONE);
    f.render_widget(clear_block, area);

    // Now, set up the rebase UI layout
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20), // File list
            Constraint::Percentage(80), // Rebase content
        ])
        .split(area);

    // Render file list
    render_file_list(f, app, content_chunks[0]);

    // Render rebase content area
    if let Some(current_file) = app.file_names.get(app.current_file_idx) {
        // Create a clean background for the rebase area
        let rebase_bg = Block::default()
            .style(Style::default().bg(Color::Black))
            .borders(Borders::ALL)
            .title(format!("Rebase: {}", current_file));
        f.render_widget(&rebase_bg, content_chunks[1]);

        let inner_area = rebase_bg.inner(content_chunks[1]);

        if let Some(changes) = app.rebase_changes.get(current_file) {
            if changes.is_empty() {
                // Show a message if there are no changes
                let no_changes_text = Paragraph::new("No changes to rebase in this file")
                    .style(Style::default().fg(Color::White))
                    .alignment(Alignment::Center);

                f.render_widget(no_changes_text, inner_area);
                return;
            }

            // Split the rebase area into current change and context
            let rebase_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Percentage(40), // Current change
                    Constraint::Percentage(60), // Context
                ])
                .split(inner_area);

            // Current change section
            if app.current_change_idx < changes.len() {
                let current_change = &changes[app.current_change_idx];

                // Format the current change nicely
                let change_type = if current_change.is_base {
                    "Removed"
                } else {
                    "Added"
                };
                let state_symbol = match current_change.state {
                    ChangeState::Unselected => "[ ]",
                    ChangeState::Accepted => "[✓]",
                    ChangeState::Rejected => "[✗]",
                };

                let line_content = current_change
                    .content
                    .strip_prefix('+')
                    .or_else(|| current_change.content.strip_prefix('-'))
                    .unwrap_or(&current_change.content);

                let header = format!(
                    "{} {} (Line {})",
                    state_symbol, change_type, current_change.line_num
                );

                let mut content_text = vec![
                    Line::from(Span::styled(
                        header,
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    )),
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("Current: {}", line_content),
                        Style::default()
                            .fg(if current_change.is_base {
                                Color::Red
                            } else {
                                Color::Green
                            })
                            .add_modifier(Modifier::BOLD),
                    )),
                ];

                // If there's paired content (for changed lines), show it
                if let Some(paired_content) = &current_change.paired_content {
                    let paired_text = paired_content
                        .strip_prefix('+')
                        .or_else(|| paired_content.strip_prefix('-'))
                        .unwrap_or(paired_content);

                    content_text.push(Line::from(""));
                    content_text.push(Line::from(Span::styled(
                        format!("Incoming: {}", paired_text),
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    )));

                    // Add explicit choice text for removed lines
                    if current_change.is_base {
                        content_text.push(Line::from(""));
                        content_text.push(Line::from(Span::styled(
                            "Press 'a' to ACCEPT the incoming change (green)",
                            Style::default().fg(Color::Green),
                        )));
                        content_text.push(Line::from(Span::styled(
                            "Press 'x' to KEEP the current line and reject the incoming change",
                            Style::default().fg(Color::Red),
                        )));
                    }
                }

                let change_block = Paragraph::new(Text::from(content_text))
                    .block(
                        Block::default()
                            .title(format!(
                                "Change {}/{}",
                                app.current_change_idx + 1,
                                changes.len()
                            ))
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::Yellow)),
                    )
                    .alignment(Alignment::Left);

                f.render_widget(change_block, rebase_chunks[0]);

                // Context section
                let mut context_lines = Vec::new();
                context_lines.push(Line::from(Span::styled(
                    "Context:",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )));
                context_lines.push(Line::from(""));

                for line in &current_change.context {
                    context_lines.push(Line::from(Span::styled(
                        line,
                        Style::default().fg(Color::Gray),
                    )));
                }

                // Add instructions
                context_lines.push(Line::from(""));
                context_lines.push(Line::from(Span::styled(
                    "Press 'a' to accept this change",
                    Style::default().fg(Color::Green),
                )));
                context_lines.push(Line::from(Span::styled(
                    "Press 'x' to reject this change",
                    Style::default().fg(Color::Red),
                )));
                context_lines.push(Line::from(Span::styled(
                    "Press 'j'/'k' to navigate between changes",
                    Style::default().fg(Color::White),
                )));
                context_lines.push(Line::from(Span::styled(
                    "Press 'n'/'p' to navigate between files",
                    Style::default().fg(Color::White),
                )));
                context_lines.push(Line::from(Span::styled(
                    "Press 'c' to commit all accepted changes",
                    Style::default().fg(Color::Yellow),
                )));
                context_lines.push(Line::from(Span::styled(
                    "Press 'Esc' or 'q' to cancel and return to diff view",
                    Style::default().fg(Color::White),
                )));

                let context_block = Paragraph::new(Text::from(context_lines)).block(
                    Block::default()
                        .title("Context and Help")
                        .borders(Borders::ALL),
                );

                f.render_widget(context_block, rebase_chunks[1]);
            }
        } else {
            // Show message if no changes for this file
            let no_changes_text = Paragraph::new("No changes found for this file")
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Center);

            f.render_widget(no_changes_text, inner_area);
        }
    }
}
