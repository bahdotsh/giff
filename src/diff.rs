use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::process::Command;

pub type LineChange = (usize, String);
pub type FileChanges = HashMap<String, (Vec<LineChange>, Vec<LineChange>)>;

pub fn get_changes(branch: &str) -> Result<FileChanges, Box<dyn Error>> {
    let diff_output = get_diff_output(branch)?;
    Ok(parse_diff_output(&diff_output))
}

fn get_diff_output(branch: &str) -> Result<String, Box<dyn Error>> {
    let output = Command::new("git")
        .args(["diff", &format!("{}..HEAD", branch)])
        .output()?;

    if !output.status.success() {
        return Err("Failed to execute git diff command".into());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn parse_diff_output(diff_output: &str) -> FileChanges {
    let diff_file_regex = Regex::new(r"^diff --git a/(.+) b/(.+)$").unwrap();
    let hunk_header_regex = Regex::new(r"^@@ -(\d+),\d+ \+(\d+),\d+ @@").unwrap();
    let ansi_escape_regex = Regex::new(r"\x1b\[.*?m").unwrap();

    let mut file_changes = HashMap::new();
    let mut current_file = String::new();
    let mut base_lines = Vec::new();
    let mut head_lines = Vec::new();
    let mut base_line_number = 1;
    let mut head_line_number = 1;

    for line in diff_output.lines() {
        let trimmed_line = ansi_escape_regex.replace_all(line.trim(), "");

        // Handle file header
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

        // Handle hunk header
        if let Some(caps) = hunk_header_regex.captures(trimmed_line.as_ref()) {
            base_line_number = caps.get(1).unwrap().as_str().parse::<usize>().unwrap();
            head_line_number = caps.get(2).unwrap().as_str().parse::<usize>().unwrap();
            continue;
        }

        // Skip metadata lines
        if trimmed_line.starts_with("index")
            || trimmed_line.starts_with("---")
            || trimmed_line.starts_with("+++")
            || trimmed_line.starts_with("@@")
            || trimmed_line.starts_with("new")
        {
            continue;
        }

        // Process diff lines
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

    // Add last file changes
    if !current_file.is_empty() {
        file_changes.insert(current_file, (base_lines, head_lines));
    }

    file_changes
}

pub fn apply_changes(
    file_path: &str,
    changes: &[(usize, String, bool)],
) -> Result<(), Box<dyn Error>> {
    let original_content = std::fs::read_to_string(file_path)?;
    // Use owned strings instead of references
    let mut lines: Vec<String> = original_content.lines().map(|s| s.to_string()).collect();

    // Sort changes by line number in descending order to avoid messing up line numbers
    let mut sorted_changes = changes.to_vec();
    sorted_changes.sort_by(|a, b| b.0.cmp(&a.0));

    // Apply changes
    for (line_num, content, is_accepted) in sorted_changes {
        if is_accepted {
            // Convert from 1-indexed (display) to 0-indexed (internal)
            let idx = line_num - 1;
            if idx < lines.len() {
                // Remove the +/- prefix if present
                let clean_content = if content.starts_with('+') || content.starts_with('-') {
                    content[1..].to_string()
                } else {
                    content.clone()
                };

                // Now we can assign to our owned String
                lines[idx] = clean_content.trim_start().to_string();
            }
        }
    }

    // Write back to file
    std::fs::write(file_path, lines.join("\n"))?;

    Ok(())
}
