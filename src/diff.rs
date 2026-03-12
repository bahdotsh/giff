use regex::Regex;
use std::collections::HashMap;
use std::error::Error;
use std::process::Command;
use std::sync::LazyLock;

static DIFF_FILE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^diff --git a/(.+) b/(.+)$").unwrap());
static HUNK_HEADER_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^@@ -(\d+),?\d* \+(\d+),?\d* @@").unwrap());
static ANSI_ESCAPE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b\[.*?m").unwrap());

pub type LineChange = (usize, String);
pub type FileChanges = HashMap<String, (Vec<LineChange>, Vec<LineChange>)>;

// Get changes with completely custom diff args
pub fn get_changes_with_args(args: &str) -> Result<(FileChanges, String, String), Box<dyn Error>> {
    let args_vec: Vec<&str> = args.split_whitespace().collect();
    let diff_output = get_diff_output_with_args(&args_vec)?;

    // Try to extract meaningful labels from the args
    let left_label = extract_left_label(args);
    let right_label = extract_right_label(args);

    Ok((parse_diff_output(&diff_output)?, left_label, right_label))
}

// Compare uncommitted changes (git diff)
pub fn get_uncommitted_changes() -> Result<(FileChanges, String, String), Box<dyn Error>> {
    let diff_output = get_diff_output_with_args(&[])?;
    Ok((
        parse_diff_output(&diff_output)?,
        "HEAD".to_string(),
        "Working Tree".to_string(),
    ))
}

// Compare a specific reference to working tree (git diff <ref>)
pub fn get_changes_to_ref(
    reference: &str,
) -> Result<(FileChanges, String, String), Box<dyn Error>> {
    let diff_output = get_diff_output_with_args(&[reference])?;
    Ok((
        parse_diff_output(&diff_output)?,
        reference.to_string(),
        "Working Tree".to_string(),
    ))
}

// Compare two references (git diff <from>..<to>)
pub fn get_changes_between(
    from: &str,
    to: &str,
) -> Result<(FileChanges, String, String), Box<dyn Error>> {
    let diff_output = get_diff_output_with_args(&[&format!("{}..{}", from, to)])?;
    Ok((
        parse_diff_output(&diff_output)?,
        from.to_string(),
        to.to_string(),
    ))
}

fn get_diff_output_with_args(args: &[&str]) -> Result<String, Box<dyn Error>> {
    let mut cmd_args = vec!["diff"];
    cmd_args.extend_from_slice(args);

    let output = Command::new("git").args(&cmd_args).output()?;

    if !output.status.success() {
        return Err(format!(
            "Failed to execute git diff command: {}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn extract_left_label(args: &str) -> String {
    // Try to extract meaningful label from diff args
    if args.contains("..") {
        // For format like "branch1..branch2"
        let parts: Vec<&str> = args.split("..").collect();
        if !parts.is_empty() {
            return parts[0].trim().to_string();
        }
    }
    // Default label
    "Base".to_string()
}

fn extract_right_label(args: &str) -> String {
    // Try to extract meaningful label from diff args
    if args.contains("..") {
        // For format like "branch1..branch2"
        let parts: Vec<&str> = args.split("..").collect();
        if parts.len() > 1 {
            return parts[1].trim().to_string();
        }
    }
    // Default label
    "Target".to_string()
}

fn parse_diff_output(diff_output: &str) -> Result<FileChanges, Box<dyn Error>> {
    let mut file_changes = HashMap::new();
    let mut current_file = String::new();
    let mut base_lines = Vec::new();
    let mut head_lines = Vec::new();
    let mut base_line_number = 1;
    let mut head_line_number = 1;

    for line in diff_output.lines() {
        let trimmed_line = ANSI_ESCAPE_RE.replace_all(line.trim(), "");

        // Handle file header
        if let Some(caps) = DIFF_FILE_RE.captures(trimmed_line.as_ref()) {
            if !current_file.is_empty() {
                file_changes.insert(
                    current_file.clone(),
                    (base_lines.clone(), head_lines.clone()),
                );
                base_lines.clear();
                head_lines.clear();
            }

            // Use second capture group as file path in most cases (the "b/" file)
            current_file = match caps.get(2) {
                Some(m) => m.as_str().to_string(),
                None => continue,
            };
            base_line_number = 1;
            head_line_number = 1;
            continue;
        }

        // Handle hunk header
        if let Some(caps) = HUNK_HEADER_RE.captures(trimmed_line.as_ref()) {
            base_line_number = caps
                .get(1)
                .and_then(|m| m.as_str().parse::<usize>().ok())
                .unwrap_or(1);
            head_line_number = caps
                .get(2)
                .and_then(|m| m.as_str().parse::<usize>().ok())
                .unwrap_or(1);
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

    Ok(file_changes)
}

#[derive(Clone)]
pub enum ChangeOp {
    /// Replace line at the given 1-indexed base position with new content
    Replace(usize, String),
    /// Delete line at the given 1-indexed base position
    Delete(usize),
    /// Insert content at the given 1-indexed base position.
    /// `order` is the original head line number, used to keep multiple
    /// insertions at the same base position in the correct order.
    Insert {
        base_pos: usize,
        order: usize,
        content: String,
    },
}

impl ChangeOp {
    fn line_num(&self) -> usize {
        match self {
            ChangeOp::Replace(n, _) | ChangeOp::Delete(n) => *n,
            ChangeOp::Insert { base_pos, .. } => *base_pos,
        }
    }
}

pub fn apply_changes(file_path: &str, operations: &[ChangeOp]) -> Result<(), Box<dyn Error>> {
    if operations.is_empty() {
        return Ok(());
    }

    let original_content = std::fs::read_to_string(file_path)?;
    let has_trailing_newline = original_content.ends_with('\n');
    let mut lines: Vec<String> = original_content.lines().map(|s| s.to_string()).collect();

    // Phase 1: Apply Delete and Replace operations (already in base coordinates).
    // Process in descending line-number order so that removals at higher
    // positions don't shift indices for lower positions.
    let mut base_ops: Vec<&ChangeOp> = operations
        .iter()
        .filter(|op| matches!(op, ChangeOp::Replace(..) | ChangeOp::Delete(..)))
        .collect();
    base_ops.sort_by_key(|op| std::cmp::Reverse(op.line_num()));

    let mut deleted_positions: Vec<usize> = Vec::new();

    for op in &base_ops {
        match op {
            ChangeOp::Replace(line_num, content) => {
                if *line_num == 0 {
                    continue;
                }
                let idx = line_num - 1;
                if idx < lines.len() {
                    lines[idx] = content.clone();
                }
            }
            ChangeOp::Delete(line_num) => {
                if *line_num == 0 {
                    continue;
                }
                let idx = line_num - 1;
                if idx < lines.len() {
                    lines.remove(idx);
                    deleted_positions.push(*line_num);
                }
            }
            _ => {}
        }
    }

    // Phase 2: Apply Insert operations, adjusting positions for prior deletions.
    // Sort by (base_pos DESC, order DESC) so that multiple inserts at the
    // same base position end up in the correct source order: the last one
    // processed at a position pushes earlier ones down.
    let mut insert_ops: Vec<&ChangeOp> = operations
        .iter()
        .filter(|op| matches!(op, ChangeOp::Insert { .. }))
        .collect();
    insert_ops.sort_by(|a, b| {
        let pos_cmp = b.line_num().cmp(&a.line_num());
        if pos_cmp != std::cmp::Ordering::Equal {
            return pos_cmp;
        }
        // Tiebreak: higher order (head line number) processed first
        let a_order = if let ChangeOp::Insert { order, .. } = a {
            *order
        } else {
            0
        };
        let b_order = if let ChangeOp::Insert { order, .. } = b {
            *order
        } else {
            0
        };
        b_order.cmp(&a_order)
    });

    for op in &insert_ops {
        if let ChangeOp::Insert {
            base_pos, content, ..
        } = op
        {
            if *base_pos == 0 {
                continue;
            }
            // Adjust for lines that were deleted at positions before this one
            let deletes_before = deleted_positions.iter().filter(|&&d| d < *base_pos).count();
            let adjusted = base_pos.saturating_sub(deletes_before);
            let idx = adjusted.saturating_sub(1).min(lines.len());
            lines.insert(idx, content.clone());
        }
    }

    let mut result = lines.join("\n");
    if has_trailing_newline {
        result.push('\n');
    }
    std::fs::write(file_path, result)?;

    Ok(())
}

pub fn check_rebase_needed() -> Result<Option<String>, Box<dyn Error>> {
    // Check if we're in a git repository
    let status = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()?;

    if !status.status.success() {
        return Ok(None);
    }

    // Get current branch name
    let branch_output = Command::new("git")
        .args(["symbolic-ref", "--short", "HEAD"])
        .output()?;

    if !branch_output.status.success() {
        return Ok(None);
    }

    let current_branch = String::from_utf8_lossy(&branch_output.stdout)
        .trim()
        .to_string();

    // Check if branch has an upstream and get its name
    let upstream_output = match Command::new("git")
        .args([
            "rev-parse",
            "--abbrev-ref",
            &format!("{}@{{u}}", current_branch),
        ])
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => return Ok(None), // No upstream configured
    };

    let upstream_name = String::from_utf8_lossy(&upstream_output.stdout)
        .trim()
        .to_string();

    // Check branch status relative to upstream
    let status_output = Command::new("git").args(["status", "-sb"]).output()?;
    let status_text = String::from_utf8_lossy(&status_output.stdout).to_string();

    // Check for diverged state (both ahead and behind)
    if status_text.contains("ahead") && status_text.contains("behind") {
        return Ok(Some(format!(
            "Your branch '{}' has diverged from '{}'.\nConsider rebasing to integrate changes cleanly.",
            current_branch, upstream_name
        )));
    }

    // Check for behind-only state
    if status_text.contains("[behind") {
        return Ok(Some(format!(
            "Your branch '{}' is behind '{}'. A rebase is recommended.",
            current_branch, upstream_name
        )));
    }

    Ok(None)
}

pub fn perform_rebase(upstream: &str) -> Result<bool, Box<dyn Error>> {
    let output = Command::new("git").args(["rebase", upstream]).output()?;

    Ok(output.status.success())
}
