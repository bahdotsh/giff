use std::error::Error;
use std::process::Command;

pub fn get_diff_output(branch: &str) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git")
        .args(["diff", &format!("{}..HEAD", branch)])
        .output()?;

    if !output.status.success() {
        eprintln!("Failed to execute git diff command");
        std::process::exit(1);
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
