use std::time::Duration;

use gpui::{Context, Rgba};

use crate::filesystem::entry::FsEntry;
use crate::git::{self, GitFileStatus};
use crate::theme;

use super::Explorer;

impl Explorer {
    pub fn refresh_git(&mut self, cx: &mut Context<Self>) {
        self.git_task = None;

        if !self.git_settings.enabled || self.is_trash() {
            self.git_snapshot = None;
            return;
        }

        let dir = self.current_dir().to_path_buf();
        let status = self.git_settings.status.clone();
        let cli = self.git_settings.cli.clone();

        self.git_task = Some(cx.spawn(async move |weak, cx| {
            let snapshot = cx
                .background_executor()
                .spawn(async move { git::snapshot(&dir, &status, &cli) })
                .await;

            let _ = weak.update(cx, |explorer, cx| {
                explorer.git_snapshot = snapshot;
                cx.notify();
            });
        }));
    }

    pub(super) fn start_git_poll(&mut self, cx: &mut Context<Self>) {
        self.git_poll_task = None;

        let interval_ms = self.git_settings.refresh.poll_interval_ms;
        if !self.git_settings.enabled || interval_ms == 0 || self.is_trash() {
            return;
        }

        self.git_poll_task = Some(cx.spawn(async move |weak, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(interval_ms))
                    .await;

                let alive = weak
                    .update(cx, |explorer, cx| {
                        explorer.refresh_git(cx);
                    })
                    .is_ok();
                if !alive {
                    break;
                }
            }
        }));
    }

    pub fn git_status_for(&self, entry: &FsEntry) -> Option<git::GitFileStatus> {
        if !self.git_settings.enabled {
            return None;
        }
        let snapshot = self.git_snapshot.as_ref()?;

        if entry.is_dir {
            if !self.git_settings.status.aggregate_folders {
                return None;
            }
            git::worst_status_under(&snapshot.statuses, &entry.path)
        } else {
            snapshot.statuses.get(&entry.path).copied()
        }
    }

    pub fn git_branch_label(&self) -> Option<&str> {
        self.git_snapshot.as_ref()?.branch.as_deref()
    }

    pub fn git_ahead_behind(&self) -> Option<(u32, u32)> {
        let snapshot = self.git_snapshot.as_ref()?;
        Some((snapshot.ahead, snapshot.behind))
    }

    pub fn git_is_dirty(&self) -> bool {
        self.git_snapshot
            .as_ref()
            .is_some_and(|snapshot| snapshot.statuses.values().any(|status| *status != GitFileStatus::Ignored))
    }

    pub fn git_color_for(&self, status: GitFileStatus) -> Rgba {
        match status {
            GitFileStatus::Modified => theme::git_color_modified(),
            GitFileStatus::Added => theme::git_color_added(),
            GitFileStatus::Deleted => theme::git_color_deleted(),
            GitFileStatus::Renamed => theme::git_color_renamed(),
            GitFileStatus::Untracked => theme::git_color_untracked(),
            GitFileStatus::Ignored => theme::git_color_ignored(),
            GitFileStatus::Conflicted => theme::git_color_conflicted(),
        }
    }
}

pub fn git_letter_for(status: GitFileStatus) -> &'static str {
    match status {
        GitFileStatus::Modified => "M",
        GitFileStatus::Added => "A",
        GitFileStatus::Deleted => "D",
        GitFileStatus::Renamed => "R",
        GitFileStatus::Untracked => "U",
        GitFileStatus::Ignored => "I",
        GitFileStatus::Conflicted => "!",
    }
}
