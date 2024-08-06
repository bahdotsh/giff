# rdiff

This Rust program visualizes the differences between the current HEAD and a specified branch in a git repository using a formatted table output in your terminal. The differences are displayed with color-coded additions and deletions for better readability.

## Features

- **Branch Comparison**: Compare changes between the current HEAD and a specified branch.
- **Color-coded Output**: Additions are displayed in green and deletions in red.
- **Table Formatting**: Uses `comfy_table` to format the output.

## Requirements

- Rust (latest stable version)
- Git
- A terminal supporting ANSI escape codes for color output

## Dependencies

This project uses the following Rust crates:

- `clap`: For command-line argument parsing.
- `comfy_table`: For creating and formatting tables.
- `crossterm`: For terminal manipulation.
- `regex`: For parsing git diff output.

## Installation
```
cargo install rdiff
```

## From source
```
git clone https://github.com/bahdotsh/rdiff.git
cd rdiff
cargo install --path .
```

## Usage
```
rdiff -b branch //by default, the branch will be main
```

# Example Output

<img width="1725" alt="Screenshot 2024-08-06 at 3 34 30 PM" src="https://github.com/user-attachments/assets/c196df7d-90e9-41f5-ab8e-cce1356740a3">
