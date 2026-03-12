use ratatui::{
    prelude::*,
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
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

            // Build a lookup map for O(1) context line access
            let base_line_map: HashMap<usize, &String> =
                base_lines.iter().map(|(n, l)| (*n, l)).collect();
            let head_line_map: HashMap<usize, &String> =
                head_lines.iter().map(|(n, l)| (*n, l)).collect();

            let get_context =
                |line_map: &HashMap<usize, &String>, line_num: usize| -> Vec<String> {
                    let mut context = Vec::new();
                    let start = if line_num > 3 { line_num - 3 } else { 1 };

                    for i in start..line_num {
                        if let Some(line) = line_map.get(&i) {
                            context.push(format!("{}: {}", i, line));
                        }
                    }
                    for i in line_num + 1..=line_num + 3 {
                        if let Some(line) = line_map.get(&i) {
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
                    let context = get_context(&base_line_map, *line_num);

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
                if line.starts_with('+') && !used_head_nums.contains(line_num) {
                    let context = get_context(&head_line_map, *line_num);
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
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(80),
        ])
        .split(area);

    render_file_list(f, app, content_chunks[0]);

    if let Some(current_file) = app.file_names.get(app.current_file_idx) {
        let rebase_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(BORDER_FOCUSED))
            .title(Span::styled(
                format!(" Rebase: {} ", current_file),
                Style::default()
                    .fg(ACCENT)
                    .add_modifier(Modifier::BOLD),
            ));
        f.render_widget(&rebase_block, content_chunks[1]);
        let inner_area = rebase_block.inner(content_chunks[1]);

        if let Some(changes) = app.rebase_changes.get(current_file) {
            if changes.is_empty() {
                let msg = Paragraph::new(Span::styled(
                    "No changes to rebase in this file",
                    Style::default().fg(FG_DIM),
                ))
                .alignment(Alignment::Center);
                f.render_widget(msg, inner_area);
                return;
            }

            // Count states for progress
            let accepted = changes
                .iter()
                .filter(|c| c.state == ChangeState::Accepted)
                .count();
            let rejected = changes
                .iter()
                .filter(|c| c.state == ChangeState::Rejected)
                .count();
            let remaining = changes.len() - accepted - rejected;

            let rebase_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2),      // Progress
                    Constraint::Percentage(50), // Current change
                    Constraint::Min(0),         // Context
                ])
                .split(inner_area);

            // Progress indicator
            let progress_spans = vec![
                Span::styled("  ", Style::default()),
                Span::styled(
                    format!(
                        "Change {}/{}",
                        app.current_change_idx + 1,
                        changes.len()
                    ),
                    Style::default()
                        .fg(FG_BRIGHT)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled("  \u{2502}  ", Style::default().fg(BORDER_DIM)),
                Span::styled(
                    format!("{} accepted", accepted),
                    Style::default().fg(FG_ADDED),
                ),
                Span::styled("  ", Style::default()),
                Span::styled(
                    format!("{} rejected", rejected),
                    Style::default().fg(FG_REMOVED),
                ),
                Span::styled("  ", Style::default()),
                Span::styled(
                    format!("{} remaining", remaining),
                    Style::default().fg(FG_DIM),
                ),
            ];
            let progress = Paragraph::new(Line::from(progress_spans));
            f.render_widget(progress, rebase_chunks[0]);

            // Current change
            if app.current_change_idx < changes.len() {
                let current_change = &changes[app.current_change_idx];
                let change_type = if current_change.is_base {
                    "Removed"
                } else {
                    "Added"
                };
                let (state_symbol, state_color) = match current_change.state {
                    ChangeState::Unselected => ("\u{25cb}", FG_DIM),
                    ChangeState::Accepted => ("\u{25cf}", FG_ADDED),
                    ChangeState::Rejected => ("\u{25cf}", FG_REMOVED),
                };

                let line_content = current_change
                    .content
                    .strip_prefix('+')
                    .or_else(|| current_change.content.strip_prefix('-'))
                    .unwrap_or(&current_change.content);

                let type_color = if current_change.is_base {
                    FG_REMOVED
                } else {
                    FG_ADDED
                };

                let mut content_text = vec![
                    Line::from(vec![
                        Span::styled(
                            format!(" {} ", state_symbol),
                            Style::default().fg(state_color),
                        ),
                        Span::styled(
                            format!("{} ", change_type),
                            Style::default()
                                .fg(type_color)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            format!("(line {})", current_change.line_num),
                            Style::default().fg(FG_DIM),
                        ),
                    ]),
                    Line::from(""),
                    Line::from(Span::styled(
                        format!("  {}", line_content),
                        Style::default().fg(type_color),
                    )),
                ];

                if let Some(paired) = &current_change.paired_content {
                    let paired_text = paired
                        .strip_prefix('+')
                        .or_else(|| paired.strip_prefix('-'))
                        .unwrap_or(paired);
                    content_text.push(Line::from(""));
                    content_text.push(Line::from(vec![
                        Span::styled("  \u{2192} ", Style::default().fg(FG_DIM)),
                        Span::styled(
                            paired_text.to_owned(),
                            Style::default()
                                .fg(FG_ADDED)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]));

                    if current_change.is_base {
                        content_text.push(Line::from(""));
                        content_text.push(Line::from(vec![
                            Span::styled("  ", Style::default()),
                            Span::styled("a", Style::default().fg(FG_KEY)),
                            Span::styled(" accept incoming  ", Style::default().fg(FG_DIM)),
                            Span::styled("x", Style::default().fg(FG_KEY)),
                            Span::styled(" keep current", Style::default().fg(FG_DIM)),
                        ]));
                    }
                }

                let change_block_widget = Block::default()
                    .title(Span::styled(
                        " Current Change ",
                        Style::default().fg(FG_KEY),
                    ))
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(BORDER_FOCUSED));

                let mut change_paragraph = Paragraph::new(Text::from(content_text))
                    .block(change_block_widget);

                match current_change.state {
                    ChangeState::Accepted => {
                        change_paragraph =
                            change_paragraph.style(Style::default().bg(BG_ACCEPTED));
                    }
                    ChangeState::Rejected => {
                        change_paragraph =
                            change_paragraph.style(Style::default().bg(BG_REJECTED));
                    }
                    ChangeState::Unselected => {}
                }

                f.render_widget(change_paragraph, rebase_chunks[1]);

                // Context section
                let mut context_lines = vec![Line::from("")];
                for line in &current_change.context {
                    context_lines.push(Line::from(Span::styled(
                        format!("  {}", line),
                        Style::default().fg(FG_DIM),
                    )));
                }

                let context_block = Paragraph::new(Text::from(context_lines)).block(
                    Block::default()
                        .title(Span::styled(
                            " Context ",
                            Style::default().fg(FG_DIM),
                        ))
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .border_style(Style::default().fg(BORDER_DIM)),
                );
                f.render_widget(context_block, rebase_chunks[2]);
            }
        } else {
            let msg = Paragraph::new(Span::styled(
                "No changes found for this file",
                Style::default().fg(FG_DIM),
            ))
            .alignment(Alignment::Center);
            f.render_widget(msg, inner_area);
        }
    }
}
