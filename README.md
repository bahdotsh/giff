# Giff - Git Diff Viewer with Interactive Rebase Support

Giff is a terminal-based Git diff viewer with interactive rebase capabilities, syntax highlighting, and theme support.

## Features

- **Side-by-Side or Unified Diff View**: Toggle between two viewing modes for comparing changes
- **Syntax Highlighting**: Automatic language-aware syntax highlighting via syntect
- **Dark/Light Themes**: Built-in dark and light themes with full customization support
- **Interactive Navigation**: Keyboard and mouse support for navigating files and diffs
- **Help Modal**: Press `?` to view all keybindings in context
- **Rebase Detection**: Automatically detects when a rebase is needed
- **Interactive Rebasing**: Accept or reject individual changes during rebase
- **Configuration File**: Persistent settings via `~/.config/giff/config.toml`

## Installation

Using Cargo Install (Recommended):

```
cargo install giff
```

From source:

```bash
git clone https://github.com/your-username/giff.git
cd giff
cargo build --release
```

The compiled binary will be available at `target/release/giff`.

## Usage

```bash
# View diff between main branch and HEAD
giff

# View diff between two refs
giff main feature-branch

# Use a specific theme
giff --theme light

# Pass custom git diff arguments
giff -d "--stat"

# Auto-rebase if needed (non-interactive)
giff --auto-rebase
```

## Configuration

Giff reads settings from `~/.config/giff/config.toml`:

```toml
# Set default theme ("dark" or "light")
theme = "dark"
```

The theme can also be overridden with `--theme` on the command line.

## Keyboard Shortcuts

### Diff Mode

| Key | Action |
|-----|--------|
| `j` / `Down` | Navigate down |
| `k` / `Up` | Navigate up |
| `PageDown` | Page down |
| `PageUp` | Page up |
| `Home` | Go to first item |
| `End` | Go to last item |
| `Tab` | Toggle focus between file list and diff content |
| `h` / `Left` | Focus file list |
| `l` / `Right` | Focus diff content |
| `u` | Toggle between unified and side-by-side view |
| `t` | Toggle between dark and light theme |
| `r` | Enter rebase mode |
| `?` | Show help modal |
| `q` / `Esc` | Quit |

### Rebase Mode

| Key | Action |
|-----|--------|
| `j` / `Down` | Navigate to next change |
| `k` / `Up` | Navigate to previous change |
| `a` | Accept change |
| `x` | Reject change |
| `n` | Go to next file with changes |
| `p` | Go to previous file with changes |
| `c` | Commit accepted changes |
| `?` | Show help modal |
| `Esc` | Cancel and return to diff mode |

### Rebase Notification Dialog

| Key | Action |
|-----|--------|
| `r` | Perform rebase |
| `i` | Ignore rebase suggestion |
| `Esc` | Dismiss notification |

### Mouse

| Input | Action |
|-------|--------|
| Scroll wheel | Scroll the focused pane (file list or diff content) |

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
