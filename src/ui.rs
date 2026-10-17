use crate::app::{ActivePane, ActiveTab, App, InputMode};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Tabs, Wrap},
    Frame,
};

pub fn render(f: &mut Frame, app: &mut App) {
    let size = f.area();

    // Top-level vertical split: Header, Main Content, Footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header & Tabs
            Constraint::Min(5),    // Main Content
            Constraint::Length(2), // Footer & Status bar
        ])
        .split(size);

    render_header(f, app, chunks[0]);
    render_main(f, app, chunks[1]);
    render_footer(f, app, chunks[2]);

    // Render Modal Dialog if any prompt is active
    render_modals(f, app, size);
}

fn render_header(f: &mut Frame, app: &App, area: Rect) {
    let header_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(28), Constraint::Min(20)])
        .split(area);

    // Title & Repo Badge
    let repo_title = format!(" GIT-PILOT | {} ", app.repo_name);
    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));
    let title_paragraph = Paragraph::new(Span::styled(
        repo_title,
        Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
    ))
    .block(title_block);
    f.render_widget(title_paragraph, header_chunks[0]);

    // Tabs
    let tab_titles = vec![
        format!(" [1] Branches ({}) ", app.branches.len()),
        format!(" [2] Worktrees ({}) ", app.worktrees.len()),
        format!(" [3] Changes ({}) ", app.files.len()),
        format!(" [4] Commits ({}) ", app.commits.len()),
    ];

    let titles: Vec<Line> = tab_titles
        .into_iter()
        .map(|t| Line::from(t))
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .select(app.active_tab as usize)
        .style(Style::default().fg(Color::Gray))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
                .bg(Color::Rgb(40, 40, 50)),
        )
        .divider(Span::styled("|", Style::default().fg(Color::DarkGray)));

    f.render_widget(tabs, header_chunks[1]);
}

fn render_main(f: &mut Frame, app: &mut App, area: Rect) {
    // Horizontal Split: Left List (40%) and Right Preview (60%)
    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(42), Constraint::Percentage(58)])
        .split(area);

    render_left_panel(f, app, content_chunks[0]);
    render_right_preview(f, app, content_chunks[1]);
}

