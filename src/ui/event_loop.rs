use crate::diff;
use crossterm::event::{self, Event, KeyCode, KeyEventKind, MouseEventKind};
use ratatui::{prelude::*, Terminal};
use std::io;

use super::rebase::prepare_rebase_changes;
use super::render::ui;
use super::types::*;

fn commit_rebase_changes(app: &mut App) {
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
                    let clean = paired_content.strip_prefix('+').unwrap_or(paired_content);
                    operations.push(diff::ChangeOp::Replace(change.line_num, clean.to_string()));
                } else {
                    // Delete: remove the line entirely
                    operations.push(diff::ChangeOp::Delete(change.line_num));
                }
            } else {
                // Insert: use computed base position
                let clean = change.content.strip_prefix('+').unwrap_or(&change.content);
                let base_pos = change.base_insert_pos.unwrap_or(change.line_num);
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
        app.status_message = Some(format!("Error: {}", errors.join("; ")));
    } else if any_applied {
        app.status_message = Some("Changes applied successfully!".to_string());
    } else {
        app.status_message = Some("No accepted changes to apply.".to_string());
    }

    // Return to diff mode
    app.app_mode = AppMode::Diff;
}

fn set_change_state(app: &mut App, state: ChangeState) {
    if let Some(file) = app.file_names.get(app.current_file_idx) {
        if let Some(changes) = app.rebase_changes.get_mut(file) {
            if app.current_change_idx < changes.len() {
                changes[app.current_change_idx].state = state;
                if app.current_change_idx < changes.len() - 1 {
                    app.current_change_idx += 1;
                }
            }
        }
    }
}

