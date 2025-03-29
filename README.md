# Giff - Git Diff Viewer with Interactive Rebase Support

Giff is a terminal-based Git diff viewer with interactive rebase capabilities that allows you to view and manage changes between branches.

## Features

- **Side-by-Side or Unified Diff View**: Choose between two different viewing modes for comparing changes
- **Interactive Navigation**: Easily navigate through files and changes with keyboard shortcuts
- **Rebase Detection**: Automatically detects when a rebase is needed
- **Interactive Rebasing**: Accept or reject changes during rebase right from the interface
- **Auto-Rebase Option**: Command-line flag to automatically rebase when needed

## Installation

Clone the repository and build the project:

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

# View diff between a specific branch and HEAD
giff --branch feature-branch

# Automatically rebase if needed
giff --auto-rebase
```

## Keyboard Shortcuts

### Diff Mode

| Key | Action |
|-----|--------|
| `j` / `Down` | Navigate down |
| `k` / `Up` | Navigate up |
| `Tab` | Toggle focus between file list and diff content |
| `h` / `Left` | Focus file list |
| `l` / `Right` | Focus diff content |
| `u` | Toggle between unified and side-by-side view |
| `r` | Enter rebase mode |
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
| `q` / `Esc` | Cancel and return to diff mode |

### Rebase Notification Dialog

| Key | Action |
|-----|--------|
| `r` | Perform rebase |
| `i` | Ignore rebase suggestion |
| `Esc` | Dismiss notification |

## Rebase Functionality

Giff detects situations when a rebase may be needed, such as:

1. When your local branch is behind its upstream
2. When your branch has diverged from its upstream

When a rebase is recommended, Giff will:

1. Display a notification with details about why a rebase is needed
2. Offer options to perform the rebase from within the app or ignore the suggestion
3. If you choose to rebase, Giff will handle the process and update the display

You can also use the `--auto-rebase` flag to automatically perform a rebase if needed without prompting.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
