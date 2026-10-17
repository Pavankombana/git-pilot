use anyhow::{Context, Result};
use std::process::Command;

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
    pub is_remote: bool,
    pub upstream: Option<String>,
    pub commit_hash: String,
    pub commit_msg: String,
    pub commit_date: String,
}

#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub path: String,
    pub head_hash: String,
    pub branch: Option<String>,
    pub is_bare: bool,
    pub is_locked: bool,
    pub is_current: bool,
}

#[derive(Debug, Clone)]
pub struct FileStatus {
    pub path: String,
    pub staged_code: char,
    pub unstaged_code: char,
}

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
}

/// Helper to execute git command and return stdout string
fn run_git(args: &[&str]) -> Result<String> {
    let output = Command::new("git")
        .args(args)
        .output()
        .with_context(|| format!("Failed to execute 'git {}'", args.join(" ")))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        anyhow::bail!(
            "{}",
            if err.is_empty() {
                format!("git {} returned exit code {:?}", args.join(" "), output.status.code())
            } else {
                err
            }
        );
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn is_git_repo() -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn get_repo_name() -> String {
    if let Ok(path) = run_git(&["rev-parse", "--show-toplevel"]) {
        let trimmed = path.trim();
        let p = std::path::Path::new(trimmed);
        if let Some(name) = p.file_name() {
            return name.to_string_lossy().to_string();
        }
    }
    "Unknown Repo".to_string()
}

pub fn get_current_branch() -> Option<String> {
    run_git(&["branch", "--show-current"])
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

pub fn get_branches() -> Result<Vec<BranchInfo>> {
    let format_arg = "--format=%(HEAD)|%(refname:short)|%(upstream:short)|%(objectname:short)|%(committerdate:relative)|%(subject)";
    let output = run_git(&["branch", "-a", format_arg])?;
    let mut branches = Vec::new();

    for line in output.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 6 {
            continue;
        }

        let is_head = parts[0].trim() == "*";
        let ref_name = parts[1].trim().to_string();
        if ref_name.is_empty() || ref_name.contains("HEAD ->") {
            continue;
        }

        let is_remote = ref_name.starts_with("origin/") || ref_name.starts_with("remotes/");
        let upstream = if parts[2].trim().is_empty() {
            None
        } else {
            Some(parts[2].trim().to_string())
        };

        branches.push(BranchInfo {
            name: ref_name,
            is_head,
            is_remote,
            upstream,
            commit_hash: parts[3].trim().to_string(),
            commit_date: parts[4].trim().to_string(),
            commit_msg: parts[5].trim().to_string(),
        });
    }

    // Sort: current HEAD first, then local branches, then remotes
    branches.sort_by(|a, b| {
        b.is_head.cmp(&a.is_head)
            .then_with(|| a.is_remote.cmp(&b.is_remote))
            .then_with(|| a.name.cmp(&b.name))
    });

    Ok(branches)
}

pub fn get_worktrees() -> Result<Vec<WorktreeInfo>> {
    let output = match run_git(&["worktree", "list", "--porcelain"]) {
        Ok(out) => out,
        Err(_) => return Ok(Vec::new()),
    };

    let current_dir = std::env::current_dir().ok();
    let mut worktrees = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_head: Option<String> = None;
    let mut current_branch: Option<String> = None;
    let mut is_bare = false;
    let mut is_locked = false;

    for line in output.lines() {
        if line.is_empty() {
            if let (Some(path), Some(head)) = (current_path.take(), current_head.take()) {
                let is_current = current_dir.as_ref().map_or(false, |d| {
                    std::path::Path::new(&path) == d.as_path()
                });
                worktrees.push(WorktreeInfo {
                    path,
                    head_hash: head,
                    branch: current_branch.take(),
                    is_bare,
                    is_locked,
                    is_current,
                });
            }
            is_bare = false;
            is_locked = false;
            continue;
        }

        if let Some(path) = line.strip_prefix("worktree ") {
            current_path = Some(path.trim().to_string());
        } else if let Some(head) = line.strip_prefix("HEAD ") {
            current_head = Some(head.trim().chars().take(8).collect());
        } else if let Some(branch) = line.strip_prefix("branch ") {
            let branch_name = branch.trim().trim_start_matches("refs/heads/");
            current_branch = Some(branch_name.to_string());
        } else if line == "bare" {
            is_bare = true;
        } else if line.starts_with("locked") {
            is_locked = true;
        }
    }

    // Flush last entry if not terminated by empty line
    if let (Some(path), Some(head)) = (current_path, current_head) {
        let is_current = current_dir.as_ref().map_or(false, |d| {
            std::path::Path::new(&path) == d.as_path()
        });
        worktrees.push(WorktreeInfo {
            path,
            head_hash: head,
            branch: current_branch,
            is_bare,
            is_locked,
            is_current,
        });
    }

    Ok(worktrees)
}

pub fn get_status() -> Result<Vec<FileStatus>> {
    let output = run_git(&["status", "--porcelain=v1"])?;
    let mut files = Vec::new();

    for line in output.lines() {
        if line.len() < 3 {
            continue;
        }
        let mut chars = line.chars();
        let staged_code = chars.next().unwrap_or(' ');
        let unstaged_code = chars.next().unwrap_or(' ');
        let path = line[3..].trim().to_string();

        files.push(FileStatus {
            path,
            staged_code,
            unstaged_code,
        });
    }

    Ok(files)
}

pub fn get_recent_commits(limit: usize) -> Result<Vec<CommitInfo>> {
    let limit_str = format!("-n{}", limit);
    let output = run_git(&["log", &limit_str, "--format=%h|%an|%cr|%s"])?;
    let mut commits = Vec::new();

    for line in output.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() < 4 {
            continue;
        }
        commits.push(CommitInfo {
            hash: parts[0].trim().to_string(),
            author: parts[1].trim().to_string(),
            date: parts[2].trim().to_string(),
            message: parts[3].trim().to_string(),
        });
    }

    Ok(commits)
}

pub fn get_file_diff(path: &str, staged: bool) -> Result<String> {
    let mut args = vec!["diff"];
    if staged {
        args.push("--cached");
    }
    args.push("--");
    args.push(path);
    run_git(&args)
}

pub fn get_branch_diff(branch: &str) -> Result<String> {
    run_git(&["log", "-p", "-n1", branch])
}

pub fn get_commit_diff(hash: &str) -> Result<String> {
    run_git(&["show", "--stat", "-p", hash])
}

pub fn checkout_branch(branch: &str) -> Result<String> {
    run_git(&["checkout", branch])
}

pub fn create_branch(branch_name: &str) -> Result<String> {
    run_git(&["checkout", "-b", branch_name])
}

pub fn delete_branch(branch_name: &str, force: bool) -> Result<String> {
    let flag = if force { "-D" } else { "-d" };
    run_git(&["branch", flag, branch_name])
}

pub fn add_worktree(path: &str, branch: &str) -> Result<String> {
    run_git(&["worktree", "add", path, branch])
}

pub fn remove_worktree(path: &str, force: bool) -> Result<String> {
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(path);
    run_git(&args)
}

pub fn stage_file(path: &str) -> Result<String> {
    run_git(&["add", "--", path])
}

pub fn unstage_file(path: &str) -> Result<String> {
    run_git(&["restore", "--staged", "--", path])
}