fn render_left_panel(f: &mut Frame, app: &mut App, area: Rect) {
    // If search query is non-empty or search mode is active, reserve space for search bar
    let has_search = !app.search_query.is_empty() || matches!(app.input_mode, InputMode::Search);

    let (search_area, list_area) = if has_search {
        let split = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(3), Constraint::Min(4)])
            .split(area);
        (Some(split[0]), split[1])
    } else {
        (None, area)
    };

    if let Some(s_area) = search_area {
        let search_block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title(" Filter (Esc to clear) ")
            .border_style(if matches!(app.input_mode, InputMode::Search) {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::DarkGray)
            });

        let cursor_char = if matches!(app.input_mode, InputMode::Search) { "_" } else { "" };
        let search_text = format!(" {}{}", app.search_query, cursor_char);
        let search_paragraph = Paragraph::new(search_text)
            .block(search_block)
            .style(Style::default().fg(Color::White));
        f.render_widget(search_paragraph, s_area);
    }

    let is_focused = app.active_pane == ActivePane::List;
    let list_border_color = if is_focused { Color::Cyan } else { Color::DarkGray };

    match app.active_tab {
        ActiveTab::Branches => {
            let items: Vec<ListItem> = app
                .filtered_branches
                .iter()
                .enumerate()
                .map(|(display_idx, &real_idx)| {
                    let b = &app.branches[real_idx];
                    let is_selected = display_idx == app.selected_branch;

                    let prefix = if b.is_head {
                        Span::styled("* ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                    } else if b.is_remote {
                        Span::styled("[r] ", Style::default().fg(Color::Blue))
                    } else {
                        Span::styled("    ", Style::default())
                    };

                    let name_style = if b.is_head {
                        Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                    } else if is_selected {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    let details = format!(" ({}) {}", b.commit_hash, b.commit_msg);

                    ListItem::new(Line::from(vec![
                        prefix,
                        Span::styled(&b.name, name_style),
                        Span::styled(details, Style::default().fg(Color::DarkGray)),
                    ]))
                })
                .collect();

            let title = format!(" Branches [{}/{}] ", app.filtered_branches.len(), app.branches.len());
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .title(title)
                        .border_style(Style::default().fg(list_border_color)),
                )
                .highlight_symbol("▶ ")
                .highlight_style(
                    Style::default()
                        .bg(Color::Rgb(30, 40, 60))
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                );

            let mut state = ratatui::widgets::ListState::default();
            if !app.filtered_branches.is_empty() {
                state.select(Some(app.selected_branch));
            }
            f.render_stateful_widget(list, list_area, &mut state);
        }

        ActiveTab::Worktrees => {
            let items: Vec<ListItem> = app
                .filtered_worktrees
                .iter()
                .enumerate()
                .map(|(display_idx, &real_idx)| {
                    let w = &app.worktrees[real_idx];
                    let is_selected = display_idx == app.selected_worktree;

                    let badge = if w.is_current {
                        Span::styled(" [CURRENT] ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
                    } else {
                        Span::styled("           ", Style::default())
                    };

                    let branch_name = w.branch.as_deref().unwrap_or("detached");
                    let name_style = if is_selected {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    ListItem::new(Line::from(vec![
                        badge,
                        Span::styled(branch_name, name_style),
                        Span::styled(format!(" -> {}", w.path), Style::default().fg(Color::DarkGray)),
                    ]))
                })
                .collect();

            let title = format!(" Worktrees [{}/{}] ", app.filtered_worktrees.len(), app.worktrees.len());
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .title(title)
                        .border_style(Style::default().fg(list_border_color)),
                )
                .highlight_symbol("▶ ")
                .highlight_style(
                    Style::default()
                        .bg(Color::Rgb(30, 40, 60))
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                );

            let mut state = ratatui::widgets::ListState::default();
            if !app.filtered_worktrees.is_empty() {
                state.select(Some(app.selected_worktree));
            }
            f.render_stateful_widget(list, list_area, &mut state);
        }

        ActiveTab::Changes => {
            let items: Vec<ListItem> = app
                .filtered_files
                .iter()
                .enumerate()
                .map(|(display_idx, &real_idx)| {
                    let file = &app.files[real_idx];
                    let is_selected = display_idx == app.selected_file;

                    let status_badge = format!("[{}{}] ", file.staged_code, file.unstaged_code);
                    let badge_color = if file.staged_code != ' ' && file.staged_code != '?' {
                        Color::Green
                    } else if file.unstaged_code == '?' {
                        Color::Blue
                    } else {
                        Color::Red
                    };

                    let file_style = if is_selected {
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };

                    ListItem::new(Line::from(vec![
                        Span::styled(status_badge, Style::default().fg(badge_color).add_modifier(Modifier::BOLD)),
                        Span::styled(&file.path, file_style),
                    ]))
                })
                .collect();

            let title = format!(" Working Tree Changes [{}/{}] ", app.filtered_files.len(), app.files.len());
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .title(title)
                        .border_style(Style::default().fg(list_border_color)),
                )
                .highlight_symbol("▶ ")
                .highlight_style(
                    Style::default()
                        .bg(Color::Rgb(30, 40, 60))
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                );

            let mut state = ratatui::widgets::ListState::default();
            if !app.filtered_files.is_empty() {
                state.select(Some(app.selected_file));
            }
            f.render_stateful_widget(list, list_area, &mut state);
        }

        ActiveTab::Commits => {
            let items: Vec<ListItem> = app
                .filtered_commits
                .iter()
                .enumerate()
                .map(|(display_idx, &real_idx)| {
                    let c = &app.commits[real_idx];
                    let is_selected = display_idx == app.selected_commit;

                    let hash_span = Span::styled(
                        format!("{} ", c.hash),
                        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
                    );
                    let date_span = Span::styled(
                        format!("[{}] ", c.date),
                        Style::default().fg(Color::DarkGray),
                    );
                    let msg_style = if is_selected {
                        Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::Gray)
                    };

                    ListItem::new(Line::from(vec![
                        hash_span,
                        date_span,
                        Span::styled(&c.message, msg_style),
                    ]))
                })
                .collect();

            let title = format!(" Recent Commits [{}/{}] ", app.filtered_commits.len(), app.commits.len());
            let list = List::new(items)
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .title(title)
                        .border_style(Style::default().fg(list_border_color)),
                )
                .highlight_symbol("▶ ")
                .highlight_style(
                    Style::default()
                        .bg(Color::Rgb(30, 40, 60))
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                );

            let mut state = ratatui::widgets::ListState::default();
            if !app.filtered_commits.is_empty() {
                state.select(Some(app.selected_commit));
            }
            f.render_stateful_widget(list, list_area, &mut state);
        }
    }
}

