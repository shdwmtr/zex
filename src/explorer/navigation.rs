use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use gpui::Context;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

use crate::filesystem::entry::{self, FsEntry};

use super::{Explorer, TRASH_VIRTUAL_PATH};

/// Watches a single directory (non-recursively) for changes, forwarding a
/// unit signal whenever *something* changed; we don't care what. Backed by
/// `notify`, which uses inotify on Linux and FSEvents on macOS.
pub(super) struct DirWatcher {
    _watcher: RecommendedWatcher,
}

impl DirWatcher {
    fn spawn(dir: &Path) -> notify::Result<(Self, mpsc::Receiver<()>)> {
        let (tx, rx) = mpsc::channel();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res
                && !matches!(event.kind, EventKind::Access(_) | EventKind::Other)
            {
                let _ = tx.send(());
            }
        })?;
        watcher.watch(dir, RecursiveMode::NonRecursive)?;

        Ok((
            Self {
                _watcher: watcher,
            },
            rx,
        ))
    }
}

impl Explorer {
    pub fn current_dir(&self) -> &Path {
        self.history.current()
    }

    pub fn is_trash(&self) -> bool {
        self.current_dir() == Path::new(TRASH_VIRTUAL_PATH)
    }

    pub fn can_go_back(&self) -> bool {
        self.history.can_go_back()
    }

    pub fn can_go_forward(&self) -> bool {
        self.history.can_go_forward()
    }

    fn compute_free_space_label(dir: &Path) -> String {
        match crate::filesystem::disk::resolve_disk_for(dir) {
            Some(disk) => format!(
                "{} free of {}",
                entry::format_size(disk.available_space),
                entry::format_size(disk.total_space)
            ),
            None => String::new(),
        }
    }

    fn read_dir_filtered(&self, dir: &Path) -> (Vec<FsEntry>, Option<String>) {
        match entry::read_dir_sorted(dir, self.show_hidden) {
            Ok(mut entries) => {
                entry::sort_entries(&mut entries, self.sort_column, self.sort_direction);
                (entries, None)
            }
            Err(err) => (
                Vec::new(),
                Some(format!("Couldn't read {}: {err}", dir.display())),
            ),
        }
    }

    pub(super) fn load_dir(&mut self) {
        if self.is_trash() {
            self.load_trash();
            return;
        }
        let dir = self.history.current().to_path_buf();
        let (entries, error) = self.read_dir_filtered(&dir);
        self.entries = entries;
        self.error = error;
        self.rebuild_entry_index();
        self.selected.clear();
        self.focused_path = None;
        self.trash_entries.clear();
        self.trash_entry_index.clear();
    }

    pub(super) fn refresh_free_space(&mut self, cx: &mut Context<Self>) {
        self.free_space_task = None;

        if self.is_trash() {
            self.free_space_label = String::new();
            return;
        }

        let dir = self.history.current().to_path_buf();
        self.free_space_task = Some(cx.spawn(async move |weak, cx| {
            let label = cx
                .background_executor()
                .spawn(async move { Self::compute_free_space_label(&dir) })
                .await;

            let _ = weak.update(cx, |explorer, cx| {
                explorer.free_space_label = label;
                cx.notify();
            });
        }));
    }

    pub(super) fn rebuild_entry_index(&mut self) {
        self.entry_index.clear();
        self.entry_index.extend(
            self.entries
                .iter()
                .enumerate()
                .map(|(ix, entry)| (entry.path.clone(), ix)),
        );
    }

    pub(super) fn enter_directory(&mut self, cx: &mut Context<Self>) {
        self.load_dir();
        if self.is_trash() {
            self.watcher = None;
            self.watch_task = None;
        } else {
            self.watch_current_dir(cx);
        }
        self.refresh_free_space(cx);
        self.refresh_git(cx);
        self.start_git_poll(cx);
    }

