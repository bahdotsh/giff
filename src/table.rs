use comfy_table::{Cell, Color, Table};
use std::collections::HashMap;

pub fn populate_table(
    table: &mut Table,
    file_changes: HashMap<String, (Vec<(usize, String)>, Vec<(usize, String)>)>,
) {
    for (file, (base_lines, head_lines)) in file_changes {
        let max_lines = base_lines.len().max(head_lines.len());

        // Add the file name row
        table.add_row(vec![file.clone(), "".to_string(), "".to_string()]);

        let base_cells: Vec<Cell> = base_lines
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
            .collect();

        let head_cells: Vec<Cell> = head_lines
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
            .collect();

        // Add a single row with the combined lines
        let max_len = max_lines;
        let mut base_cells_padded = base_cells.clone();
        let mut head_cells_padded = head_cells.clone();

        base_cells_padded.resize(max_len, Cell::new(""));
        head_cells_padded.resize(max_len, Cell::new(""));

        // Add rows to the table
        for i in 0..max_len {
            table.add_row(vec![
                Cell::new(""), // Placeholder for the first column
                base_cells_padded[i].clone(),
                head_cells_padded[i].clone(),
            ]);
        }
    }
}
