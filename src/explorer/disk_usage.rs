use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use gpui::{Bounds, Context, FocusHandle, Pixels, Point, Task, UniformListScrollHandle, Window, point};

use crate::filesystem::disk_usage::{DiskUsageTree, NodeKind, scan_tree};
use crate::filesystem::operations::error;
use crate::filesystem::operations::trash as trash_ops;
use crate::filesystem::undo_op::UndoOp;
use crate::settings::DiskUsageSettings;
use crate::ui::disk_usage::geometry::{self, WedgeGeometry, WedgeTarget};

use super::Explorer;
use super::columns::{self, SortDirection};
use super::drag::SidebarResizeDrag;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiskUsageSortColumn {
    Name,
    Size,
    Contents,
    Modified,
}

pub const MIN_LIST_WIDTH: f32 = 360.0;
pub const MAX_LIST_WIDTH: f32 = 900.0;
pub const DEFAULT_LIST_WIDTH: f32 = 680.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DiskUsageColumn {
    Size,
    Contents,
    Modified,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DiskUsageColumnWidths {
    pub size: f32,
    pub contents: f32,
    pub modified: f32,
}

impl Default for DiskUsageColumnWidths {
    fn default() -> Self {
        Self { size: 90.0, contents: 90.0, modified: 160.0 }
    }
}

impl DiskUsageColumnWidths {
    pub fn get(&self, column: DiskUsageColumn) -> f32 {
        match column {
            DiskUsageColumn::Size => self.size,
            DiskUsageColumn::Contents => self.contents,
            DiskUsageColumn::Modified => self.modified,
        }
    }

    fn set(&mut self, column: DiskUsageColumn, value: f32) {
        let field = match column {
            DiskUsageColumn::Size => &mut self.size,
            DiskUsageColumn::Contents => &mut self.contents,
            DiskUsageColumn::Modified => &mut self.modified,
        };
        *field = value.max(columns::MIN_COLUMN_WIDTH);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct DiskUsageColumnResizeDrag {
    pub column: DiskUsageColumn,
    pub anchor_x: f32,
    pub start_width: f32,
}

pub enum ScanState {
    Scanning { files_scanned: u64, bytes_scanned: u64 },
    Ready { tree: Arc<DiskUsageTree> },
    Failed { error: String },
}

pub struct DiskUsageState {
    pub scan: ScanState,
    pub mount_point: PathBuf,
    pub current_root: PathBuf,
    pub selected_row: Option<PathBuf>,
    pub sort_column: DiskUsageSortColumn,
    pub sort_direction: SortDirection,
    pub focus_handle: FocusHandle,
    pub wedges: Vec<WedgeGeometry>,
    pub hovered_wedge: Option<WedgeTarget>,
    pub hovering_center: bool,
    pub hover_local_pos: Option<Point<Pixels>>,
    pub list_width: f32,
    pub list_resize_drag: Option<SidebarResizeDrag>,
    pub column_widths: DiskUsageColumnWidths,
    pub column_resize_drag: Option<DiskUsageColumnResizeDrag>,
    pub tree_scroll_handle: UniformListScrollHandle,
    cancel: Arc<AtomicBool>,
    _task: Task<()>,
}

/// Swaps out a possibly-huge scanned tree for a cheap placeholder and drops the real
/// `Arc<DiskUsageTree>` on a background thread. Freeing a multi-million-node tree synchronously
/// (e.g. on the close button's click handler) would stall the UI thread for the duration of the
/// deallocation.
fn dispose_tree(scan: &mut ScanState, cx: &mut Context<Explorer>) {
    let placeholder = ScanState::Scanning { files_scanned: 0, bytes_scanned: 0 };
    if let ScanState::Ready { tree } = std::mem::replace(scan, placeholder) {
        cx.background_executor().spawn(async move { drop(tree) }).detach();
    }
}

fn recompute_wedges(state: &mut DiskUsageState) {
    let ScanState::Ready { tree } = &state.scan else {
        state.wedges = Vec::new();
        return;
    };
    let root_id = tree.find(&state.current_root).unwrap_or(tree.root());
    state.wedges = geometry::layout_wedges(tree, root_id, geometry::RING_DEPTH);
    state.hovered_wedge = None;
    state.hovering_center = false;
    state.hover_local_pos = None;
}

fn spawn_scan(
    mount_point: PathBuf,
    settings: DiskUsageSettings,
    cx: &mut Context<Explorer>,
) -> (Arc<AtomicBool>, Task<()>) {
    let cancel = Arc::new(AtomicBool::new(false));
    let files_scanned = Arc::new(AtomicU64::new(0));
    let bytes_scanned = Arc::new(AtomicU64::new(0));
    let (tx, rx) = std::sync::mpsc::channel::<DiskUsageTree>();

    {
        let cancel = cancel.clone();
        let files_scanned = files_scanned.clone();
        let bytes_scanned = bytes_scanned.clone();
        let root = mount_point.clone();
        cx.background_executor()
            .spawn(async move {
                let tree = scan_tree(
                    &root,
                    settings.cross_filesystem_boundaries,
                    settings.follow_symlinks,
                    &cancel,
                    &files_scanned,
                    &bytes_scanned,
                );
                let _ = tx.send(tree);
            })
            .detach();
    }

    let task = cx.spawn(async move |weak, cx| {
        loop {
            match rx.try_recv() {
                Ok(tree) => {
                    let _ = weak.update(cx, |explorer, cx| {
                        if let Some(state) = &mut explorer.disk_usage {
                            state.scan = ScanState::Ready { tree: Arc::new(tree) };
                            recompute_wedges(state);
                        }
                        cx.notify();
                    });
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    let _ = weak.update(cx, |explorer, cx| {
                        if let Some(state) = &mut explorer.disk_usage {
                            state.scan = ScanState::Failed {
                                error: "The scan stopped unexpectedly.".to_string(),
                            };
                        }
                        cx.notify();
                    });
                    break;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }

            let alive = weak
                .update(cx, |explorer, cx| {
                    if let Some(state) = &mut explorer.disk_usage {
                        state.scan = ScanState::Scanning {
                            files_scanned: files_scanned.load(Ordering::Relaxed),
                            bytes_scanned: bytes_scanned.load(Ordering::Relaxed),
                        };
                    }
                    cx.notify();
                })
                .is_ok();
            if !alive {
                break;
            }

            cx.background_executor().timer(Duration::from_millis(120)).await;
        }
    });

    (cancel, task)
}

impl Explorer {
    pub fn open_disk_usage(
        &mut self,
        initial_root: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(disk) = crate::filesystem::disk::resolve_disk_for(self.current_dir()) else {
            self.op_error = Some("Couldn't determine the disk for the current folder".into());
            cx.notify();
            return;
        };

        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle);

        let (cancel, task) = spawn_scan(disk.mount_point.clone(), self.disk_usage_settings.clone(), cx);

        self.disk_usage = Some(DiskUsageState {
            scan: ScanState::Scanning { files_scanned: 0, bytes_scanned: 0 },
            current_root: initial_root.unwrap_or_else(|| disk.mount_point.clone()),
            mount_point: disk.mount_point,
            selected_row: None,
            sort_column: DiskUsageSortColumn::Size,
            sort_direction: SortDirection::Descending,
            focus_handle,
            wedges: Vec::new(),
            hovered_wedge: None,
            hovering_center: false,
            hover_local_pos: None,
            list_width: DEFAULT_LIST_WIDTH,
            list_resize_drag: None,
            column_widths: DiskUsageColumnWidths::default(),
            column_resize_drag: None,
            tree_scroll_handle: UniformListScrollHandle::new(),
            cancel,
            _task: task,
        });
        cx.notify();
    }

    pub fn close_disk_usage(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(mut state) = self.disk_usage.take() {
            state.cancel.store(true, Ordering::SeqCst);
            dispose_tree(&mut state.scan, cx);
            window.focus(&self.focus_handle);
            cx.notify();
        }
    }

    pub fn refresh_scan(&mut self, cx: &mut Context<Self>) {
        let Some(state) = &self.disk_usage else { return };
        state.cancel.store(true, Ordering::SeqCst);
        let mount_point = state.mount_point.clone();

        let (cancel, task) = spawn_scan(mount_point, self.disk_usage_settings.clone(), cx);

        let Some(state) = &mut self.disk_usage else { return };
        dispose_tree(&mut state.scan, cx);
        state.cancel = cancel;
        state._task = task;
        cx.notify();
    }

    pub fn drill_into(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let Some(state) = &mut self.disk_usage else { return };
        let ScanState::Ready { tree } = &state.scan else { return };
        let Some(id) = tree.find(&path) else { return };
        if tree.get(id).kind != NodeKind::Directory {
            return;
        }
        state.current_root = path;
        recompute_wedges(state);
        cx.notify();
    }

    pub fn go_up_one_level(&mut self, cx: &mut Context<Self>) {
        let Some(state) = &mut self.disk_usage else { return };
        let ScanState::Ready { tree } = &state.scan else { return };
        let Some(id) = tree.find(&state.current_root) else { return };
        let Some(parent_id) = tree.get(id).parent else { return };
        state.current_root = tree.get(parent_id).path.clone();
        recompute_wedges(state);
        cx.notify();
    }

    pub fn click_sunburst(&mut self, cx: &mut Context<Self>) {
        let target = self.disk_usage.as_ref().and_then(|state| {
            if let Some(WedgeTarget::Node(id)) = state.hovered_wedge {
                let ScanState::Ready { tree } = &state.scan else { return None };
                Some(Some(tree.get(id).path.clone()))
            } else if state.hovering_center {
                Some(None)
            } else {
                None
            }
        });

        match target {
            Some(Some(path)) => self.drill_into(path, cx),
            Some(None) => self.go_up_one_level(cx),
            None => {}
        }
    }

    pub fn update_sunburst_hover(&mut self, bounds: Bounds<Pixels>, position: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(state) = &mut self.disk_usage else { return };

        if !bounds.contains(&position) {
            let changed = state.hovered_wedge.is_some() || state.hovering_center;
            state.hovered_wedge = None;
            state.hovering_center = false;
            state.hover_local_pos = None;
            if changed {
                cx.notify();
            }
            return;
        }

        let center = bounds.center();
        let max_radius = f32::from(bounds.size.width).min(f32::from(bounds.size.height)) / 2.0 * 0.95;
        let offset = (f32::from(position.x - center.x), f32::from(position.y - center.y));
        let (angle, radius_frac) = geometry::polar(offset, max_radius);

        state.hovering_center = radius_frac < geometry::CENTER_HOLE_FRAC;
        state.hovered_wedge = if state.hovering_center {
            None
        } else {
            geometry::hit_test(&state.wedges, angle, radius_frac)
        };
        state.hover_local_pos = Some(point(position.x - bounds.origin.x, position.y - bounds.origin.y));
        cx.notify();
    }

    pub fn select_disk_usage_row(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let Some(state) = &mut self.disk_usage else { return };
        state.selected_row = Some(path);
        cx.notify();
    }

    pub fn set_disk_usage_sort(&mut self, column: DiskUsageSortColumn, cx: &mut Context<Self>) {
        let Some(state) = &mut self.disk_usage else { return };
        if state.sort_column == column {
            state.sort_direction = match state.sort_direction {
                SortDirection::Ascending => SortDirection::Descending,
                SortDirection::Descending => SortDirection::Ascending,
            };
        } else {
            state.sort_column = column;
            state.sort_direction = SortDirection::Ascending;
        }
        cx.notify();
    }

    pub fn begin_disk_usage_list_resize(&mut self, anchor_x: f32, cx: &mut Context<Self>) {
        let Some(state) = &mut self.disk_usage else { return };
        state.list_resize_drag = Some(SidebarResizeDrag { anchor_x, start_width: state.list_width });
        cx.notify();
    }

    pub fn update_disk_usage_list_resize(&mut self, current_x: f32, cx: &mut Context<Self>) {
        let Some(state) = &mut self.disk_usage else { return };
        let Some(drag) = state.list_resize_drag else { return };
        let new_width = drag.start_width + (current_x - drag.anchor_x);
        state.list_width = new_width.clamp(MIN_LIST_WIDTH, MAX_LIST_WIDTH);
        cx.notify();
    }

    pub fn end_disk_usage_list_resize(&mut self, cx: &mut Context<Self>) {
        let Some(state) = &mut self.disk_usage else { return };
        if state.list_resize_drag.take().is_some() {
            cx.notify();
        }
    }

    pub fn begin_disk_usage_column_resize(
        &mut self,
        column: DiskUsageColumn,
        anchor_x: f32,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = &mut self.disk_usage else { return };
        state.column_resize_drag = Some(DiskUsageColumnResizeDrag {
            column,
            anchor_x,
            start_width: state.column_widths.get(column),
        });
        cx.notify();
    }

    pub fn update_disk_usage_column_resize(&mut self, current_x: f32, cx: &mut Context<Self>) {
        let Some(state) = &mut self.disk_usage else { return };
        let Some(drag) = state.column_resize_drag else { return };
        let new_width = drag.start_width - (current_x - drag.anchor_x);
        state.column_widths.set(drag.column, new_width);
        cx.notify();
    }

    pub fn end_disk_usage_column_resize(&mut self, cx: &mut Context<Self>) {
        let Some(state) = &mut self.disk_usage else { return };
        if state.column_resize_drag.take().is_some() {
            cx.notify();
        }
    }

    pub fn open_disk_usage_entry(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if let Err(err) = open::that_detached(&path) {
            self.op_error = Some(format!("Couldn't open {}: {err}", path.display()));
            cx.notify();
        }
    }

    pub fn trash_disk_usage_entry(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();

        self.show_warning(
            "Move to Trash",
            format!("Are you sure you want to move \"{name}\" to the trash?"),
            "Move to Trash",
            move |explorer, cx| explorer.trash_disk_usage_entry_confirmed(path.clone(), cx),
            cx,
        );
    }

    fn trash_disk_usage_entry_confirmed(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        match trash_ops::trash_capturing(std::slice::from_ref(&path)) {
            Ok(items) => {
                self.push_undo(UndoOp::Trash { original_paths: vec![path], items }, cx);
                self.refresh_scan(cx);
            }
            Err(errors) => {
                self.op_error = Some(format!("Couldn't move to trash: {}", error::describe(&errors)));
                cx.notify();
            }
        }
    }
}