fn render_right_preview(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.active_pane == ActivePane::Preview;
    let border_color = if is_focused { Color::Cyan } else { Color::DarkGray };

    let title = match app.active_tab {
        ActiveTab::Branches => " Branch Diff & Log Preview ",
        ActiveTab::Worktrees => " Worktree Inspector ",
        ActiveTab::Changes => " File Diff (Staged/Unstaged) ",
        ActiveTab::Commits => " Commit Details & Patch ",
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title(title)
        .border_style(Style::default().fg(border_color));

    let styled_lines: Vec<Line> = app
        .preview_lines
        .iter()
        .map(|line| {
            if line.starts_with('+') && !line.starts_with("+++") {
                Line::from(Span::styled(line, Style::default().fg(Color::Green)))
            } else if line.starts_with('-') && !line.starts_with("---") {
                Line::from(Span::styled(line, Style::default().fg(Color::Red)))
            } else if line.starts_with("@@") {
                Line::from(Span::styled(line, Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)))
            } else if line.starts_with("diff --git") || line.starts_with("---") || line.starts_with("+++") {
                Line::from(Span::styled(line, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
            } else if line.starts_with("commit ") {
                Line::from(Span::styled(line, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
            } else if line.starts_with("Author:") || line.starts_with("Date:") || line.starts_with("Branch:") {
                Line::from(Span::styled(line, Style::default().fg(Color::Magenta)))
            } else {
                Line::from(Span::styled(line, Style::default().fg(Color::White)))
            }
        })
        .collect();

    let paragraph = Paragraph::new(styled_lines)
        .block(block)
        .scroll((app.preview_scroll, 0))
        .wrap(Wrap { trim: false });

    f.render_widget(paragraph, area);
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(area);

    let shortcuts_text = match app.active_tab {
        ActiveTab::Branches => "[j/k]: Nav | [/]: Search | [Enter]: Switch | [b]: New Branch | [d]: Delete | [?]: Help | [q]: Quit",
        ActiveTab::Worktrees => "[j/k]: Nav | [/]: Search | [w]: New Worktree | [d]: Delete WT | [?]: Help | [q]: Quit",
        ActiveTab::Changes => "[j/k]: Nav | [/]: Search | [Space]: Stage | [a]: All | [c]: Commit | [?]: Help | [q]: Quit",
        ActiveTab::Commits => "[j/k]: Nav | [/]: Search | [p]: Toggle Pane | [?]: Help | [q]: Quit",
    };

    let helper = Paragraph::new(Span::styled(
        shortcuts_text,
        Style::default().fg(Color::DarkGray),
    ));
    f.render_widget(helper, chunks[0]);

    if let Some(toast) = &app.toast {
        let toast_style = if toast.is_error {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
        };
        let toast_p = Paragraph::new(Span::styled(format!("* {}", toast.message), toast_style))
            .alignment(Alignment::Right);
        f.render_widget(toast_p, chunks[1]);
    }
}

fn render_modals(f: &mut Frame, app: &App, size: Rect) {
    match &app.input_mode {
        InputMode::Help => {
            let area = centered_rect(70, 60, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .title(" GIT-PILOT Help & Shortcuts ")
                .border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

            let lines = vec![
                Line::from(Span::styled("NAVIGATION & TABS", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
                Line::from("  [1] - [4]           Switch directly to tab (Branches, Worktrees, Changes, Commits)"),
                Line::from("  [Tab] / [Shift+Tab] Cycle between tabs"),
                Line::from("  [j] / [k] / Arrows  Navigate list up and down"),
                Line::from("  [p]                 Toggle focus to preview pane (use j/k to scroll diffs)"),
                Line::from("  [PageUp] / [PageDn] Fast scroll preview pane"),
                Line::from(""),
                Line::from(Span::styled("ACTIONS BY TAB", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
                Line::from("  Branches:   [Enter] Switch/checkout  |  [b] New branch  |  [d] Delete branch"),
                Line::from("  Worktrees:  [w] New worktree         |  [d] Delete worktree"),
                Line::from("  Changes:    [Space] Stage/Unstage    |  [a] Stage/Unstage all  |  [c] Commit staged"),
                Line::from(""),
                Line::from(Span::styled("GLOBAL CONTROLS", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))),
                Line::from("  [/]                 Fuzzy search across active list"),
                Line::from("  [r]                 Refresh repository status"),
                Line::from("  [?] / [h]           Open this help window"),
                Line::from("  [q] / [Ctrl+C]      Quit git-pilot"),
                Line::from(""),
                Line::from(Span::styled("Press [Esc], [q], or [Enter] to close", Style::default().fg(Color::DarkGray))),
            ];

            let p = Paragraph::new(lines).block(block);
            f.render_widget(p, area);
        }

        InputMode::CommitStaged { buffer } => {
            let area = centered_rect(65, 25, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .title(" Commit Staged Changes ")
                .border_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD));

            let text = vec![
                Line::from(Span::styled("Enter commit message:", Style::default().fg(Color::Gray))),
                Line::from(""),
                Line::from(vec![
                    Span::styled("> ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::styled(buffer, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::styled("_", Style::default().fg(Color::Green)),
                ]),
                Line::from(""),
                Line::from(Span::styled("[Enter] Commit  |  [Esc] Cancel", Style::default().fg(Color::DarkGray))),
            ];

            let p = Paragraph::new(text).block(block).alignment(Alignment::Center);
            f.render_widget(p, area);
        }

        InputMode::NewBranch { buffer } => {
            let area = centered_rect(60, 20, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .title(" Create New Branch ")
                .border_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD));

            let text = vec![
                Line::from(Span::styled("Enter new branch name:", Style::default().fg(Color::Gray))),
                Line::from(""),
                Line::from(vec![
                    Span::styled("> ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
                    Span::styled(buffer, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::styled("_", Style::default().fg(Color::Green)),
                ]),
                Line::from(""),
                Line::from(Span::styled("[Enter] Create  |  [Esc] Cancel", Style::default().fg(Color::DarkGray))),
            ];

            let p = Paragraph::new(text).block(block).alignment(Alignment::Center);
            f.render_widget(p, area);
        }

        InputMode::NewWorktreePath { buffer } => {
            let area = centered_rect(65, 25, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .title(" Add Git Worktree ")
                .border_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

            let branch_name = if let Some(&idx) = app.filtered_branches.get(app.selected_branch) {
                app.branches.get(real_branch_idx(app, idx)).map(|b| b.name.as_str()).unwrap_or("HEAD")
            } else {
                "HEAD"
            };

            let text = vec![
                Line::from(Span::styled(
                    format!("Creating worktree for branch: '{}'", branch_name),
                    Style::default().fg(Color::Yellow),
                )),
                Line::from("Enter destination folder path (e.g. ../feature-preview):"),
                Line::from(""),
                Line::from(vec![
                    Span::styled("> ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                    Span::styled(buffer, Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                    Span::styled("_", Style::default().fg(Color::Cyan)),
                ]),
                Line::from(""),
                Line::from(Span::styled("[Enter] Create  |  [Esc] Cancel", Style::default().fg(Color::DarkGray))),
            ];

            let p = Paragraph::new(text).block(block).alignment(Alignment::Center);
            f.render_widget(p, area);
        }

        InputMode::ConfirmDeleteBranch { branch } => {
            let area = centered_rect(55, 20, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .title(" Confirm Branch Deletion ")
                .border_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));

            let text = vec![
                Line::from(Span::styled(
                    format!("Are you sure you want to delete branch '{}'?", branch),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled("This action cannot be undone.", Style::default().fg(Color::Red))),
                Line::from(""),
                Line::from(Span::styled("[y / Enter] Delete  |  [n / Esc] Cancel", Style::default().fg(Color::Yellow))),
            ];

            let p = Paragraph::new(text).block(block).alignment(Alignment::Center);
            f.render_widget(p, area);
        }

        InputMode::ConfirmDeleteWorktree { path } => {
            let area = centered_rect(60, 20, size);
            f.render_widget(Clear, area);

            let block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .title(" Confirm Worktree Removal ")
                .border_style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD));

            let text = vec![
                Line::from(Span::styled(
                    format!("Remove worktree at '{}'?", path),
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled("[y / Enter] Remove  |  [n / Esc] Cancel", Style::default().fg(Color::Yellow))),
            ];

            let p = Paragraph::new(text).block(block).alignment(Alignment::Center);
            f.render_widget(p, area);
        }

        _ => {}
    }
}

fn real_branch_idx(app: &App, idx: usize) -> usize {
    if idx < app.branches.len() {
        idx
    } else {
        0
    }
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
