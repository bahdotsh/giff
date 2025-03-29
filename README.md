# giff

An interactive git diff viewer with a terminal UI, designed to help you understand changes between branches more easily.

## Features

- **Interactive TUI Interface**: Navigate through changed files and their content with ease
- **Side-by-side Diff View**: Compare branch and HEAD changes in parallel panes
- **Color-coded Changes**: Quickly identify additions, deletions, and unchanged lines
- **Keyboard Navigation**: Intuitive shortcuts for exploring diffs efficiently
- **Independent Scrolling**: Scroll through base and HEAD content separately

## Installation

```bash
cargo install giff
```

Or build from source:

```bash
git clone https://github.com/bahdotsh/giff.git
cd giff
cargo build --release
```

## Usage

```bash
# Compare current branch with main (default)
giff

# Compare with a specific branch
giff -b develop

# View help
giff --help
```

## Navigation

| Key | Action |
|-----|--------|
| `j` / Down Arrow | Move down in current pane |
| `k` / Up Arrow | Move up in current pane |
| `Tab` | Cycle focus between file list, base content, and HEAD content |
| `h` / Left Arrow | Focus the file list |
| `l` / Right Arrow | Focus content panes |
| `q` | Quit the application |

## Screenshots

![giff interface](screenshot.png)

## How It Works

giff uses `git diff` to generate a comparison between your current HEAD and a specified branch. It then parses this output and presents it in an interactive terminal UI powered by Ratatui, allowing you to:

1. Browse the list of changed files
2. View the base branch content on the left
3. View the HEAD content on the right
4. Navigate through changes with intuitive keyboard controls

The application highlights additions in green and deletions in red, making it easy to identify what changed between branches.

## Dependencies

- [clap](https://crates.io/crates/clap) - Command line argument parsing
- [regex](https://crates.io/crates/regex) - For parsing diff output
- [ratatui](https://crates.io/crates/ratatui) - Terminal UI framework
- [crossterm](https://crates.io/crates/crossterm) - Terminal manipulation

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
