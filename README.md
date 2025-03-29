# giff - Git Diff Viewer and Interactive Rebaser

`giff` is a terminal-based Git diff viewer that allows you to easily visualize changes between branches and selectively apply specific modifications from one branch to another.

## Features

### Powerful Diff Viewing
- Side-by-side comparison of changes between branches
- Unified diff view option for compact rendering
- Color-coded additions and deletions
- Keyboard navigation through files and changes

### Interactive Rebase Mode
- Selectively apply changes from one branch to another
- Review each modification individually with context
- Accept or reject changes with simple keystrokes
- See clear comparison between current and incoming changes
- Apply only the changes you want

## Installation

```bash
cargo install giff
```

## Usage

```bash
# Compare current branch with main
giff

# Compare specific branch with HEAD
giff -b feature-branch
```

## Keyboard Controls

### Diff Viewing Mode
- `j`/`k` or Up/Down: Navigate through files and content
- `Tab` or `h`/`l`: Switch focus between file list and diff content
- `u`: Toggle between side-by-side and unified diff views
- `r`: Enter rebase mode
- `q` or `Esc`: Quit

### Rebase Mode
- `j`/`k`: Navigate between changes
- `n`/`p`: Navigate between files with changes
- `a`: Accept the change (incoming modification)
- `x`: Reject the change (keep current version)
- `c`: Commit all accepted changes to disk
- `q` or `Esc`: Exit rebase mode without applying changes

## The Rebase Workflow

The interactive rebase mode allows you to selectively apply changes from your target branch to your current branch:

1. Start `giff` and view the differences between branches
2. Press `r` to enter rebase mode
3. For each change:
   - Review the current code and the incoming modification
   - For removed lines, you'll see both the current line and its replacement
   - Press `a` to accept the incoming change
   - Press `x` to reject it and keep your current code
4. Navigate between changes with `j`/`k` and between files with `n`/`p`
5. When finished reviewing, press `c` to commit all accepted changes
6. The changes will be applied to your working copy

This enables you to cherry-pick specific changes from another branch without having to manage the complexity of Git's cherry-pick or rebase commands directly.

## Example

Let's say you've made several changes in a feature branch, but main has also progressed with some changes you want to incorporate:

```
$ giff -b main
```

This shows you all differences between your current branch and main. After reviewing the diffs:

1. Press `r` to enter rebase mode
2. Use `j`/`k` to navigate through the individual changes
3. For each change:
   - See both the original code and the new version
   - Accept changes that you want (`a`) and reject others (`x`)
4. Press `c` to apply all accepted changes to your working files

This creates a selective merge of only the changes you want, without needing to resolve conflicts manually.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is licensed under the MIT License - see the LICENSE file for details.
