use crate::diff;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseEventKind};
use ratatui::{prelude::*, Terminal};
use std::io;
use std::process::Command;

use super::rebase::prepare_rebase_changes;
use super::render::ui;
use super::theme::Theme;
use super::types::*;

/// Returns `Ok(true)` when the app exits after a successful rebase
/// (so the caller can print a message), `Ok(false)` for normal exit.
pub fn run_ui<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<bool>
where
    std::io::Error: From<B::Error>,
{
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        // Block for the first event, then drain any queued events before
        // redrawing.  This batches rapid scroll inputs so the UI stays snappy.
        let first = event::read()?;
        let mut events = vec![first];
        while event::poll(std::time::Duration::ZERO)? {
            events.push(event::read()?);
        }

        for ev in events {
            match ev {
                Event::Key(key) if key.kind == KeyEventKind::Press => {
                    // Clear transient status message on any keypress
                    app.status_message = None;

                    // Handle help modal if shown
                    if app.show_help_modal {
                        match key.code {
                            KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q') => {
                                app.show_help_modal = false;
                            }
                            _ => {}
                        }
                        continue;
                    }

                    // Handle rebase modal if shown
                    if app.show_rebase_modal {
                        match key.code {
                            KeyCode::Char('r') => {
                                // Get upstream branch
                                if let Ok(output) = Command::new("git")
                                    .args(["rev-parse", "--abbrev-ref", "HEAD@{u}"])
                                    .output()
                                {
                                    if output.status.success() {
                                        let upstream = String::from_utf8_lossy(&output.stdout)
                                            .trim()
                                            .to_string();
                                        // Perform rebase
                                        match diff::perform_rebase(&upstream) {
                                            Ok(success) => {
                                                if success {
                                                    // Rebase successful — exit so the user
                                                    // can re-run with fresh diff data.
                                                    app.show_rebase_modal = false;
                                                    return Ok(true);
                                                } else {
                                                    app.rebase_notification = Some(
                                                    "Rebase failed. There might be conflicts to resolve.".to_string()
                                                );
                                                }
                                            }
                                            Err(e) => {
                                                app.rebase_notification =
                                                    Some(format!("Error during rebase: {}", e));
                                            }
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('i') => {
                                // Ignore rebase suggestion
                                app.show_rebase_modal = false;
                            }
                            KeyCode::Esc => {
                                // Dismiss modal
                                app.show_rebase_modal = false;
                            }
                            _ => {}
                        }
                        continue; // Skip other key processing when modal is shown
                    }
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            match app.app_mode {
                                AppMode::Diff => return Ok(false),
                                AppMode::Rebase => {
                                    // Return to diff mode without applying changes
                                    app.app_mode = AppMode::Diff;
                                }
                            }
                        }
                        KeyCode::Char('r') => {
                            if let AppMode::Diff = app.app_mode {
                                app.app_mode = AppMode::Rebase;
                                prepare_rebase_changes(&mut app);
                            }
                        }
                        KeyCode::Char('a') => {
                            if let AppMode::Rebase = app.app_mode {
                                if let Some(file) = app.file_names.get(app.current_file_idx) {
                                    if let Some(changes) = app.rebase_changes.get_mut(file) {
                                        if app.current_change_idx < changes.len() {
                                            changes[app.current_change_idx].state =
                                                ChangeState::Accepted;
                                            // Auto-advance to next change
                                            if app.current_change_idx < changes.len() - 1 {
                                                app.current_change_idx += 1;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Char('x') => {
                            if let AppMode::Rebase = app.app_mode {
                                if let Some(file) = app.file_names.get(app.current_file_idx) {
                                    if let Some(changes) = app.rebase_changes.get_mut(file) {
                                        if app.current_change_idx < changes.len() {
                                            changes[app.current_change_idx].state =
                                                ChangeState::Rejected;
                                            // Auto-advance to next change
                                            if app.current_change_idx < changes.len() - 1 {
                                                app.current_change_idx += 1;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Char('c') => {
                            if let AppMode::Rebase = app.app_mode {
                                // Commit rebase changes
                                let mut any_applied = false;
                                let mut errors = Vec::new();

                                for (file, changes) in &app.rebase_changes {
                                    let mut operations = Vec::new();

                                    for change in changes {
                                        if change.state != ChangeState::Accepted {
                                            continue;
                                        }

                                        if change.is_base {
                                            if let Some(paired_content) = &change.paired_content {
                                                // Replace: swap old content with incoming content
                                                let clean = paired_content
                                                    .strip_prefix('+')
                                                    .unwrap_or(paired_content);
                                                operations.push(diff::ChangeOp::Replace(
                                                    change.line_num,
                                                    clean.to_string(),
                                                ));
                                            } else {
                                                // Delete: remove the line entirely
                                                operations
                                                    .push(diff::ChangeOp::Delete(change.line_num));
                                            }
                                        } else {
                                            // Insert: use computed base position
                                            let clean = change
                                                .content
                                                .strip_prefix('+')
                                                .unwrap_or(&change.content);
                                            let base_pos =
                                                change.base_insert_pos.unwrap_or(change.line_num);
                                            operations.push(diff::ChangeOp::Insert {
                                                base_pos,
                                                order: change.line_num,
                                                content: clean.to_string(),
                                            });
                                        }
                                    }

                                    if !operations.is_empty() {
                                        any_applied = true;
                                        if let Err(e) = diff::apply_changes(file, &operations) {
                                            errors.push(format!("{}: {}", file, e));
                                        }
                                    }
                                }

                                // Surface feedback through the UI
                                if !errors.is_empty() {
                                    app.status_message =
                                        Some(format!("Error: {}", errors.join("; ")));
                                } else if any_applied {
                                    app.status_message =
                                        Some("Changes applied successfully!".to_string());
                                } else {
                                    app.status_message =
                                        Some("No accepted changes to apply.".to_string());
                                }

                                // Return to diff mode
                                app.app_mode = AppMode::Diff;
                            }
                        }
                        KeyCode::Char('j') | KeyCode::Down => match app.app_mode {
                            AppMode::Diff => match app.focused_pane {
                                Pane::FileList => {
                                    if app.current_file_idx + 1 < app.file_names.len() {
                                        app.current_file_idx += 1;
                                    }
                                }
                                Pane::DiffContent => {
                                    if let Some(file) = app.file_names.get(app.current_file_idx) {
                                        let scroll = *app.scroll_positions.get(file).unwrap_or(&0);
                                        app.scroll_positions.insert(file.clone(), scroll + 1);
                                    }
                                }
                            },
                            AppMode::Rebase => {
                                if let Some(file) = app.file_names.get(app.current_file_idx) {
                                    if let Some(changes) = app.rebase_changes.get(file) {
                                        if !changes.is_empty()
                                            && app.current_change_idx < changes.len() - 1
                                        {
                                            app.current_change_idx += 1;
                                        }
                                    }
                                }
                            }
                        },
                        KeyCode::Char('k') | KeyCode::Up => match app.app_mode {
                            AppMode::Diff => match app.focused_pane {
                                Pane::FileList => {
                                    if app.current_file_idx > 0 {
                                        app.current_file_idx -= 1;
                                    }
                                }
                                Pane::DiffContent => {
                                    if let Some(file) = app.file_names.get(app.current_file_idx) {
                                        let scroll = *app.scroll_positions.get(file).unwrap_or(&0);
                                        if scroll > 0 {
                                            app.scroll_positions.insert(file.clone(), scroll - 1);
                                        }
                                    }
                                }
                            },
                            AppMode::Rebase => {
                                if app.current_change_idx > 0 {
                                    app.current_change_idx -= 1;
                                }
                            }
                        },
                        KeyCode::PageDown => match app.app_mode {
                            AppMode::Diff => match app.focused_pane {
                                Pane::FileList => {
                                    let page = terminal.size()?.height.saturating_sub(6) as usize;
                                    app.current_file_idx = (app.current_file_idx + page)
                                        .min(app.file_names.len().saturating_sub(1));
                                }
                                Pane::DiffContent => {
                                    if let Some(file) = app.file_names.get(app.current_file_idx) {
                                        let scroll = *app.scroll_positions.get(file).unwrap_or(&0);
                                        let page = terminal.size()?.height.saturating_sub(6);
                                        app.scroll_positions
                                            .insert(file.clone(), scroll.saturating_add(page));
                                    }
                                }
                            },
                            AppMode::Rebase => {
                                if let Some(file) = app.file_names.get(app.current_file_idx) {
                                    if let Some(changes) = app.rebase_changes.get(file) {
                                        if !changes.is_empty() {
                                            let page =
                                                terminal.size()?.height.saturating_sub(6) as usize;
                                            app.current_change_idx = (app.current_change_idx
                                                + page)
                                                .min(changes.len() - 1);
                                        }
                                    }
                                }
                            }
                        },
                        KeyCode::PageUp => match app.app_mode {
                            AppMode::Diff => match app.focused_pane {
                                Pane::FileList => {
                                    let page = terminal.size()?.height.saturating_sub(6) as usize;
                                    app.current_file_idx =
                                        app.current_file_idx.saturating_sub(page);
                                }
                                Pane::DiffContent => {
                                    if let Some(file) = app.file_names.get(app.current_file_idx) {
                                        let scroll = *app.scroll_positions.get(file).unwrap_or(&0);
                                        let page = terminal.size()?.height.saturating_sub(6);
                                        app.scroll_positions
                                            .insert(file.clone(), scroll.saturating_sub(page));
                                    }
                                }
                            },
                            AppMode::Rebase => {
                                let page = terminal.size()?.height.saturating_sub(6) as usize;
                                app.current_change_idx =
                                    app.current_change_idx.saturating_sub(page);
                            }
                        },
                        KeyCode::Home => match app.app_mode {
                            AppMode::Diff => match app.focused_pane {
                                Pane::FileList => {
                                    app.current_file_idx = 0;
                                }
                                Pane::DiffContent => {
                                    if let Some(file) = app.file_names.get(app.current_file_idx) {
                                        app.scroll_positions.insert(file.clone(), 0);
                                    }
                                }
                            },
                            AppMode::Rebase => {
                                app.current_change_idx = 0;
                            }
                        },
                        KeyCode::End => match app.app_mode {
                            AppMode::Diff => match app.focused_pane {
                                Pane::FileList => {
                                    app.current_file_idx = app.file_names.len().saturating_sub(1);
                                }
                                Pane::DiffContent => {
                                    if let Some(file) = app.file_names.get(app.current_file_idx) {
                                        app.scroll_positions.insert(file.clone(), u16::MAX);
                                    }
                                }
                            },
                            AppMode::Rebase => {
                                if let Some(file) = app.file_names.get(app.current_file_idx) {
                                    if let Some(changes) = app.rebase_changes.get(file) {
                                        if !changes.is_empty() {
                                            app.current_change_idx = changes.len() - 1;
                                        }
                                    }
                                }
                            }
                        },
                        KeyCode::Tab => {
                            // Toggle between file list and diff content (only in diff mode)
                            if let AppMode::Diff = app.app_mode {
                                app.focused_pane = match app.focused_pane {
                                    Pane::FileList => Pane::DiffContent,
                                    Pane::DiffContent => Pane::FileList,
                                }
                            }
                        }
                        KeyCode::Char('h') | KeyCode::Left => {
                            if let AppMode::Diff = app.app_mode {
                                app.focused_pane = Pane::FileList;
                            }
                        }
                        KeyCode::Char('l') | KeyCode::Right => {
                            if let AppMode::Diff = app.app_mode {
                                app.focused_pane = Pane::DiffContent;
                            }
                        }
                        KeyCode::Char('t') => {
                            // Toggle between dark and light theme
                            app.theme = if app.theme.is_dark {
                                Theme::light()
                            } else {
                                Theme::dark()
                            };
                        }
                        KeyCode::Char('u') => {
                            // Toggle between unified and side-by-side view (only in diff mode)
                            if let AppMode::Diff = app.app_mode {
                                app.view_mode = match app.view_mode {
                                    ViewMode::SideBySide => ViewMode::Unified,
                                    ViewMode::Unified => ViewMode::SideBySide,
                                }
                            }
                        }
                        KeyCode::Char('n') => {
                            // Navigate to next file with changes in rebase mode
                            if let AppMode::Rebase = app.app_mode {
                                let mut next_idx = app.current_file_idx;
                                let mut found = false;

                                // Look for the next file with changes
                                while next_idx + 1 < app.file_names.len() {
                                    next_idx += 1;
                                    if let Some(changes) =
                                        app.rebase_changes.get(&app.file_names[next_idx])
                                    {
                                        if !changes.is_empty() {
                                            app.current_file_idx = next_idx;
                                            app.current_change_idx = 0;
                                            found = true;
                                            break;
                                        }
                                    }
                                }

                                // If no more files with changes after current, loop to beginning
                                if !found {
                                    for (idx, file_name) in app.file_names.iter().enumerate() {
                                        if idx >= app.current_file_idx {
                                            continue;
                                        }

                                        if let Some(changes) = app.rebase_changes.get(file_name) {
                                            if !changes.is_empty() {
                                                app.current_file_idx = idx;
                                                app.current_change_idx = 0;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Char('p') => {
                            // Navigate to previous file with changes in rebase mode
                            if let AppMode::Rebase = app.app_mode {
                                let mut prev_idx = app.current_file_idx;
                                let mut found = false;

                                // Look for the previous file with changes
                                while prev_idx > 0 {
                                    prev_idx -= 1;
                                    if let Some(changes) =
                                        app.rebase_changes.get(&app.file_names[prev_idx])
                                    {
                                        if !changes.is_empty() {
                                            app.current_file_idx = prev_idx;
                                            app.current_change_idx = 0;
                                            found = true;
                                            break;
                                        }
                                    }
                                }

                                // If no more files with changes before current, loop to end
                                if !found {
                                    for (idx, file_name) in app.file_names.iter().enumerate().rev()
                                    {
                                        if idx <= app.current_file_idx {
                                            continue;
                                        }

                                        if let Some(changes) = app.rebase_changes.get(file_name) {
                                            if !changes.is_empty() {
                                                app.current_file_idx = idx;
                                                app.current_change_idx = 0;
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        KeyCode::Char('?') => {
                            app.show_help_modal = true;
                        }
                        _ => {}
                    }
                }
                Event::Mouse(mouse) => {
                    if app.show_help_modal || app.show_rebase_modal {
                        continue;
                    }
                    let size = terminal.size()?;
                    let scroll_amount: i16 = 3;
                    match mouse.kind {
                        MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                            if mouse.row == 0 || mouse.row >= size.height.saturating_sub(1) {
                                continue;
                            }
                            let delta = if matches!(mouse.kind, MouseEventKind::ScrollDown) {
                                scroll_amount
                            } else {
                                -scroll_amount
                            };
                            match app.app_mode {
                                AppMode::Diff => {
                                    let file_list_width = size.width / 5;
                                    if mouse.column < file_list_width {
                                        if !app.file_names.is_empty() {
                                            if delta > 0 {
                                                app.current_file_idx = (app.current_file_idx
                                                    + delta as usize)
                                                    .min(app.file_names.len() - 1);
                                            } else {
                                                app.current_file_idx = app
                                                    .current_file_idx
                                                    .saturating_sub((-delta) as usize);
                                            }
                                        }
                                    } else if let Some(file) =
                                        app.file_names.get(app.current_file_idx)
                                    {
                                        let scroll = *app.scroll_positions.get(file).unwrap_or(&0);
                                        if delta > 0 {
                                            app.scroll_positions.insert(
                                                file.clone(),
                                                scroll.saturating_add(delta as u16),
                                            );
                                        } else {
                                            app.scroll_positions.insert(
                                                file.clone(),
                                                scroll.saturating_sub((-delta) as u16),
                                            );
                                        }
                                    }
                                }
                                AppMode::Rebase => {
                                    if let Some(file) = app.file_names.get(app.current_file_idx) {
                                        if let Some(changes) = app.rebase_changes.get(file) {
                                            if !changes.is_empty() {
                                                if delta > 0 {
                                                    app.current_change_idx =
                                                        (app.current_change_idx + delta as usize)
                                                            .min(changes.len() - 1);
                                                } else {
                                                    app.current_change_idx = app
                                                        .current_change_idx
                                                        .saturating_sub((-delta) as usize);
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        } // end event batch
    }
}
