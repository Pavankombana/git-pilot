# git-pilot

A fast, lightweight Terminal User Interface (TUI) for Git branch navigation, worktree management, live diff inspection, and in-terminal staging.

Built in Rust with Ratatui and Crossterm.

---

## Overview

git-pilot provides an interactive terminal dashboard for Git repositories. Instead of typing repetitive status, log, diff, and branch commands, developers can inspect repository state and perform Git operations using keyboard navigation.

### Highlights

- Sub-5ms startup latency with zero runtime dependencies.
- 60 FPS flicker-free rendering using Ratatui double-buffering.
- Instant fuzzy search across branches, worktrees, changed files, and commits.
- Git worktree pilot for creating and switching isolated checkouts.
- Syntax-highlighted live diff preview for files and commits.
- In-terminal staging, unstaging, and commit creation.
- Built-in help modal accessible from any view.

---

## Installation

### From Source

Requires Rust and Cargo installed:

```bash
git clone https://github.com/Pavankombana/git-pilot.git
cd git-pilot
cargo build --release
```

The compiled binary will be placed at:
```text
./target/release/git-pilot.exe    (Windows)
./target/release/git-pilot        (Linux / macOS)
```

### Adding to PATH

Copy the executable to any folder included in your system PATH to run `git-pilot` from any terminal prompt inside any Git repository.

---

## Keyboard Controls

### Navigation & Tabs

| Shortcut | Description |
| :--- | :--- |
| `1` - `4` | Switch directly to tab (Branches, Worktrees, Changes, Commits) |
| `Tab` / `Shift+Tab` | Cycle between tabs |
| `j` / `k` or `Down` / `Up` | Move selection down or up |
| `p` | Toggle focus between list panel and preview pane |
| `PageDown` / `PageUp` | Fast-scroll preview pane |
| `/` | Start fuzzy filtering across active list |
| `Esc` | Clear active search or close dialog |
| `?` or `h` | Open interactive shortcuts cheat-sheet |
| `q` or `Ctrl+C` | Exit git-pilot |

### Tab Actions

#### Tab 1: Branches
- `Enter`: Switch to and check out selected branch.
- `b`: Prompt to create and switch to a new branch.
- `d`: Prompt to delete selected branch (cannot delete active HEAD).

#### Tab 2: Worktrees
- `w`: Prompt to create a new Git worktree at a specified directory path.
- `d`: Prompt to remove selected worktree.

#### Tab 3: Changed Files
- `Space` or `Enter`: Toggle stage or unstage on selected file.
- `a`: Stage all modified files (or unstage all if already staged).
- `c`: Open commit prompt to commit all currently staged changes.

#### Tab 4: Commits
- `j` / `k`: Browse recent commit log.
- `p`: Focus diff pane to view patch details for selected commit.

---

## Architecture

The project is structured into four primary modules:

- `src/git.rs`: Direct Git CLI abstraction layer for porcelain status parsing, diff generation, branch manipulation, and worktree control.
- `src/app.rs`: State machine handling active tabs, selection indices, fuzzy filtering with SkimMatcherV2, preview caching, and modal states.
- `src/ui.rs`: Split-pane layout, colorized diff syntax styling, status indicator toasts, and modal dialogs.
- `src/main.rs`: Terminal raw mode management, event loop with 50ms polling, mouse-wheel handler, and panic recovery hook.

---

## Contributing

Pull requests and issues are welcome. Please ensure that all changes pass:

```bash
cargo check
cargo test
cargo build
```

---

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
