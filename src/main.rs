mod diff;
mod ui;

use clap::Parser;
use std::error::Error;

#[derive(Parser)]
#[command(author="bahdotsh", version, about, long_about = None)]
struct Args {
    /// Base reference for diff (commit, branch, etc.)
    #[arg(default_value = "")]
    from: String,

    /// Target reference for diff (commit, branch, etc.; defaults to current state)
    #[arg(default_value = "")]
    to: String,

    /// Pass this to run diff with custom arguments
    #[arg(short, long)]
    diff_args: Option<String>,

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

    // Get diff data based on arguments
    let (file_changes, left_label, right_label) = if let Some(diff_args) = &args.diff_args {
        // Use custom diff arguments
        diff::get_changes_with_args(diff_args)?
    } else if !args.from.is_empty() && !args.to.is_empty() {
        // Compare two refs (from..to)
        diff::get_changes_between(&args.from, &args.to)?
    } else if !args.from.is_empty() {
        // Compare ref to working tree (like git diff <ref>)
        diff::get_changes_to_ref(&args.from)?
    } else {
        // Default behavior: show uncommitted changes
        diff::get_uncommitted_changes()?
    };

    // Start the interactive UI
    ui::run_app(file_changes, &left_label, &right_label)?;

    Ok(())
}