fn navigate_rebase_file(app: &mut App, forward: bool) {
    let len = app.file_names.len();
    if len == 0 {
        return;
    }
    for offset in 1..len {
        let idx = if forward {
            (app.current_file_idx + offset) % len
        } else {
            (app.current_file_idx + len - offset) % len
        };
        if let Some(changes) = app.rebase_changes.get(&app.file_names[idx]) {
            if !changes.is_empty() {
                app.current_file_idx = idx;
                app.current_change_idx = 0;
                return;
            }
        }
    }
}

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
                            KeyCode::Char('r') => match diff::get_upstream_branch() {
                                Ok(Some(upstream)) => match diff::perform_rebase(&upstream) {
                                    Ok(true) => {
                                        app.show_rebase_modal = false;
                                        return Ok(true);
                                    }
                                    Ok(false) => {
                                        app.rebase_notification = Some(
                                            "Rebase failed due to conflicts and was rolled back."
                                                .to_string(),
                                        );
                                    }
                                    Err(e) => {
                                        app.show_rebase_modal = false;
                                        app.status_message = Some(format!("Error: {}", e));
                                    }
                                },
                                Ok(None) => {
                                    app.show_rebase_modal = false;
                                    app.status_message =
                                        Some("No upstream branch configured.".to_string());
                                }
                                Err(e) => {
                                    app.show_rebase_modal = false;
                                    app.status_message = Some(format!("Error: {}", e));
                                }
                            },
                            KeyCode::Char('i') | KeyCode::Esc => {
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
                                set_change_state(&mut app, ChangeState::Accepted);
                            }
                        }
                        KeyCode::Char('x') => {
                            if let AppMode::Rebase = app.app_mode {
                                set_change_state(&mut app, ChangeState::Rejected);
                            }
                        }
                        KeyCode::Char('c') => {
                            if let AppMode::Rebase = app.app_mode {
                                commit_rebase_changes(&mut app);
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
                                        let page =
                                            terminal.size()?.height.saturating_sub(6) as usize;
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
                                        let page =
                                            terminal.size()?.height.saturating_sub(6) as usize;
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
                                        app.scroll_positions.insert(file.clone(), usize::MAX);
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
                            // Cycle through available themes
                            if !app.theme_cycle.is_empty() {
                                app.theme_cycle_idx =
                                    (app.theme_cycle_idx + 1) % app.theme_cycle.len();
                                app.theme = app.theme_cycle[app.theme_cycle_idx].clone();
                            }
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
                            if let AppMode::Rebase = app.app_mode {
                                navigate_rebase_file(&mut app, true);
                            }
                        }
                        KeyCode::Char('p') => {
                            if let AppMode::Rebase = app.app_mode {
                                navigate_rebase_file(&mut app, false);
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
                    let scroll_amount: usize = 3;
                    match mouse.kind {
                        MouseEventKind::ScrollDown | MouseEventKind::ScrollUp => {
                            if mouse.row == 0 || mouse.row >= size.height.saturating_sub(1) {
                                continue;
                            }
                            let is_down = matches!(mouse.kind, MouseEventKind::ScrollDown);
                            match app.app_mode {
                                AppMode::Diff => {
                                    let file_list_width = size.width / 5;
                                    if mouse.column < file_list_width {
                                        if !app.file_names.is_empty() {
                                            if is_down {
                                                app.current_file_idx = (app.current_file_idx
                                                    + scroll_amount)
                                                    .min(app.file_names.len() - 1);
                                            } else {
                                                app.current_file_idx = app
                                                    .current_file_idx
                                                    .saturating_sub(scroll_amount);
                                            }
                                        }
                                    } else if let Some(file) =
                                        app.file_names.get(app.current_file_idx)
                                    {
                                        let scroll = *app.scroll_positions.get(file).unwrap_or(&0);
                                        let new_scroll = if is_down {
                                            scroll.saturating_add(scroll_amount)
                                        } else {
                                            scroll.saturating_sub(scroll_amount)
                                        };
                                        app.scroll_positions.insert(file.clone(), new_scroll);
                                    }
                                }
                                AppMode::Rebase => {
                                    if let Some(file) = app.file_names.get(app.current_file_idx) {
                                        if let Some(changes) = app.rebase_changes.get(file) {
                                            if !changes.is_empty() {
                                                if is_down {
                                                    app.current_change_idx =
                                                        (app.current_change_idx + scroll_amount)
                                                            .min(changes.len() - 1);
                                                } else {
                                                    app.current_change_idx = app
                                                        .current_change_idx
                                                        .saturating_sub(scroll_amount);
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

#[cfg(test)]
mod tests {
    use super::super::theme::Theme;
    use super::*;
    use crate::diff::FileChanges;
    use std::collections::HashMap;

    fn make_app(file_names: Vec<&str>, changes_for: Vec<&str>) -> App<'static> {
        let file_names: Vec<String> = file_names.into_iter().map(|s| s.to_string()).collect();
        let mut rebase_changes = HashMap::new();
        for name in &file_names {
            let changes = if changes_for.contains(&name.as_str()) {
                vec![Change {
                    line_num: 1,
                    content: "-old".to_string(),
                    paired_content: None,
                    state: ChangeState::Unselected,
                    is_base: true,
                    context: vec![],
                    base_insert_pos: None,
                }]
            } else {
                vec![]
            };
            rebase_changes.insert(name.clone(), changes);
        }
        // App borrows file_changes, but we only need rebase navigation,
        // so leak an empty map to satisfy the lifetime.
        let file_changes: &'static FileChanges = Box::leak(Box::new(HashMap::new()));
        App {
            file_changes,
            left_label: "",
            right_label: "",
            current_file_idx: 0,
            file_names,
            scroll_positions: HashMap::new(),
            focused_pane: Pane::FileList,
            view_mode: ViewMode::SideBySide,
            app_mode: AppMode::Rebase,
            rebase_changes,
            current_change_idx: 0,
            rebase_notification: None,
            show_rebase_modal: false,
            status_message: None,
            show_help_modal: false,
            theme: Theme::dark(),
            theme_cycle: vec![Theme::dark(), Theme::light()],
            theme_cycle_idx: 0,
        }
    }

    #[test]
    fn navigate_forward_finds_next_file_with_changes() {
        let mut app = make_app(vec!["a.rs", "b.rs", "c.rs"], vec!["a.rs", "c.rs"]);
        app.current_file_idx = 0;
        navigate_rebase_file(&mut app, true);
        assert_eq!(app.current_file_idx, 2); // skips b.rs (empty)
    }

    #[test]
    fn navigate_forward_wraps_around() {
        let mut app = make_app(vec!["a.rs", "b.rs", "c.rs"], vec!["a.rs", "c.rs"]);
        app.current_file_idx = 2;
        navigate_rebase_file(&mut app, true);
        assert_eq!(app.current_file_idx, 0); // wraps to a.rs
    }

    #[test]
    fn navigate_backward_finds_previous_file_with_changes() {
        let mut app = make_app(vec!["a.rs", "b.rs", "c.rs"], vec!["a.rs", "c.rs"]);
        app.current_file_idx = 2;
        navigate_rebase_file(&mut app, false);
        assert_eq!(app.current_file_idx, 0); // skips b.rs
    }

    #[test]
    fn navigate_backward_wraps_around() {
        let mut app = make_app(vec!["a.rs", "b.rs", "c.rs"], vec!["a.rs", "c.rs"]);
        app.current_file_idx = 0;
        navigate_rebase_file(&mut app, false);
        assert_eq!(app.current_file_idx, 2); // wraps to c.rs
    }

    #[test]
    fn navigate_no_files_with_changes_stays_put() {
        let mut app = make_app(vec!["a.rs", "b.rs"], vec![]);
        app.current_file_idx = 0;
        navigate_rebase_file(&mut app, true);
        assert_eq!(app.current_file_idx, 0); // unchanged
    }

    #[test]
    fn navigate_single_file_with_changes_stays_put() {
        let mut app = make_app(vec!["a.rs", "b.rs"], vec!["a.rs"]);
        app.current_file_idx = 0;
        navigate_rebase_file(&mut app, true);
        assert_eq!(app.current_file_idx, 0); // only file with changes
    }

    #[test]
    fn navigate_empty_file_list() {
        let mut app = make_app(vec![], vec![]);
        navigate_rebase_file(&mut app, true);
        assert_eq!(app.current_file_idx, 0);
    }

    #[test]
    fn navigate_resets_change_idx() {
        let mut app = make_app(vec!["a.rs", "b.rs"], vec!["a.rs", "b.rs"]);
        app.current_file_idx = 0;
        app.current_change_idx = 5;
        navigate_rebase_file(&mut app, true);
        assert_eq!(app.current_file_idx, 1);
        assert_eq!(app.current_change_idx, 0);
    }
}
