mod app;
mod git;
mod ui;

use anyhow::Result;
use app::{ActivePane, ActiveTab, App, InputMode};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{io, time::Duration};

fn main() -> Result<()> {
    // Check if in git repository
    if !git::is_git_repo() {
        eprintln!("Error: git-pilot must be executed inside a Git repository.");
        eprintln!("Try running `git init` or `cd` into an existing Git project first.");
        std::process::exit(1);
    }

    // Install panic hook to restore terminal if unexpected panic occurs
    let default_panic = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        default_panic(panic_info);
    }));

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();

    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        eprintln!("git-pilot error: {:#}", err);
    }

    Ok(())
}

fn run_app(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>, app: &mut App) -> Result<()> {
    while !app.should_quit {
        terminal.draw(|f| ui::render(f, app))?;

        if event::poll(Duration::from_millis(50))? {
            match event::read()? {
                Event::Key(key) => {
                    // Only process key press events (ignore release on Windows)
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    handle_key_event(app, key.code, key.modifiers);
                }
                Event::Mouse(mouse) => {
                    match mouse.kind {
                        MouseEventKind::ScrollDown => {
                            if app.active_pane == ActivePane::Preview {
                                app.scroll_preview_down(3);
                            } else {
                                app.next_item();
                            }
                        }
                        MouseEventKind::ScrollUp => {
                            if app.active_pane == ActivePane::Preview {
                                app.scroll_preview_up(3);
                            } else {
                                app.prev_item();
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}

fn handle_key_event(app: &mut App, code: KeyCode, modifiers: KeyModifiers) {
    // Global Ctrl+C quits anytime
    if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
        app.should_quit = true;
        return;
    }

    match &mut app.input_mode {
        InputMode::Normal => match code {
            KeyCode::Char('q') => app.should_quit = true,
            KeyCode::Tab => {
                app.active_tab = app.active_tab.next();
                app.apply_filter();
                app.update_preview();
            }
            KeyCode::BackTab => {
                app.active_tab = app.active_tab.prev();
                app.apply_filter();
                app.update_preview();
            }
            KeyCode::Char('1') => {
                app.active_tab = ActiveTab::Branches;
                app.apply_filter();
                app.update_preview();
            }
            KeyCode::Char('2') => {
                app.active_tab = ActiveTab::Worktrees;
                app.apply_filter();
                app.update_preview();
            }
            KeyCode::Char('3') => {
                app.active_tab = ActiveTab::Changes;
                app.apply_filter();
                app.update_preview();
            }
            KeyCode::Char('4') => {
                app.active_tab = ActiveTab::Commits;
                app.apply_filter();
                app.update_preview();
            }
            KeyCode::Char('p') => {
                app.active_pane = match app.active_pane {
                    ActivePane::List => ActivePane::Preview,
                    ActivePane::Preview => ActivePane::List,
                };
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if app.active_pane == ActivePane::Preview {
                    app.scroll_preview_down(2);
                } else {
                    app.next_item();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if app.active_pane == ActivePane::Preview {
                    app.scroll_preview_up(2);
                } else {
                    app.prev_item();
                }
            }
            KeyCode::PageDown => app.scroll_preview_down(10),
            KeyCode::PageUp => app.scroll_preview_up(10),
            KeyCode::Char('/') => {
                app.input_mode = InputMode::Search;
            }
            KeyCode::Char('r') => {
                app.refresh_all();
                app.set_toast("Refreshed repository status", false);
            }
            KeyCode::Enter => match app.active_tab {
                ActiveTab::Branches => {
                    if let Some(&idx) = app.filtered_branches.get(app.selected_branch) {
                        let branch_name = app.branches[idx].name.clone();
                        match git::checkout_branch(&branch_name) {
                            Ok(_) => {
                                app.set_toast(format!("Switched to branch '{}'", branch_name), false);
                                app.refresh_all();
                            }
                            Err(e) => {
                                app.set_toast(format!("Checkout failed: {}", e), true);
                            }
                        }
                    }
                }
                ActiveTab::Changes => {
                    toggle_stage_selected_file(app);
                }
                _ => {}
            },
            KeyCode::Char(' ') => {
                if app.active_tab == ActiveTab::Changes {
                    toggle_stage_selected_file(app);
                }
            }
            KeyCode::Char('b') => {
                if app.active_tab == ActiveTab::Branches {
                    app.input_mode = InputMode::NewBranch { buffer: String::new() };
                }
            }
            KeyCode::Char('w') => {
                app.input_mode = InputMode::NewWorktreePath { buffer: String::new() };
            }
            KeyCode::Char('d') => match app.active_tab {
                ActiveTab::Branches => {
                    if let Some(&idx) = app.filtered_branches.get(app.selected_branch) {
                        let b = &app.branches[idx];
                        if b.is_head {
                            app.set_toast("Cannot delete active HEAD branch", true);
                        } else {
                            app.input_mode = InputMode::ConfirmDeleteBranch { branch: b.name.clone() };
                        }
                    }
                }
                ActiveTab::Worktrees => {
                    if let Some(&idx) = app.filtered_worktrees.get(app.selected_worktree) {
                        let w = &app.worktrees[idx];
                        if w.is_current {
                            app.set_toast("Cannot remove current active worktree", true);
                        } else {
                            app.input_mode = InputMode::ConfirmDeleteWorktree { path: w.path.clone() };
                        }
                    }
                }
                _ => {}
            },
            KeyCode::Char('a') => {
                if app.active_tab == ActiveTab::Changes {
                    let has_unstaged = app.files.iter().any(|f| f.unstaged_code != ' ' || f.staged_code == '?');
                    if has_unstaged {
                        match git::stage_all() {
                            Ok(_) => {
                                app.set_toast("Staged all changed files", false);
                                app.refresh_all();
                            }
                            Err(e) => app.set_toast(format!("Failed to stage all: {}", e), true),
                        }
                    } else {
                        match git::unstage_all() {
                            Ok(_) => {
                                app.set_toast("Unstaged all files", false);
                                app.refresh_all();
                            }
                            Err(e) => app.set_toast(format!("Failed to unstage all: {}", e), true),
                        }
                    }
                }
            }
            KeyCode::Char('c') => {
                if app.active_tab == ActiveTab::Changes {
                    let has_staged = app.files.iter().any(|f| f.staged_code != ' ' && f.staged_code != '?');
                    if has_staged {
                        app.input_mode = InputMode::CommitStaged { buffer: String::new() };
                    } else {
                        app.set_toast("No staged files to commit. Press [Space] or [a] to stage first.", true);
                    }
                }
            }
            KeyCode::Char('?') | KeyCode::Char('h') => {
                app.input_mode = InputMode::Help;
            }
            KeyCode::Esc => {
                if !app.search_query.is_empty() {
                    app.search_query.clear();
                    app.apply_filter();
                    app.update_preview();
                }
            }
            _ => {}
        },

        InputMode::Search => match code {
            KeyCode::Esc | KeyCode::Enter => {
                app.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                app.search_query.pop();
                app.apply_filter();
                app.update_preview();
            }
            KeyCode::Char(c) => {
                app.search_query.push(c);
                app.apply_filter();
                app.update_preview();
            }
            _ => {}
        },

        InputMode::Help => match code {
            _ => {
                app.input_mode = InputMode::Normal;
            }
        },

        InputMode::CommitStaged { buffer } => match code {
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                buffer.pop();
            }
            KeyCode::Char(c) => {
                buffer.push(c);
            }
            KeyCode::Enter => {
                let msg = buffer.trim().to_string();
                if msg.is_empty() {
                    app.set_toast("Commit message cannot be empty", true);
                } else {
                    match git::commit_staged(&msg) {
                        Ok(_) => {
                            app.set_toast(format!("Committed: {}", msg), false);
                            app.refresh_all();
                        }
                        Err(e) => {
                            app.set_toast(format!("Commit failed: {}", e), true);
                        }
                    }
                }
                app.input_mode = InputMode::Normal;
            }
            _ => {}
        },

        InputMode::NewBranch { buffer } => match code {
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                buffer.pop();
            }
            KeyCode::Char(c) => {
                if !c.is_whitespace() {
                    buffer.push(c);
                }
            }
            KeyCode::Enter => {
                let name = buffer.trim().to_string();
                if name.is_empty() {
                    app.set_toast("Branch name cannot be empty", true);
                } else {
                    match git::create_branch(&name) {
                        Ok(_) => {
                            app.set_toast(format!("Created and switched to branch '{}'", name), false);
                            app.refresh_all();
                        }
                        Err(e) => {
                            app.set_toast(format!("Failed to create branch: {}", e), true);
                        }
                    }
                }
                app.input_mode = InputMode::Normal;
            }
            _ => {}
        },

        InputMode::NewWorktreePath { buffer } => match code {
            KeyCode::Esc => {
                app.input_mode = InputMode::Normal;
            }
            KeyCode::Backspace => {
                buffer.pop();
            }
            KeyCode::Char(c) => {
                buffer.push(c);
            }
            KeyCode::Enter => {
                let path = buffer.trim().to_string();
                let branch_name = if let Some(&idx) = app.filtered_branches.get(app.selected_branch) {
                    app.branches.get(idx).map(|b| b.name.as_str()).unwrap_or("HEAD")
                } else {
                    "HEAD"
                };

                if path.is_empty() {
                    app.set_toast("Worktree path cannot be empty", true);
                } else {
                    match git::add_worktree(&path, branch_name) {
                        Ok(_) => {
                            app.set_toast(format!("Created worktree at '{}'", path), false);
                            app.active_tab = ActiveTab::Worktrees;
                            app.refresh_all();
                        }
                        Err(e) => {
                            app.set_toast(format!("Failed to create worktree: {}", e), true);
                        }
                    }
                }
                app.input_mode = InputMode::Normal;
            }
            _ => {}
        },

        InputMode::ConfirmDeleteBranch { branch } => {
            let b_name = branch.clone();
            match code {
                KeyCode::Char('y') | KeyCode::Enter => {
                    match git::delete_branch(&b_name, false) {
                        Ok(_) => {
                            app.set_toast(format!("Deleted branch '{}'", b_name), false);
                            app.refresh_all();
                        }
                        Err(_) => match git::delete_branch(&b_name, true) {
                            Ok(_) => {
                                app.set_toast(format!("Force-deleted branch '{}'", b_name), false);
                                app.refresh_all();
                            }
                            Err(e) => {
                                app.set_toast(format!("Failed to delete branch: {}", e), true);
                            }
                        },
                    }
                    app.input_mode = InputMode::Normal;
                }
                KeyCode::Char('n') | KeyCode::Esc => {
                    app.set_toast("Branch deletion canceled", false);
                    app.input_mode = InputMode::Normal;
                }
                _ => {}
            }
        }

        InputMode::ConfirmDeleteWorktree { path } => {
            let w_path = path.clone();
            match code {
                KeyCode::Char('y') | KeyCode::Enter => {
                    match git::remove_worktree(&w_path, false) {
                        Ok(_) => {
                            app.set_toast(format!("Removed worktree at '{}'", w_path), false);
                            app.refresh_all();
                        }
                        Err(_) => match git::remove_worktree(&w_path, true) {
                            Ok(_) => {
                                app.set_toast(format!("Force-removed worktree at '{}'", w_path), false);
                                app.refresh_all();
                            }
                            Err(e) => {
                                app.set_toast(format!("Failed to remove worktree: {}", e), true);
                            }
                        },
                    }
                    app.input_mode = InputMode::Normal;
                }
                KeyCode::Char('n') | KeyCode::Esc => {
                    app.set_toast("Worktree removal canceled", false);
                    app.input_mode = InputMode::Normal;
                }
                _ => {}
            }
        }
    }
}

fn toggle_stage_selected_file(app: &mut App) {
    if let Some(&idx) = app.filtered_files.get(app.selected_file) {
        let f = &app.files[idx];
        let file_path = f.path.clone();
        let is_staged = f.staged_code != ' ' && f.staged_code != '?';

        if is_staged {
            match git::unstage_file(&file_path) {
                Ok(_) => {
                    app.set_toast(format!("Unstaged {}", file_path), false);
                    app.refresh_all();
                }
                Err(e) => app.set_toast(format!("Failed to unstage: {}", e), true),
            }
        } else {
            match git::stage_file(&file_path) {
                Ok(_) => {
                    app.set_toast(format!("Staged {}", file_path), false);
                    app.refresh_all();
                }
                Err(e) => app.set_toast(format!("Failed to stage: {}", e), true),
            }
        }
    }
}
