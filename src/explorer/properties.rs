use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use gpui::{Context, FocusHandle, SharedString, Task, Window};

use crate::filesystem::entry;
use crate::filesystem::operations::stats::DirStats;

use super::Explorer;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PropertiesTab {
    General,
    Permissions,
}

pub struct SelectionStats {
    pub file_count: u64,
    pub total_size: u64,
    pub size_on_disk: u64,
}

pub enum StatsState {
    Loading,
    Ready(SelectionStats),
}

pub struct SingleItemInfo {
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
    pub permissions_changed: Option<SystemTime>,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
}

pub struct PropertiesState {
    pub paths: Vec<PathBuf>,
    pub tab: PropertiesTab,
    pub name: SharedString,
    pub location: SharedString,
    pub type_label: SharedString,
    pub is_dir: bool,
    pub is_symlink: bool,
    pub link_target: Option<SharedString>,
    pub link_target_is_dir: bool,
    pub single: Option<SingleItemInfo>,
    pub stats: StatsState,
    pub focus_handle: FocusHandle,
    _task: Option<Task<()>>,
}

impl Explorer {
    pub fn open_properties(
        &mut self,
        paths: Vec<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let metas: Vec<(PathBuf, std::fs::Metadata)> = paths
            .into_iter()
            .filter_map(|path| {
                std::fs::symlink_metadata(&path)
                    .ok()
                    .map(|meta| (path, meta))
            })
            .collect();

        let Some((first_path, first_meta)) = metas.first() else {
            return;
        };

        let name: SharedString = if metas.len() == 1 {
            first_path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default()
                .into()
        } else {
            format!("{} items selected", metas.len()).into()
        };

        let location: SharedString = self.current_dir().to_string_lossy().into_owned().into();

        let is_dir = metas.iter().all(|(_, meta)| meta.is_dir());
        let any_dir = metas.iter().any(|(_, meta)| meta.is_dir());

        let (is_symlink, is_broken_symlink, link_target, link_target_is_dir) =
            if metas.len() == 1 && first_meta.is_symlink() {
                let link_target = std::fs::read_link(first_path)
                    .ok()
                    .map(|target| target.to_string_lossy().into_owned().into());
                match std::fs::metadata(first_path) {
                    Ok(target_meta) => (true, false, link_target, target_meta.is_dir()),
                    Err(_) => (true, true, link_target, false),
                }
            } else {
                (false, false, None, false)
            };

        let type_label: SharedString = if metas.len() == 1 && is_broken_symlink {
            "Broken Symbolic Link".into()
        } else if metas.len() == 1 && is_symlink {
            let name = first_path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            format!(
                "Symbolic Link ({})",
                entry::type_label_for(&name, link_target_is_dir)
            )
            .into()
        } else if metas.len() == 1 {
            let name = first_path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
            entry::type_label_for(&name, first_meta.is_dir()).into()
        } else if is_dir {
            "Multiple Folders".into()
        } else if !any_dir {
            "Multiple Files".into()
        } else {
            "Multiple Items".into()
        };

        let single = (metas.len() == 1).then(|| SingleItemInfo {
            modified: first_meta.modified().ok(),
            accessed: first_meta.accessed().ok(),
            permissions_changed: Some(
                SystemTime::UNIX_EPOCH + Duration::from_secs(first_meta.ctime().max(0) as u64),
            ),
            mode: first_meta.mode(),
            uid: first_meta.uid(),
            gid: first_meta.gid(),
        });

        let scan_paths: Vec<PathBuf> = metas.iter().map(|(path, _)| path.clone()).collect();

        let (stats, task) = if any_dir {
            let task_paths = scan_paths.clone();
            let task = cx.spawn(async move |weak, cx| {
                let stats = cx
                    .background_executor()
                    .spawn(async move {
                        let mut total = DirStats::default();
                        for path in &task_paths {
                            total.merge(crate::filesystem::operations::stats::scan_stats(path));
                        }
                        total
                    })
                    .await;

                let _ = weak.update(cx, |explorer, cx| {
                    if let Some(properties) = &mut explorer.properties {
                        properties.stats = StatsState::Ready(SelectionStats {
                            file_count: stats.file_count,
                            total_size: stats.total_size,
                            size_on_disk: stats.size_on_disk,
                        });
                    }
                    cx.notify();
                });
            });
            (StatsState::Loading, Some(task))
        } else {
            let mut total = DirStats::default();
            for (_, meta) in &metas {
                total.file_count += 1;
                total.total_size += meta.len();
                total.size_on_disk += meta.blocks() * 512;
            }
            (
                StatsState::Ready(SelectionStats {
                    file_count: total.file_count,
                    total_size: total.total_size,
                    size_on_disk: total.size_on_disk,
                }),
                None,
            )
        };

        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle);

        self.properties = Some(PropertiesState {
            paths: scan_paths,
            tab: PropertiesTab::General,
            name,
            location,
            type_label,
            is_dir,
            is_symlink,
            link_target,
            link_target_is_dir,
            single,
            stats,
            focus_handle,
            _task: task,
        });
        cx.notify();
    }

    pub fn close_properties(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.properties.take().is_some() {
            window.focus(&self.focus_handle);
            cx.notify();
        }
    }

    pub fn set_properties_tab(&mut self, tab: PropertiesTab, cx: &mut Context<Self>) {
        if let Some(properties) = &mut self.properties {
            properties.tab = tab;
            cx.notify();
        }
    }
}
