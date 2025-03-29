mod diff;
mod ui;

use clap::Parser;
use std::error::Error;

#[derive(Parser)]
#[command(author="bahdotsh", version, about, long_about = None)]
struct Args {
    #[arg(short, long, default_value = "main")]
    branch: String,

    #[arg(short, long, help = "Auto-rebase if needed")]
    auto_rebase: bool,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();

    // Check if rebase is needed before proceeding
    if args.auto_rebase {
        if let Some(rebase_msg) = diff::check_rebase_needed()? {
            eprintln!("{}", rebase_msg);

            // Get upstream branch
            let output = std::process::Command::new("git")
                .args(["rev-parse", "--abbrev-ref", "HEAD@{u}"])
                .output()?;

            if output.status.success() {
                let upstream = String::from_utf8_lossy(&output.stdout).trim().to_string();

                eprintln!("Auto-rebasing onto {}...", upstream);
                if diff::perform_rebase(&upstream)? {
                    eprintln!("Rebase successful!");
                } else {
                    eprintln!("Rebase failed. There might be conflicts to resolve.");
                    return Err("Rebase failed".into());
                }
            }
        }
    }

    // Get diff data
    let file_changes = diff::get_changes(&args.branch)?;

    // Start the interactive UI
    ui::run_app(file_changes, &args.branch)?;

    Ok(())
}
