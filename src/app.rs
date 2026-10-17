use crate::git::{self, BranchInfo, CommitInfo, FileStatus, WorktreeInfo};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActiveTab {
    Branches = 0,
    Worktrees = 1,
    Changes = 2,
    Commits = 3,
}

impl ActiveTab {
    pub fn next(self) -> Self {
        match self {
            Self::Branches => Self::Worktrees,
            Self::Worktrees => Self::Changes,
            Self::Changes => Self::Commits,
            Self::Commits => Self::Branches,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            Self::Branches => Self::Commits,
            Self::Worktrees => Self::Branches,
            Self::Changes => Self::Worktrees,
            Self::Commits => Self::Changes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
    NewBranch { buffer: String },
    NewWorktreePath { buffer: String },
    ConfirmDeleteBranch { branch: String },
    ConfirmDeleteWorktree { path: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivePane {
    List,
    Preview,
}

#[derive(Debug, Clone)]
pub struct Toast {
    pub message: String,
    pub is_error: bool,
}

pub struct App {
    pub is_git: bool,
    pub repo_name: String,
    pub current_branch: Option<String>,
    pub active_tab: ActiveTab,
    pub active_pane: ActivePane,
    pub input_mode: InputMode,
    pub search_query: String,

    pub branches: Vec<BranchInfo>,
    pub filtered_branches: Vec<usize>,
    pub selected_branch: usize,

    pub worktrees: Vec<WorktreeInfo>,
    pub filtered_worktrees: Vec<usize>,
    pub selected_worktree: usize,

    pub files: Vec<FileStatus>,
    pub filtered_files: Vec<usize>,
    pub selected_file: usize,

    pub commits: Vec<CommitInfo>,
    pub filtered_commits: Vec<usize>,
    pub selected_commit: usize,

    pub preview_lines: Vec<String>,
    pub preview_scroll: u16,
    last_preview_key: String,

    pub toast: Option<Toast>,
    pub matcher: SkimMatcherV2,
    pub should_quit: bool,
}

impl App {
    pub fn new() -> Self {
        let is_git = git::is_git_repo();
        let mut app = Self {
            is_git,
            repo_name: if is_git { git::get_repo_name() } else { "Not in a Git repo".to_string() },
            current_branch: if is_git { git::get_current_branch() } else { None },
            active_tab: ActiveTab::Branches,
            active_pane: ActivePane::List,
            input_mode: InputMode::Normal,
            search_query: String::new(),

            branches: Vec::new(),
            filtered_branches: Vec::new(),
            selected_branch: 0,

            worktrees: Vec::new(),
            filtered_worktrees: Vec::new(),
            selected_worktree: 0,

            files: Vec::new(),
            filtered_files: Vec::new(),
            selected_file: 0,

            commits: Vec::new(),
            filtered_commits: Vec::new(),
            selected_commit: 0,

            preview_lines: Vec::new(),
            preview_scroll: 0,
            last_preview_key: String::new(),

            toast: None,
            matcher: SkimMatcherV2::default(),
            should_quit: false,
        };

        if app.is_git {
            app.refresh_all();
        }
        app
    }

    pub fn set_toast(&mut self, msg: impl Into<String>, is_error: bool) {
        self.toast = Some(Toast {
            message: msg.into(),
            is_error,
        });
    }

    pub fn refresh_all(&mut self) {
        if !self.is_git {
            return;
        }

        self.repo_name = git::get_repo_name();
        self.current_branch = git::get_current_branch();

        if let Ok(b) = git::get_branches() {
            self.branches = b;
        }
        if let Ok(w) = git::get_worktrees() {
            self.worktrees = w;
        }
        if let Ok(f) = git::get_status() {
            self.files = f;
        }
        if let Ok(c) = git::get_recent_commits(50) {
            self.commits = c;
        }

        self.apply_filter();
        self.update_preview();
    }

    pub fn apply_filter(&mut self) {
        let q = self.search_query.trim();

        if q.is_empty() {
            self.filtered_branches = (0..self.branches.len()).collect();
            self.filtered_worktrees = (0..self.worktrees.len()).collect();
            self.filtered_files = (0..self.files.len()).collect();
            self.filtered_commits = (0..self.commits.len()).collect();
        } else {
            self.filtered_branches = self
                .branches
                .iter()
                .enumerate()
                .filter(|(_, b)| self.matcher.fuzzy_match(&b.name, q).is_some() || self.matcher.fuzzy_match(&b.commit_msg, q).is_some())
                .map(|(i, _)| i)
                .collect();

            self.filtered_worktrees = self
                .worktrees
                .iter()
                .enumerate()
                .filter(|(_, w)| {
                    self.matcher.fuzzy_match(&w.path, q).is_some()
                        || w.branch.as_ref().map_or(false, |b| self.matcher.fuzzy_match(b, q).is_some())
                })
                .map(|(i, _)| i)
                .collect();

            self.filtered_files = self
                .files
                .iter()
                .enumerate()
                .filter(|(_, f)| self.matcher.fuzzy_match(&f.path, q).is_some())
                .map(|(i, _)| i)
                .collect();

            self.filtered_commits = self
                .commits
                .iter()
                .enumerate()
                .filter(|(_, c)| {
                    self.matcher.fuzzy_match(&c.message, q).is_some()
                        || self.matcher.fuzzy_match(&c.hash, q).is_some()
                        || self.matcher.fuzzy_match(&c.author, q).is_some()
                })
                .map(|(i, _)| i)
                .collect();
        }

        self.clamp_selections();
    }

    fn clamp_selections(&mut self) {
        if !self.filtered_branches.is_empty() && self.selected_branch >= self.filtered_branches.len() {
            self.selected_branch = self.filtered_branches.len() - 1;
        }
        if !self.filtered_worktrees.is_empty() && self.selected_worktree >= self.filtered_worktrees.len() {
            self.selected_worktree = self.filtered_worktrees.len() - 1;
        }
        if !self.filtered_files.is_empty() && self.selected_file >= self.filtered_files.len() {
            self.selected_file = self.filtered_files.len() - 1;
        }
        if !self.filtered_commits.is_empty() && self.selected_commit >= self.filtered_commits.len() {
            self.selected_commit = self.filtered_commits.len() - 1;
        }
    }

    pub fn next_item(&mut self) {
        match self.active_tab {
            ActiveTab::Branches => {
                if !self.filtered_branches.is_empty() && self.selected_branch + 1 < self.filtered_branches.len() {
                    self.selected_branch += 1;
                }
            }
            ActiveTab::Worktrees => {
                if !self.filtered_worktrees.is_empty() && self.selected_worktree + 1 < self.filtered_worktrees.len() {
                    self.selected_worktree += 1;
                }
            }
            ActiveTab::Changes => {
                if !self.filtered_files.is_empty() && self.selected_file + 1 < self.filtered_files.len() {
                    self.selected_file += 1;
                }
            }
            ActiveTab::Commits => {
                if !self.filtered_commits.is_empty() && self.selected_commit + 1 < self.filtered_commits.len() {
                    self.selected_commit += 1;
                }
            }
        }
        self.preview_scroll = 0;
        self.update_preview();
    }

    pub fn prev_item(&mut self) {
        match self.active_tab {
            ActiveTab::Branches => {
                if self.selected_branch > 0 {
                    self.selected_branch -= 1;
                }
            }
            ActiveTab::Worktrees => {
                if self.selected_worktree > 0 {
                    self.selected_worktree -= 1;
                }
            }
            ActiveTab::Changes => {
                if self.selected_file > 0 {
                    self.selected_file -= 1;
                }
            }
            ActiveTab::Commits => {
                if self.selected_commit > 0 {
                    self.selected_commit -= 1;
                }
            }
        }
        self.preview_scroll = 0;
        self.update_preview();
    }

    pub fn update_preview(&mut self) {
        if !self.is_git {
            self.preview_lines = vec!["Not in a git repository.".to_string()];
            return;
        }

        let key = match self.active_tab {
            ActiveTab::Branches => {
                if let Some(&idx) = self.filtered_branches.get(self.selected_branch) {
                    format!("b:{}", self.branches[idx].name)
                } else {
                    "b:none".to_string()
                }
            }
            ActiveTab::Worktrees => {
                if let Some(&idx) = self.filtered_worktrees.get(self.selected_worktree) {
                    format!("w:{}", self.worktrees[idx].path)
                } else {
                    "w:none".to_string()
                }
            }
            ActiveTab::Changes => {
                if let Some(&idx) = self.filtered_files.get(self.selected_file) {
                    format!("f:{}", self.files[idx].path)
                } else {
                    "f:none".to_string()
                }
            }
            ActiveTab::Commits => {
                if let Some(&idx) = self.filtered_commits.get(self.selected_commit) {
                    format!("c:{}", self.commits[idx].hash)
                } else {
                    "c:none".to_string()
                }
            }
        };

        if key == self.last_preview_key {
            return;
        }

        self.last_preview_key = key;

        let content = match self.active_tab {
            ActiveTab::Branches => {
                if let Some(&idx) = self.filtered_branches.get(self.selected_branch) {
                    let b = &self.branches[idx];
                    let diff = git::get_branch_diff(&b.name).unwrap_or_else(|e| e.to_string());
                    format!(
                        "Branch: {}\nHEAD: {}\nUpstream: {}\nLast Commit: {} ({})\nMessage: {}\n\n{}",
                        b.name,
                        if b.is_head { "Yes (*)" } else { "No" },
                        b.upstream.as_deref().unwrap_or("None"),
                        b.commit_hash,
                        b.commit_date,
                        b.commit_msg,
                        diff
                    )
                } else {
                    "No branch selected.".to_string()
                }
            }
            ActiveTab::Worktrees => {
                if let Some(&idx) = self.filtered_worktrees.get(self.selected_worktree) {
                    let w = &self.worktrees[idx];
                    format!(
                        "Worktree Path: {}\nBranch: {}\nHEAD Commit: {}\nActive Checkout: {}\nBare: {}\nLocked: {}",
                        w.path,
                        w.branch.as_deref().unwrap_or("Detached"),
                        w.head_hash,
                        if w.is_current { "Yes (Current Directory)" } else { "No" },
                        w.is_bare,
                        w.is_locked
                    )
                } else {
                    "No worktrees found. Press 'w' to create a new worktree!".to_string()
                }
            }
            ActiveTab::Changes => {
                if let Some(&idx) = self.filtered_files.get(self.selected_file) {
                    let f = &self.files[idx];
                    let is_staged = f.staged_code != ' ' && f.staged_code != '?';
                    let diff = git::get_file_diff(&f.path, is_staged).unwrap_or_else(|e| e.to_string());
                    if diff.trim().is_empty() {
                        format!("File: {}\nStatus: [Staged: '{}', Unstaged: '{}']\n\n(No diff or untracked file)", f.path, f.staged_code, f.unstaged_code)
                    } else {
                        diff
                    }
                } else {
                    "Working tree clean! No changed files.".to_string()
                }
            }
            ActiveTab::Commits => {
                if let Some(&idx) = self.filtered_commits.get(self.selected_commit) {
                    let c = &self.commits[idx];
                    git::get_commit_diff(&c.hash).unwrap_or_else(|e| e.to_string())
                } else {
                    "No commits found.".to_string()
                }
            }
        };

        self.preview_lines = content.lines().map(|s| s.to_string()).collect();
    }

    pub fn scroll_preview_down(&mut self, delta: u16) {
        let max_scroll = (self.preview_lines.len() as u16).saturating_sub(10);
        self.preview_scroll = (self.preview_scroll + delta).min(max_scroll);
    }

    pub fn scroll_preview_up(&mut self, delta: u16) {
        self.preview_scroll = self.preview_scroll.saturating_sub(delta);
    }
}
