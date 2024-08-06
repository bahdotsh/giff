mod args;
mod giff;
mod parser;
mod table;

use args::Args;
use clap::Parser;
use comfy_table::Cell;
use comfy_table::{presets::UTF8_FULL_CONDENSED, Table};
use crossterm::{
    execute,
    terminal::{self, ClearType},
};
use std::io::{self};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    // Execute git diff command
    let diff_output = giff::get_diff_output(&args.branch)?;

    // Clear the terminal
    let mut stdout = io::stdout();
    execute!(stdout, terminal::Clear(ClearType::All))?;

    // Parse and accumulate diff output
    let file_changes = parser::parse_diff_output(&diff_output);

    // Create and configure the table
    let mut table = Table::new();
    table.set_content_arrangement(comfy_table::ContentArrangement::DynamicFullWidth);
    table.load_preset(UTF8_FULL_CONDENSED);
    table.set_header(vec![
        Cell::new("File").set_alignment(comfy_table::CellAlignment::Center),
        Cell::new(args.branch.as_str()).set_alignment(comfy_table::CellAlignment::Center),
        Cell::new("HEAD").set_alignment(comfy_table::CellAlignment::Center),
    ]);

    // Add rows to the table
    table::populate_table(&mut table, file_changes);

    // Print the table
    println!("{}", table.trim_fmt());

    Ok(())
}
