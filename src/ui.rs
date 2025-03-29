use crate::diff::{self, FileChanges};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::collections::HashMap;
use std::{error::Error, io};

enum AppMode {
    Diff,
    Rebase,
}

#[derive(Clone, PartialEq)]
enum ChangeState {
    Unselected,
    Accepted,
    Rejected,
}

#[derive(Clone, PartialEq)]
struct Change {
    line_num: usize,
    content: String,
    paired_content: Option<String>, // The paired line (if any)
    state: ChangeState,
    is_base: bool,
    context: Vec<String>,
}

struct App<'a> {
    file_changes: &'a FileChanges,
    branch: &'a str,
    current_file_idx: usize,
    file_names: Vec<String>,
    scroll_positions: HashMap<String, u16>,
    focused_pane: Pane,
    view_mode: ViewMode,
    app_mode: AppMode,
    rebase_changes: HashMap<String, Vec<Change>>,
    current_change_idx: usize,
}

enum Pane {
    FileList,
    DiffContent,
}

enum ViewMode {
    SideBySide,
    Unified,
}

pub fn run_app(file_changes: FileChanges, branch: &str) -> Result<(), Box<dyn Error>> {
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

    let app = App {
        file_changes: &file_changes,
        branch,
        current_file_idx: 0,
        file_names: file_names_sorted,
        scroll_positions,
        focused_pane: Pane::FileList,
        view_mode: ViewMode::SideBySide,
        app_mode: AppMode::Diff,
        rebase_changes: HashMap::new(),
        current_change_idx: 0,
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

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

fn prepare_rebase_changes(app: &mut App) {
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

            // Try to match lines - this is a simple approach
            // For more sophisticated matching, you'd need a diff algorithm
            for (base_num, base_line) in &base_map {
                let _base_content = base_line.strip_prefix('-').unwrap_or(base_line);

                // Try to find a matching added line with similar content
                for (head_num, head_line) in &head_map {
                    let _head_content = head_line.strip_prefix('+').unwrap_or(head_line);

                    // If line numbers are close and content is similar - pair them
                    // This is a very simplistic approach and might need refinement
                    if (*head_num as isize - *base_num as isize).abs() < 5 {
                        paired_changes.insert(*base_num, *head_num);
                        break;
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
                    });
                }
            }

            // Add added lines from head that weren't paired
            for (line_num, line) in head_lines {
                if line.starts_with('+') && !paired_changes.values().any(|num| num == line_num) {
                    let context = get_context(head_lines, *line_num);
                    changes.push(Change {
                        line_num: *line_num,
                        content: line.clone(),
                        paired_content: None,
                        state: ChangeState::Unselected,
                        is_base: false,
                        context,
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

fn run_ui<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => {
                        match app.app_mode {
                            AppMode::Diff => return Ok(()),
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
                            let mut any_changes_applied = false;

                            for (file, changes) in &app.rebase_changes {
                                let mut changes_to_apply = Vec::new();

                                for change in changes {
                                    if change.state == ChangeState::Accepted {
                                        if change.is_base {
                                            // For removed lines that were accepted, we want to apply
                                            // the paired content (if available) or remove the line
                                            if let Some(paired_content) = &change.paired_content {
                                                // Apply the paired content
                                                let clean_content = paired_content
                                                    .strip_prefix('+')
                                                    .unwrap_or(paired_content);

                                                changes_to_apply.push((
                                                    change.line_num,
                                                    clean_content.to_string(),
                                                    true,
                                                ));
                                            } else {
                                                // Just mark the line for removal
                                                changes_to_apply.push((
                                                    change.line_num,
                                                    change.content.clone(),
                                                    true,
                                                ));
                                            }
                                        } else {
                                            // For added lines, apply normally
                                            changes_to_apply.push((
                                                change.line_num,
                                                change.content.clone(),
                                                true,
                                            ));
                                        }
                                    }
                                }

                                if !changes_to_apply.is_empty() {
                                    any_changes_applied = true;
                                    if let Err(e) = diff::apply_changes(file, &changes_to_apply) {
                                        // Handle error (could add a status message to the UI)
                                        eprintln!("Error applying changes to {}: {}", file, e);
                                    }
                                }
                            }

                            // Show success message (this would be better with a status message in the UI)
                            if any_changes_applied {
                                // Could add a flash message here if the UI supported it
                            }

                            // Return to diff mode
                            app.app_mode = AppMode::Diff;
                        }
                    }
                    KeyCode::Char('j') | KeyCode::Down => match app.app_mode {
                        AppMode::Diff => match app.focused_pane {
                            Pane::FileList => {
                                if app.current_file_idx < app.file_names.len() - 1 {
                                    app.current_file_idx += 1;
                                }
                            }
                            Pane::DiffContent => {
                                if let Some(file) = app.file_names.get(app.current_file_idx) {
                                    let scroll =
                                        app.scroll_positions.get(file).unwrap_or(&0).clone();
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
                                    let scroll =
                                        app.scroll_positions.get(file).unwrap_or(&0).clone();
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
                            while next_idx < app.file_names.len() - 1 {
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
                                for (idx, file_name) in app.file_names.iter().enumerate().rev() {
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
                    _ => {}
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let size = f.size();

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
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let view_mode_text = match app.view_mode {
        ViewMode::SideBySide => "Side-by-Side",
        ViewMode::Unified => "Unified",
    };
    let title = format!(
        " giff - Comparing {} to HEAD [{}] ",
        app.branch, view_mode_text
    );
    let header = Paragraph::new(title)
        .style(Style::default().fg(Color::White).bg(Color::Blue))
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(header, area);
}

fn render_file_list(f: &mut Frame, app: &App, area: Rect) {
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

    let content = Text::from(
        base_lines
            .iter()
            .map(|(line_num, line)| {
                let color = if line.starts_with('-') {
                    Color::Red
                } else if line.starts_with('+') {
                    Color::Green
                } else {
                    Color::White
                };

                Line::from(Span::styled(
                    format!("{:4} {}", line_num, line),
                    Style::default().fg(color),
                ))
            })
            .collect::<Vec<Line>>(),
    );

    let base_paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(format!("{} ({})", app.branch, current_file))
                .borders(Borders::ALL),
        )
        .scroll((*scroll, 0));

    // Use different style if DiffContent is focused
    let base_paragraph = match app.focused_pane {
        Pane::DiffContent => base_paragraph.block(
            Block::default()
                .title(format!("{} ({})", app.branch, current_file))
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

    let content = Text::from(
        head_lines
            .iter()
            .map(|(line_num, line)| {
                let color = if line.starts_with('-') {
                    Color::Red
                } else if line.starts_with('+') {
                    Color::Green
                } else {
                    Color::White
                };

                Line::from(Span::styled(
                    format!("{:4} {}", line_num, line),
                    Style::default().fg(color),
                ))
            })
            .collect::<Vec<Line>>(),
    );

    let head_paragraph = Paragraph::new(content)
        .block(
            Block::default()
                .title(format!("HEAD ({})", current_file))
                .borders(Borders::ALL),
        )
        .scroll((*scroll, 0));

    // Use different style if DiffContent is focused
    let head_paragraph = match app.focused_pane {
        Pane::DiffContent => head_paragraph.block(
            Block::default()
                .title(format!("HEAD ({})", current_file))
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

    // Create unified diff by interleaving lines
    let mut unified_content = Vec::new();

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
                    unified_content.push(Line::from(Span::styled(
                        format!("{:4} {}", num, line),
                        Style::default().fg(if line.starts_with('+') {
                            Color::Green
                        } else {
                            Color::White
                        }),
                    )));
                    processed_lines.push(num);
                }
            }
        } else {
            // Find this line in base_lines
            if let Some((_, line)) = base_lines.iter().find(|(line_num, _)| *line_num == num) {
                if !line.starts_with('+') && !processed_lines.contains(&num) {
                    unified_content.push(Line::from(Span::styled(
                        format!("{:4} {}", num, line),
                        Style::default().fg(if line.starts_with('-') {
                            Color::Red
                        } else {
                            Color::White
                        }),
                    )));
                    processed_lines.push(num);
                }
            }
        }
    }

    let unified_paragraph = Paragraph::new(Text::from(unified_content))
        .block(
            Block::default()
                .title(format!(
                    "Unified Diff: {} vs HEAD ({})",
                    app.branch, current_file
                ))
                .borders(Borders::ALL),
        )
        .scroll((*scroll, 0));

    // Use different style if DiffContent is focused
    let unified_paragraph = match app.focused_pane {
        Pane::DiffContent => unified_paragraph.block(
            Block::default()
                .title(format!(
                    "Unified Diff: {} vs HEAD ({})",
                    app.branch, current_file
                ))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Yellow)),
        ),
        _ => unified_paragraph,
    };

    f.render_widget(unified_paragraph, area);
}

fn render_rebase_ui(f: &mut Frame, app: &App, area: Rect) {
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
    if !app.file_names.is_empty() {
        let current_file = &app.file_names[app.current_file_idx];

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

fn render_help(f: &mut Frame, app: &App, area: Rect) {
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
