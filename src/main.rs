mod diff;
mod ui;

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

    // Start the interactive UI
    ui::run_app(file_changes, &args.branch)?;

    Ok(())
}
