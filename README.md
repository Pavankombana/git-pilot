# ✈ GIT-PILOT

> Ultra-fast, keyboard-driven Git & Worktree Pilot Terminal User Interface (TUI) built in Rust with Ratatui.

![Rust](https://img.shields.io/badge/Language-Rust-orange.svg)
![Ratatui](https://img.shields.io/badge/UI-Ratatui-red.svg)
![License](https://img.shields.io/badge/License-MIT-blue.svg)

---

## ✨ Features

- **🚀 Sub-5ms Cold Startup:** Pure native Rust with zero runtime dependencies.
- **⚡ 60 FPS Flicker-Free Rendering:** Powered by Ratatui's double-buffering diff engine.
- **🔍 Instant Fuzzy Filtering:** Press `/` to fuzzy-search across branches, worktrees, changes, and commit history.
- **🌳 Worktree-Centric Workflow:** Effortlessly create (`w`) and manage isolated Git worktrees for multitasking without stashing.
- **🎨 Colorized Live Diff Preview:** Real-time diffs with syntax highlighting for changed files, branches, and commits.
- **⌨️ Keyboard & Mouse Driven:** Full Vim navigation (`j`/`k`), tab switching, and mouse-wheel scrolling.

---

## 🕹️ Keybindings

| Key | Action |
| :--- | :--- |
| `1` / `2` / `3` / `4` | Jump directly to Tab (Branches, Worktrees, Changes, Commits) |
| `Tab` / `Shift+Tab` | Cycle through tabs |
| `j` / `k` or `↓` / `↑` | Navigate active list / scroll preview |
| `p` | Toggle focus between List and Preview pane |
| `/` | Start fuzzy search across items |
| `Esc` | Clear search / Cancel dialog |
| `Enter` | **Branches:** Checkout branch<br>**Changes:** Stage / Unstage file |
| `Space` | **Changes:** Stage / Unstage file |
| `b` | **Branches:** Create and switch to new branch |
| `w` | **Worktrees:** Create a new Git worktree |
| `d` | **Branches/Worktrees:** Delete selected item (with confirmation) |
| `r` | Force refresh repository status |
| `q` or `Ctrl+C` | Quit git-pilot |

---

## 🛠️ Building & Running

### Run directly with Cargo:
```bash
cargo run
```

### Build optimized release binary:
```bash
cargo build --release
```
The compiled binary will be located at:
`./target/release/git-pilot.exe` (only ~2.6 MB).

### Add to your PATH:
Copy `target/release/git-pilot.exe` to any directory in your system PATH (e.g. `C:\Windows\System32` or a custom bin directory) to use `git-pilot` from any terminal!

---

## 🏗️ Architecture

- **`src/git.rs`**: Git CLI abstraction layer for branches, worktrees, status porcelain, diffs, and staging.
- **`src/app.rs`**: Application state machine, tabs, fuzzy-matcher indexing, preview caching, and modal dialogs.
- **`src/ui.rs`**: Ratatui layout splits, list views, diff syntax highlighters, and modal rendering.
- **`src/main.rs`**: Terminal raw mode setup, cross-platform event polling, and panic recovery hook.
