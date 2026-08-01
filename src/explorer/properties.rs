use std::os::unix::fs::MetadataExt;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

use gpui::{
    AppContext as _, Bounds, Context, FocusHandle, IntoElement, Render, SharedString, Task,
    TitlebarOptions, Window, WindowBounds, WindowOptions, px, size,
};

use crate::filesystem::entry;
use crate::filesystem::operations::stats::DirStats;
use crate::ui::properties_window;

use super::Explorer;

const WINDOW_WIDTH: f32 = 440.0;
const WINDOW_HEIGHT: f32 = 470.0;

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

pub struct PropertiesWindow {
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

impl PropertiesWindow {
    pub fn close(&mut self, window: &mut Window, _cx: &mut Context<Self>) {
        window.remove_window();
    }

    pub fn set_tab(&mut self, tab: PropertiesTab, cx: &mut Context<Self>) {
        self.tab = tab;
        cx.notify();
    }
}

impl Render for PropertiesWindow {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        properties_window::render(self, window, cx)
    }
}

impl Explorer {
    pub fn open_properties(
        &mut self,
        paths: Vec<PathBuf>,
        _window: &mut Window,
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

        let (stats, has_task) = if any_dir {
            (StatsState::Loading, true)
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
                false,
            )
        };

        let bounds = Bounds::centered(None, size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT)), cx);
        let window_title: SharedString = format!("{name} Info").into();

        let opened = cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(WINDOW_WIDTH), px(WINDOW_HEIGHT))),
                is_resizable: false,
                titlebar: Some(TitlebarOptions {
                    title: Some(window_title),
                    ..Default::default()
                }),
                ..Default::default()
            },
            move |window, cx| {
                cx.new(|cx| {
                    let focus_handle = cx.focus_handle();
                    window.focus(&focus_handle);

                    let task = has_task.then(|| {
                        let task_paths = scan_paths.clone();
                        cx.spawn(async move |weak, cx| {
                            let stats = cx
                                .background_executor()
                                .spawn(async move {
                                    let mut total = DirStats::default();
                                    for path in &task_paths {
                                        total.merge(
                                            crate::filesystem::operations::stats::scan_stats(path),
                                        );
                                    }
                                    total
                                })
                                .await;

                            let _ = weak.update(cx, |properties: &mut PropertiesWindow, cx| {
                                properties.stats = StatsState::Ready(SelectionStats {
                                    file_count: stats.file_count,
                                    total_size: stats.total_size,
                                    size_on_disk: stats.size_on_disk,
                                });
                                cx.notify();
                            });
                        })
                    });

                    PropertiesWindow {
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
                    }
                })
            },
        );

        let _ = opened;
    }
}
