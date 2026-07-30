use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use rustc_hash::FxHashMap;

use crate::settings::{GitCliSettings, GitStatusSettings};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GitFileStatus {
    Conflicted,
    Deleted,
    Modified,
    Renamed,
    Added,
    Untracked,
    Ignored,
}

impl GitFileStatus {
    fn priority(self) -> u8 {
        match self {
            GitFileStatus::Conflicted => 0,
            GitFileStatus::Deleted => 1,
            GitFileStatus::Modified => 2,
            GitFileStatus::Renamed => 3,
            GitFileStatus::Added => 3,
            GitFileStatus::Untracked => 4,
            GitFileStatus::Ignored => 5,
        }
    }

    fn worse(self, other: GitFileStatus) -> GitFileStatus {
        if self.priority() <= other.priority() {
            self
        } else {
            other
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GitSnapshot {
    pub branch: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub statuses: FxHashMap<PathBuf, GitFileStatus>,
}

pub fn snapshot(dir: &Path, status: &GitStatusSettings, cli: &GitCliSettings) -> Option<GitSnapshot> {
    let untracked_flag = if status.show_untracked {
        "--untracked-files=all"
    } else {
        "--untracked-files=no"
    };

    let mut args: Vec<&str> = vec!["status", "--porcelain=v1", "-z", "--branch", untracked_flag];
    if status.show_ignored {
        args.push("--ignored=matching");
    }
    args.push("--");
    args.push(".");

    let output = run_git(dir, cli, &args)?;
    let repo_root = find_repo_root(dir).unwrap_or_else(|| dir.to_path_buf());
    let snapshot = parse_porcelain(&repo_root, &output)?;

    if let Some(max) = cli.max_repo_entries
        && snapshot.statuses.len() as u64 > max
    {
        return None;
    }

    Some(snapshot)
}

fn find_repo_root(dir: &Path) -> Option<PathBuf> {
    let mut current = dir;
    loop {
        if current.join(".git").exists() {
            return Some(current.to_path_buf());
        }
        current = current.parent()?;
    }
}

fn run_git(dir: &Path, cli: &GitCliSettings, args: &[&str]) -> Option<Vec<u8>> {
    let binary = cli.binary_path.as_deref().unwrap_or("git");

    let mut child = Command::new(binary)
        .arg("-C")
        .arg(dir)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let mut stdout = child.stdout.take()?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        let _ = tx.send(buf);
    });

    let deadline = Instant::now() + Duration::from_millis(cli.timeout_ms.max(1));
    loop {
        match child.try_wait() {
            Ok(Some(exit_status)) => {
                let buf = rx.recv_timeout(Duration::from_millis(500)).unwrap_or_default();
                return exit_status.success().then_some(buf);
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return None;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => return None,
        }
    }
}

fn parse_porcelain(dir: &Path, output: &[u8]) -> Option<GitSnapshot> {
    let mut tokens = output.split(|&b| b == 0).map(|bytes| String::from_utf8_lossy(bytes));

    let header = tokens.next()?;
    let (branch, ahead, behind) = parse_branch_header(&header);

    let mut statuses = FxHashMap::default();
    while let Some(token) = tokens.next() {
        if token.is_empty() {
            continue;
        }
        if token.len() < 4 {
            continue;
        }
        let (xy, rest) = token.split_at(2);
        let path = &rest[1..];

        if is_rename_or_copy(xy) {
            tokens.next();
        }

        let status = classify(xy);
        statuses.insert(dir.join(path), status);
    }

    Some(GitSnapshot {
        branch,
        ahead,
        behind,
        statuses,
    })
}

fn is_rename_or_copy(xy: &str) -> bool {
    let bytes = xy.as_bytes();
    bytes[0] == b'R' || bytes[1] == b'R' || bytes[0] == b'C' || bytes[1] == b'C'
}

fn classify(xy: &str) -> GitFileStatus {
    match xy {
        "??" => GitFileStatus::Untracked,
        "!!" => GitFileStatus::Ignored,
        "UU" | "AA" | "DD" | "AU" | "UA" | "UD" | "DU" => GitFileStatus::Conflicted,
        _ => {
            let bytes = xy.as_bytes();
            let (x, y) = (bytes[0], bytes[1]);
            if x == b'D' || y == b'D' {
                GitFileStatus::Deleted
            } else if x == b'R' || y == b'R' {
                GitFileStatus::Renamed
            } else if x == b'A' || y == b'A' || x == b'C' || y == b'C' {
                GitFileStatus::Added
            } else {
                GitFileStatus::Modified
            }
        }
    }
}

fn parse_branch_header(header: &str) -> (Option<String>, u32, u32) {
    let Some(rest) = header.strip_prefix("## ") else {
        return (None, 0, 0);
    };

    if rest.starts_with("HEAD (no branch)") {
        return (None, 0, 0);
    }

    if let Some(branch) = rest.strip_prefix("No commits yet on ") {
        return (Some(branch.to_string()), 0, 0);
    }

    let (head, tracking) = match rest.split_once("...") {
        Some((head, tracking)) => (head, Some(tracking)),
        None => (rest, None),
    };

    let mut ahead = 0;
    let mut behind = 0;
    if let Some(tracking) = tracking
        && let Some(bracket_start) = tracking.find('[')
    {
        let inside = tracking[bracket_start + 1..].trim_end_matches(']');
        for part in inside.split(", ") {
            if let Some(n) = part.strip_prefix("ahead ") {
                ahead = n.trim().parse().unwrap_or(0);
            } else if let Some(n) = part.strip_prefix("behind ") {
                behind = n.trim().parse().unwrap_or(0);
            }
        }
    }

    (Some(head.to_string()), ahead, behind)
}

pub fn worst_status_under(
    statuses: &FxHashMap<PathBuf, GitFileStatus>,
    dir: &Path,
) -> Option<GitFileStatus> {
    statuses
        .iter()
        .filter(|(path, _)| path.starts_with(dir))
        .map(|(_, status)| *status)
        .reduce(GitFileStatus::worse)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir() -> PathBuf {
        PathBuf::from("/repo")
    }

    #[test]
    fn parses_clean_branch_header_with_no_tracking() {
        let output = b"## main\0";
        let snapshot = parse_porcelain(&dir(), output).unwrap();

        assert_eq!(snapshot.branch.as_deref(), Some("main"));
        assert_eq!(snapshot.ahead, 0);
        assert_eq!(snapshot.behind, 0);
        assert!(snapshot.statuses.is_empty());
    }

    #[test]
    fn parses_ahead_behind_tracking_info() {
        let output = b"## main...origin/main [ahead 1, behind 2]\0";
        let snapshot = parse_porcelain(&dir(), output).unwrap();

        assert_eq!(snapshot.branch.as_deref(), Some("main"));
        assert_eq!(snapshot.ahead, 1);
        assert_eq!(snapshot.behind, 2);
    }

    #[test]
    fn parses_ahead_only_tracking_info() {
        let output = b"## feature...origin/feature [ahead 3]\0";
        let snapshot = parse_porcelain(&dir(), output).unwrap();

        assert_eq!(snapshot.ahead, 3);
        assert_eq!(snapshot.behind, 0);
    }

    #[test]
    fn detached_head_has_no_branch() {
        let output = b"## HEAD (no branch)\0";
        let snapshot = parse_porcelain(&dir(), output).unwrap();

        assert_eq!(snapshot.branch, None);
    }

    #[test]
    fn unborn_branch_reports_name() {
        let output = b"## No commits yet on main\0";
        let snapshot = parse_porcelain(&dir(), output).unwrap();

        assert_eq!(snapshot.branch.as_deref(), Some("main"));
    }

    #[test]
    fn parses_file_status_entries() {
        let output = b"## main\0 M src/foo.rs\0?? src/new.rs\0!! target/\0A  src/added.rs\0";
        let snapshot = parse_porcelain(&dir(), output).unwrap();

        assert_eq!(
            snapshot.statuses.get(&dir().join("src/foo.rs")),
            Some(&GitFileStatus::Modified)
        );
        assert_eq!(
            snapshot.statuses.get(&dir().join("src/new.rs")),
            Some(&GitFileStatus::Untracked)
        );
        assert_eq!(
            snapshot.statuses.get(&dir().join("target/")),
            Some(&GitFileStatus::Ignored)
        );
        assert_eq!(
            snapshot.statuses.get(&dir().join("src/added.rs")),
            Some(&GitFileStatus::Added)
        );
    }

    #[test]
    fn parses_conflicted_entries() {
        let output = b"## main\0UU src/conflict.rs\0";
        let snapshot = parse_porcelain(&dir(), output).unwrap();

        assert_eq!(
            snapshot.statuses.get(&dir().join("src/conflict.rs")),
            Some(&GitFileStatus::Conflicted)
        );
    }

    #[test]
    fn rename_entries_consume_the_origin_path_token() {
        let output = b"## main\0R  src/new_name.rs\0src/old_name.rs\0?? src/untouched.rs\0";
        let snapshot = parse_porcelain(&dir(), output).unwrap();

        assert_eq!(
            snapshot.statuses.get(&dir().join("src/new_name.rs")),
            Some(&GitFileStatus::Renamed)
        );
        assert!(!snapshot.statuses.contains_key(&dir().join("src/old_name.rs")));
        assert_eq!(
            snapshot.statuses.get(&dir().join("src/untouched.rs")),
            Some(&GitFileStatus::Untracked)
        );
    }

    #[test]
    fn worst_status_under_picks_highest_priority_descendant() {
        let mut statuses = FxHashMap::default();
        statuses.insert(dir().join("sub/a.rs"), GitFileStatus::Untracked);
        statuses.insert(dir().join("sub/b.rs"), GitFileStatus::Conflicted);
        statuses.insert(dir().join("other/c.rs"), GitFileStatus::Modified);

        let worst = worst_status_under(&statuses, &dir().join("sub"));

        assert_eq!(worst, Some(GitFileStatus::Conflicted));
    }

    #[test]
    fn worst_status_under_is_none_when_nothing_matches() {
        let statuses = FxHashMap::default();
        let worst = worst_status_under(&statuses, &dir().join("sub"));

        assert_eq!(worst, None);
    }

    #[test]
    fn find_repo_root_walks_up_to_the_nearest_dot_git() {
        let root = std::env::temp_dir().join(format!("zex_repo_root_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let nested = root.join("src/explorer");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::create_dir(root.join(".git")).unwrap();

        assert_eq!(find_repo_root(&nested), Some(root.clone()));
        assert_eq!(find_repo_root(&root), Some(root.clone()));

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn find_repo_root_treats_a_dot_git_file_as_a_repo_marker() {
        let root =
            std::env::temp_dir().join(format!("zex_repo_root_file_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join(".git"), "gitdir: /elsewhere/.git/worktrees/x\n").unwrap();

        assert_eq!(find_repo_root(&root), Some(root.clone()));

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn find_repo_root_is_none_outside_any_repo() {
        let root = std::env::temp_dir().join(format!("zex_no_repo_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();

        assert_eq!(find_repo_root(&root), None);

        std::fs::remove_dir_all(&root).unwrap();
    }
}
