use clap::Parser;
use comfy_table::{presets::UTF8_FULL_CONDENSED, Table};
use comfy_table::{Cell, Color};
use crossterm::{
    execute,
    terminal::{self, ClearType},
};
use regex::Regex;
use std::collections::HashMap;
use std::io::{self};
use std::process::Command;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "main")]
    branch: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Execute git diff command
    let output = Command::new("git")
        .args(&["diff", &format!("{}..HEAD", args.branch)])
        .output()?;

    if !output.status.success() {
        eprintln!("Failed to execute git diff command");
        std::process::exit(1);
    }

    let diff_output = String::from_utf8_lossy(&output.stdout);

    // Clear the terminal
    let mut stdout = io::stdout();
    execute!(stdout, terminal::Clear(ClearType::All))?;

    // Create and configure the table
    let mut table = Table::new();
    table.set_content_arrangement(comfy_table::ContentArrangement::DynamicFullWidth);
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(vec![
        Cell::new("File").set_alignment(comfy_table::CellAlignment::Center),
        Cell::new(args.branch.as_str()).set_alignment(comfy_table::CellAlignment::Center),
        Cell::new("HEAD").set_alignment(comfy_table::CellAlignment::Center),
    ]);

    // Regex to detect diff file headers and hunk headers
    let diff_file_regex = Regex::new(r"^diff --git a/(.+) b/(.+)$").unwrap();
    let hunk_header_regex = Regex::new(r"^@@ -(\d+),\d+ \+(\d+),\d+ @@").unwrap();
    let mut file_changes: HashMap<String, (Vec<(usize, String)>, Vec<(usize, String)>)> =
        HashMap::new();
    let mut current_file = String::new();
    let mut base_lines = Vec::new();
    let mut head_lines = Vec::new();
    let mut base_line_number = 1;
    let mut head_line_number = 1;

    // Parse and accumulate diff output
    let ansi_escape_regex = Regex::new(r"\x1b\[.*?m").unwrap();

    for line in diff_output.lines() {
        let trimmed_line = line.trim();
        let trimmed_line = ansi_escape_regex.replace_all(trimmed_line, "");

        if let Some(caps) = diff_file_regex.captures(trimmed_line.as_ref()) {
            if !current_file.is_empty() {
                file_changes.insert(
                    current_file.clone(),
                    (base_lines.clone(), head_lines.clone()),
                );
                base_lines.clear();
                head_lines.clear();
            }
            current_file = caps.get(1).unwrap().as_str().to_string();
            base_line_number = 1;
            head_line_number = 1;
            continue;
        }

        if let Some(caps) = hunk_header_regex.captures(trimmed_line.as_ref()) {
            base_line_number = caps.get(1).unwrap().as_str().parse::<usize>().unwrap();
            head_line_number = caps.get(2).unwrap().as_str().parse::<usize>().unwrap();
            continue;
        }

        if trimmed_line.starts_with("index")
            || trimmed_line.starts_with("---")
            || trimmed_line.starts_with("+++")
            || trimmed_line.starts_with("@@")
            || trimmed_line.starts_with("new")
        {
            continue;
        }

        if trimmed_line.starts_with('-') {
            base_lines.push((base_line_number, trimmed_line.to_string()));
            base_line_number += 1;
        } else if trimmed_line.starts_with('+') {
            head_lines.push((head_line_number, trimmed_line.to_string()));
            head_line_number += 1;
        } else {
            base_lines.push((base_line_number, trimmed_line.to_string()));
            head_lines.push((head_line_number, trimmed_line.to_string()));
            base_line_number += 1;
            head_line_number += 1;
        }
    }

    // Insert last file changes
    if !current_file.is_empty() {
        file_changes.insert(current_file.clone(), (base_lines, head_lines));
    }

    // Add rows to the table
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

    // Print the table
    println!("{}", table.trim_fmt());

    Ok(())
}
