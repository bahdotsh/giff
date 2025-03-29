mod diff;
mod display;

use clap::Parser;
use std::error::Error;

#[derive(Parser)]
#[command(author="bahdotsh", version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "main")]
    branch: String,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // Get diff data
    let file_changes = diff::get_changes(&args.branch)?;

    // Display the changes
    display::show_diff_table(&file_changes, &args.branch)?;

    Ok(())
}
