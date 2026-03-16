mod event_loop;
mod rebase;
pub(crate) mod render;
mod syntax;
pub(crate) mod theme;
mod types;

#[cfg(test)]
mod tests;

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

/// Restore the terminal to its normal state. Best-effort: errors are ignored
/// because this is typically called during cleanup or panic recovery.
fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
}

pub fn run_app(
    file_changes: FileChanges,
    left_label: &str,
    right_label: &str,
    theme: theme::Theme,
) -> Result<(), Box<dyn Error>> {
    // Install a panic hook that restores the terminal before printing the
    // panic message. Without this, a panic leaves the terminal in raw mode
    // with the alternate screen still active, making it unusable.
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        original_hook(info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create app state
    let mut file_names: Vec<String> = file_changes.keys().cloned().collect();
    file_names.sort();

    let mut scroll_positions = HashMap::new();
    for name in &file_names {
        scroll_positions.insert(name.clone(), 0);
    }

    // Check if rebase is needed
    let rebase_notification = diff::check_rebase_needed()?;

    let app = App {
        file_changes: &file_changes,
        left_label,
        right_label,
        current_file_idx: 0,
        file_names,
        scroll_positions,
        focused_pane: Pane::FileList,
        view_mode: ViewMode::SideBySide,
        app_mode: AppMode::Diff,
        rebase_changes: HashMap::new(),
        current_change_idx: 0,
        rebase_notification: rebase_notification.clone(),
        show_rebase_modal: rebase_notification.is_some(),
        status_message: None,
        show_help_modal: false,
        theme,
    };

    // Run the main loop
    let res = run_ui(&mut terminal, app);

    // Restore terminal
    restore_terminal();
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
