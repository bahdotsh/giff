use crate::diff::{FileChanges, LineChange};
use comfy_table::{
    presets::UTF8_FULL_CONDENSED, Cell, CellAlignment, Color, ContentArrangement, Table,
};
use crossterm::{
    execute,
    terminal::{self, ClearType},
};
use std::error::Error;
use std::io::{self, Write};

pub fn show_diff_table(file_changes: &FileChanges, branch: &str) -> Result<(), Box<dyn Error>> {
    // Clear terminal
    let mut stdout = io::stdout();
    execute!(stdout, terminal::Clear(ClearType::All))?;

    // Create table
    let mut table = create_table(branch);

    // Add data
    populate_table(&mut table, file_changes);

    // Display
    println!("{}", table.trim_fmt());
    stdout.flush()?;

    Ok(())
}

fn create_table(branch: &str) -> Table {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::DynamicFullWidth);
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(vec![
        Cell::new("File").set_alignment(CellAlignment::Center),
        Cell::new(branch).set_alignment(CellAlignment::Center),
        Cell::new("HEAD").set_alignment(CellAlignment::Center),
    ]);

    table
}

fn populate_table(table: &mut Table, file_changes: &FileChanges) {
    for (file, (base_lines, head_lines)) in file_changes {
        // Add file header
        table.add_row(vec![Cell::new(file), Cell::new(""), Cell::new("")]);

        // Format cells
        let base_cells = format_line_cells(base_lines);
        let head_cells = format_line_cells(head_lines);

        // Add content rows
        let max_len = base_cells.len().max(head_cells.len());

        for i in 0..max_len {
            let base_cell = if i < base_cells.len() {
                base_cells[i].clone()
            } else {
                Cell::new("")
            };

            let head_cell = if i < head_cells.len() {
                head_cells[i].clone()
            } else {
                Cell::new("")
            };

            table.add_row(vec![Cell::new(""), base_cell, head_cell]);
        }
    }
}

fn format_line_cells(lines: &[LineChange]) -> Vec<Cell> {
    lines
        .iter()
        .map(|(num, line)| {
            let mut cell = Cell::new(format!("{} {}", num, line));
            if line.starts_with('-') {
                cell = cell.fg(Color::Red);
            } else if line.starts_with('+') {
                cell = cell.fg(Color::Green);
            }
            cell
        })
        .collect()
}
