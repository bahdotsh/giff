use regex::Regex;
use std::collections::HashMap;

pub fn parse_diff_output(
    diff_output: &str,
) -> HashMap<String, (Vec<(usize, String)>, Vec<(usize, String)>)> {
    let diff_file_regex = Regex::new(r"^diff --git a/(.+) b/(.+)$").unwrap();
    let hunk_header_regex = Regex::new(r"^@@ -(\d+),\d+ \+(\d+),\d+ @@").unwrap();
    let mut file_changes: HashMap<String, (Vec<(usize, String)>, Vec<(usize, String)>)> =
        HashMap::new();
    let mut current_file = String::new();
    let mut base_lines = Vec::new();
    let mut head_lines = Vec::new();
    let mut base_line_number = 1;
    let mut head_line_number = 1;

    // Regex to remove ANSI escape codes
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
        file_changes.insert(current_file, (base_lines, head_lines));
    }

    file_changes
}
