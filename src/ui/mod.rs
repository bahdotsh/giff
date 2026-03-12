mod event_loop;
mod rebase;
mod render;
mod syntax;
mod types;

use crate::diff::{self, FileChanges};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, Terminal};
use std::collections::HashMap;
use std::{error::Error, io};

use event_loop::run_ui;
use types::*;

pub fn run_app(
    file_changes: FileChanges,
    left_label: &str,
    right_label: &str,
) -> Result<(), Box<dyn Error>> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let file_names: Vec<String> = file_changes.keys().cloned().collect();
    let file_names_sorted = {
        let mut names = file_names.clone();
        names.sort();
        names
    };

    let mut scroll_positions = HashMap::new();
    for name in &file_names_sorted {
        scroll_positions.insert(name.clone(), 0);
    }

    // Check if rebase is needed
    let rebase_notification = diff::check_rebase_needed()?;

    let app = App {
        file_changes: &file_changes,
        left_label,
        right_label,
        current_file_idx: 0,
        file_names: file_names_sorted,
        scroll_positions,
        focused_pane: Pane::FileList,
        view_mode: ViewMode::SideBySide,
        app_mode: AppMode::Diff,
        rebase_changes: HashMap::new(),
        current_change_idx: 0,
        rebase_notification: rebase_notification.clone(),
        show_rebase_modal: rebase_notification.is_some(),
        status_message: None,
    };

    // Run the main loop
    let res = run_ui(&mut terminal, app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    match res {
        Ok(true) => {
            println!("Rebase completed successfully. Please re-run giff to see updated changes.");
        }
        Err(err) => {
            println!("{:?}", err);
        }
        _ => {}
    }

    Ok(())
}
