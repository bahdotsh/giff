use crate::diff::FileChanges;
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

struct App<'a> {
    file_changes: &'a FileChanges,
    branch: &'a str,
    current_file_idx: usize,
    file_names: Vec<String>,
    scroll_positions: HashMap<String, u16>, // Single scroll position for both panes
    focused_pane: Pane,
    view_mode: ViewMode,
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

fn run_ui<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('j') | KeyCode::Down => match app.focused_pane {
                        Pane::FileList => {
                            if app.current_file_idx < app.file_names.len() - 1 {
                                app.current_file_idx += 1;
                            }
                        }
                        Pane::DiffContent => {
                            if let Some(file) = app.file_names.get(app.current_file_idx) {
                                let scroll = app.scroll_positions.get(file).unwrap_or(&0).clone();
                                app.scroll_positions.insert(file.clone(), scroll + 1);
                            }
                        }
                    },
                    KeyCode::Char('k') | KeyCode::Up => match app.focused_pane {
                        Pane::FileList => {
                            if app.current_file_idx > 0 {
                                app.current_file_idx -= 1;
                            }
                        }
                        Pane::DiffContent => {
                            if let Some(file) = app.file_names.get(app.current_file_idx) {
                                let scroll = app.scroll_positions.get(file).unwrap_or(&0).clone();
                                if scroll > 0 {
                                    app.scroll_positions.insert(file.clone(), scroll - 1);
                                }
                            }
                        }
                    },
                    KeyCode::Tab => {
                        // Toggle between file list and diff content
                        app.focused_pane = match app.focused_pane {
                            Pane::FileList => Pane::DiffContent,
                            Pane::DiffContent => Pane::FileList,
                        }
                    }
                    KeyCode::Char('h') | KeyCode::Left => {
                        app.focused_pane = Pane::FileList;
                    }
                    KeyCode::Char('l') | KeyCode::Right => {
                        app.focused_pane = Pane::DiffContent;
                    }
                    KeyCode::Char('u') => {
                        // Toggle between unified and side-by-side view
                        app.view_mode = match app.view_mode {
                            ViewMode::SideBySide => ViewMode::Unified,
                            ViewMode::Unified => ViewMode::SideBySide,
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

    // Create main layout with 3 parts: file list, base diff, head diff
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Help
        ])
        .split(size);

    // Create header with title and controls
    render_header(f, app, main_chunks[0]);

    // Content area layout depends on view mode
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

fn render_help(f: &mut Frame, app: &App, area: Rect) {
    let help_text = match app.view_mode {
        ViewMode::SideBySide => "Esc/q: Quit | j/k: Navigate | Tab: Change focus | h/l: Switch panes | u: Toggle unified view",
        ViewMode::Unified => "Esc/q: Quit | j/k: Navigate | Tab: Change focus | h/l: Switch panes | u: Toggle side-by-side view",
    };

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Center)
        .block(Block::default().borders(Borders::ALL));
    f.render_widget(help, area);
}
