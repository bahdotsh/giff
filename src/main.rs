use clap::Parser;
use comfy_table::{presets::UTF8_FULL, Cell, Table};
use crossterm::style::Stylize;
use crossterm::{
    execute,
    terminal::{self, ClearType},
};
use std::io::{self, Write};
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
    terminal::enable_raw_mode()?;

    // Create and configure the table
    let mut table = Table::new();
    table.load_preset(UTF8_FULL);
    table.set_header(vec![args.branch.as_str(), "HEAD"]);

    // Parse and add diff output to the table
    let mut base_lines = Vec::new();
    let mut head_lines = Vec::new();

    for line in diff_output.lines() {
        let trimmed_line = line.trim(); // Trim leading and trailing spaces

        if trimmed_line.starts_with("diff --git") || trimmed_line.starts_with("index") {
            continue; // Skip lines that are not actual changes
        }

        if trimmed_line.starts_with("---") || trimmed_line.starts_with("+++") {
            continue; // Skip file change headers
        }

        if trimmed_line.starts_with("@@") {
            // This line indicates a new chunk of changes; skip it
            continue;
        }

        if trimmed_line.starts_with('-') {
            // This line is from the base branch (deleted line)
            base_lines.push(trimmed_line.red().to_string());
        } else if trimmed_line.starts_with('+') {
            // This line is from the HEAD branch (added line)
            head_lines.push(trimmed_line.green().to_string());
        } else {
            // This line is unchanged
            base_lines.push(trimmed_line.to_string());
            head_lines.push(trimmed_line.to_string());
        }
    }

    // Ensure both columns have the same number of rows
    let max_lines = base_lines.len().max(head_lines.len());

    let empty_string = "".to_string();
    // Add rows to the table
    for i in 0..max_lines {
        let base_line = base_lines.get(i).unwrap_or(&empty_string);
        let head_line = head_lines.get(i).unwrap_or(&empty_string);
        table.add_row(vec![base_line, head_line]);
    }

    // Print the table
    println!("{}", table);

    terminal::disable_raw_mode()?;
    Ok(())
}