    fn watch_current_dir(&mut self, cx: &mut Context<Self>) {
        let (watcher, rx) = match DirWatcher::spawn(self.history.current()) {
            Ok((watcher, rx)) => (Some(watcher), rx),
            Err(_) => {
                self.watcher = None;
                self.watch_task = None;
                return;
            }
        };
        self.watcher = watcher;

        self.watch_task = Some(cx.spawn(async move |weak, cx| {
            loop {
                let mut changed = false;
                while rx.try_recv().is_ok() {
                    changed = true;
                }

                if changed {
                    cx.background_executor()
                        .timer(Duration::from_millis(200))
                        .await;
                    while rx.try_recv().is_ok() {}

                    let alive = weak
                        .update(cx, |explorer, cx| {
                            explorer.refresh_entries();
                            explorer.refresh_git(cx);
                            cx.notify();
                        })
                        .is_ok();
                    if !alive {
                        break;
                    }
                } else {
                    cx.background_executor()
                        .timer(Duration::from_millis(150))
                        .await;
                }
            }
        }));
    }

    pub(super) fn refresh_entries(&mut self) {
        if self.is_trash() {
            self.refresh_trash();
            return;
        }
        let dir = self.history.current().to_path_buf();
        let (entries, error) = self.read_dir_filtered(&dir);
        self.entries = entries;
        self.error = error;
        self.rebuild_entry_index();

        let entry_index = &self.entry_index;
        self.selected.retain(|path| entry_index.contains_key(path));
        if let Some(focused) = &self.focused_path
            && !entry_index.contains_key(focused)
        {
            self.focused_path = None;
        }
    }

    pub fn toggle_hidden(&mut self, cx: &mut Context<Self>) {
        self.show_hidden = !self.show_hidden;
        self.load_dir();
        cx.notify();
    }

    pub fn navigate_to(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if self.history.navigate(path) {
            self.enter_directory(cx);
            cx.notify();
        }
    }

    pub fn go_up(&mut self, cx: &mut Context<Self>) {
        if self.is_trash() {
            return;
        }
        if let Some(parent) = self.current_dir().parent() {
            let parent = parent.to_path_buf();
            self.navigate_to(parent, cx);
        }
    }

    pub fn go_back(&mut self, cx: &mut Context<Self>) {
        if self.history.back() {
            self.enter_directory(cx);
            cx.notify();
        }
    }

    pub fn go_forward(&mut self, cx: &mut Context<Self>) {
        if self.history.forward() {
            self.enter_directory(cx);
            cx.notify();
        }
    }

    pub fn back_history_entries(&self) -> Vec<(usize, PathBuf)> {
        self.history
            .back_entries()
            .map(|(ix, path)| (ix, path.to_path_buf()))
            .collect()
    }

    pub fn forward_history_entries(&self) -> Vec<(usize, PathBuf)> {
        self.history
            .forward_entries()
            .map(|(ix, path)| (ix, path.to_path_buf()))
            .collect()
    }

    pub fn go_to_history_entry(&mut self, index: usize, cx: &mut Context<Self>) {
        if self.history.jump_to(index) {
            self.enter_directory(cx);
            cx.notify();
        }
    }

    pub fn open_entry(&mut self, path: &Path, cx: &mut Context<Self>) {
        if self.is_trash() {
            return;
        }
        let Some(&ix) = self.entry_index.get(path) else {
            return;
        };
        if self.entries[ix].is_broken_symlink {
            let path = self.entries[ix].path.clone();
            self.op_error = Some(format!("Broken symbolic link: {}", path.display()));
            cx.notify();
            return;
        }
        if self.entries[ix].is_dir {
            let path = self.entries[ix].path.clone();
            self.navigate_to(path, cx);
        } else {
            let path = self.entries[ix].path.clone();
            if let Err(err) = open::that_detached(&path) {
                self.op_error = Some(format!("Couldn't open {}: {err}", path.display()));
                cx.notify();
            }
        }
    }

    pub(super) fn open_focused(&mut self, cx: &mut Context<Self>) {
        if let Some(path) = self.focused_path.clone() {
            self.open_entry(&path, cx);
        }
    }

    pub fn dismiss_op_error(&mut self, cx: &mut Context<Self>) {
        self.op_error = None;
        cx.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::DirWatcher;
    use std::fs;
    use std::time::Duration;

    #[test]
    fn detects_file_creation_in_watched_dir() {
        let dir = std::env::temp_dir().join(format!("zex_watch_smoke_{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();

        let (watcher, rx) = DirWatcher::spawn(&dir).expect("failed to spawn watcher");

        fs::write(dir.join("new_file.txt"), b"hi").unwrap();

        let got = rx.recv_timeout(Duration::from_secs(2)).is_ok();
        drop(watcher);
        fs::remove_dir_all(&dir).ok();

        assert!(got, "expected a change notification after file creation");
    }
}
