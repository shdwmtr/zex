pub mod bulk_op;
pub mod clipboard_ops;
pub mod columns;
pub mod disk_usage;
pub mod drag;
pub mod git_ops;
mod history;
mod navigation;
mod new_entry;
mod path_edit;
pub mod properties;
mod rename;
mod selection;
mod trash_ops;
mod undo_redo;
mod warnings;

use std::path::{Path, PathBuf};

use gpui::{
    Context, FocusHandle, IntoElement, MouseButton, MouseDownEvent, NavigationDirection, Render,
    ScrollHandle, SharedString, Task, UniformListScrollHandle, Window, div, prelude::*,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::cli::Startup;
use crate::filesystem::entry::FsEntry;
use crate::filesystem::trash_entry::TrashEntry;
use crate::filesystem::undo_op::UndoOp;
use crate::git::GitSnapshot;
use crate::keys;
use crate::settings::{DiskUsageSettings, GitSettings, SidebarItem};
use crate::theme;
use crate::ui;
use crate::ui::{
    bulk_progress, file_list, path_bar, properties_window, sidebar, status_bar, warning_dialog,
};

use bulk_op::BulkOpState;
use clipboard_ops::FileClipboard;
use columns::{
    ColumnResizeDrag, ColumnVisibility as ColumnVisibilityState, ColumnWidths, SortColumn,
    SortDirection,
};
use drag::{DEFAULT_SIDEBAR_WIDTH, ScrollbarDrag, SidebarResizeDrag};
use history::History;
use new_entry::NewEntryState;
use path_edit::PathEditState;
use properties::PropertiesState;
use rename::RenameState;
use selection::BoxSelectDrag;
use warnings::PendingWarning;

pub const TRASH_VIRTUAL_PATH: &str = ":trash";

pub fn trash_virtual_path() -> PathBuf {
    PathBuf::from(TRASH_VIRTUAL_PATH)
}

fn item_label(path: &Path) -> SharedString {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned())
        .into()
}

fn describe_bulk_errors(errors: &[(SharedString, String)]) -> Option<String> {
    if errors.is_empty() {
        return None;
    }
    Some(
        errors
            .iter()
            .map(|(label, message)| format!("{label}: {message}"))
            .collect::<Vec<_>>()
            .join("; "),
    )
}

pub struct Explorer {
    history: History,
    pub entries: Vec<FsEntry>,
    entry_index: FxHashMap<PathBuf, usize>,
    pub trash_entries: Vec<TrashEntry>,
    trash_entry_index: FxHashMap<PathBuf, usize>,
    pub selected: FxHashSet<PathBuf>,
    pub focused_path: Option<PathBuf>,
    pub show_hidden: bool,
    pub sidebar_visible: bool,
    pub error: Option<String>,
    pub op_error: Option<String>,
    pub focus_handle: FocusHandle,
    pub sidebar_entries: Vec<SidebarItem>,
    pub context_menu_target: Option<PathBuf>,
    pub clipboard: Option<FileClipboard>,
    pub warning: Option<PendingWarning>,
    pub active_bulk_op: Option<BulkOpState>,
    pub properties: Option<PropertiesState>,
    pub renaming: Option<RenameState>,
    pub new_entry: Option<NewEntryState>,
    pub editing_path: Option<PathEditState>,
    pub free_space_label: String,
    pub scroll_handle: UniformListScrollHandle,
    pub sidebar_scroll_handle: ScrollHandle,
    pub column_visibility: ColumnVisibilityState,
    pub column_widths: ColumnWidths,
    pub column_resize_drag: Option<ColumnResizeDrag>,
    pub sort_column: SortColumn,
    pub sort_direction: SortDirection,
    pub sidebar_width: f32,
    pub sidebar_resize_drag: Option<SidebarResizeDrag>,
    pub box_select: Option<BoxSelectDrag>,
    pub scrollbar_drag: Option<ScrollbarDrag>,
    undo_stack: Vec<UndoOp>,
    redo_stack: Vec<UndoOp>,
    watcher: Option<notify::RecommendedWatcher>,
    watch_task: Option<Task<()>>,
    free_space_task: Option<Task<()>>,
    pub git_settings: GitSettings,
    pub git_snapshot: Option<GitSnapshot>,
    git_task: Option<Task<()>>,
    git_poll_task: Option<Task<()>>,
    pub disk_usage_settings: DiskUsageSettings,
    pub disk_usage: Option<disk_usage::DiskUsageState>,
}

impl Explorer {
    pub fn new(
        window: &mut Window,
        cx: &mut Context<Self>,
        show_hidden: bool,
        sidebar_visible: bool,
        sidebar_entries: Vec<SidebarItem>,
        git_settings: GitSettings,
        disk_usage_settings: DiskUsageSettings,
        startup: Startup,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle);

        let mut this = Self {
            history: History::new(startup.start_dir),
            entries: Vec::new(),
            entry_index: FxHashMap::default(),
            trash_entries: Vec::new(),
            trash_entry_index: FxHashMap::default(),
            selected: FxHashSet::default(),
            focused_path: None,
            show_hidden,
            sidebar_visible,
            error: None,
            op_error: None,
            focus_handle,
            sidebar_entries,
            context_menu_target: None,
            clipboard: None,
            warning: None,
            active_bulk_op: None,
            properties: None,
            renaming: None,
            new_entry: None,
            editing_path: None,
            free_space_label: String::new(),
            scroll_handle: UniformListScrollHandle::new(),
            sidebar_scroll_handle: ScrollHandle::new(),
            column_visibility: ColumnVisibilityState::default(),
            column_widths: ColumnWidths::default(),
            column_resize_drag: None,
            sort_column: SortColumn::Name,
            sort_direction: SortDirection::Ascending,
            sidebar_width: DEFAULT_SIDEBAR_WIDTH,
            sidebar_resize_drag: None,
            box_select: None,
            scrollbar_drag: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            watcher: None,
            watch_task: None,
            free_space_task: None,
            git_settings,
            git_snapshot: None,
            git_task: None,
            git_poll_task: None,
            disk_usage_settings,
            disk_usage: None,
        };
        this.enter_directory(cx);

        if let Some(select_path) = &startup.select {
            if let Some(entry) = this.entries.iter().find(|entry| &entry.path == select_path) {
                let path = entry.path.clone();
                this.selected = std::iter::once(path.clone()).collect();
                this.focused_path = Some(path);
            }
        }

        if startup.disk_usage {
            this.open_disk_usage(startup.disk_usage_root, window, cx);
        }

        this
    }
}

impl Render for Explorer {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.disk_usage.is_some() {
            ui::disk_usage::render_panel(self, cx).into_any_element()
        } else {
            self.render_browser(window, cx).into_any_element()
        }
    }
}

impl Explorer {
    fn render_browser(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("zex-root")
            .font_weight(cx.global::<theme::UiFont>().weight)
            .track_focus(&self.focus_handle)
            .key_context("Explorer")
            .on_mouse_down(
                MouseButton::Navigate(NavigationDirection::Back),
                cx.listener(|explorer, _: &MouseDownEvent, _window, cx| explorer.go_back(cx)),
            )
            .on_mouse_down(
                MouseButton::Navigate(NavigationDirection::Forward),
                cx.listener(|explorer, _: &MouseDownEvent, _window, cx| explorer.go_forward(cx)),
            )
            .on_action(cx.listener(|explorer, _: &keys::MoveUp, _window, cx| {
                explorer.move_selection(-1, cx)
            }))
            .on_action(cx.listener(|explorer, _: &keys::MoveDown, _window, cx| {
                explorer.move_selection(1, cx)
            }))
            .on_action(
                cx.listener(|explorer, _: &keys::Open, _window, cx| explorer.open_focused(cx)),
            )
            .on_action(cx.listener(|explorer, _: &keys::GoUp, _window, cx| explorer.go_up(cx)))
            .on_action(cx.listener(|explorer, _: &keys::GoBack, _window, cx| explorer.go_back(cx)))
            .on_action(
                cx.listener(|explorer, _: &keys::GoForward, _window, cx| explorer.go_forward(cx)),
            )
            .on_action(
                cx.listener(|explorer, _: &keys::SelectAll, _window, cx| explorer.select_all(cx)),
            )
            .on_action(
                cx.listener(|explorer, _: &keys::ToggleHidden, _window, cx| {
                    explorer.toggle_hidden(cx)
                }),
            )
            .on_action(
                cx.listener(|explorer, _: &keys::ToggleSidebar, _window, cx| {
                    explorer.toggle_sidebar(cx)
                }),
            )
            .on_action(cx.listener(|explorer, _: &keys::Rename, window, cx| {
                if let Some(path) = explorer.focused_path.clone() {
                    explorer.begin_rename(path, window, cx);
                }
            }))
            .on_action(
                cx.listener(|explorer, _: &keys::Delete, _window, cx| {
                    explorer.delete_selection(cx)
                }),
            )
            .on_action(
                cx.listener(|explorer, _: &keys::Copy, _window, cx| explorer.copy_selection(cx)),
            )
            .on_action(
                cx.listener(|explorer, _: &keys::Cut, _window, cx| explorer.cut_selection(cx)),
            )
            .on_action(cx.listener(|explorer, _: &keys::Paste, _window, cx| explorer.paste(cx)))
            .on_action(cx.listener(|explorer, _: &keys::Undo, _window, cx| explorer.undo(cx)))
            .on_action(cx.listener(|explorer, _: &keys::Redo, _window, cx| explorer.redo(cx)))
            .on_action(cx.listener(|explorer, _: &keys::NewFolder, window, cx| {
                explorer.begin_new_folder(window, cx)
            }))
            .on_action(cx.listener(|explorer, _: &keys::NewFile, window, cx| {
                explorer.begin_new_file(window, cx)
            }))
            .size_full()
            .relative()
            .flex()
            .flex_row()
            .bg(theme::bg_window())
            .text_color(theme::text_primary())
            .when(self.sidebar_should_render(), |this| {
                this.child(sidebar::render(self, cx))
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .size_full()
                    .bg(theme::bg_root())
                    .child(path_bar::render(self, cx))
                    .children(self.op_error.as_ref().map(|message| {
                        div()
                            .relative()
                            .w_full()
                            .py_2()
                            .pl_3()
                            .pr_8()
                            .bg(theme::bg_on_error())
                            .border_1()
                            .border_color(theme::text_error())
                            .text_color(theme::text_on_error())
                            .child(div().whitespace_normal().child(message.clone()))
                            .child(
                                div()
                                    .id("dismiss-op-error")
                                    .absolute()
                                    .top_1()
                                    .right_1()
                                    .cursor_pointer()
                                    .px_2()
                                    .on_click(cx.listener(|explorer, _, _, cx| {
                                        explorer.dismiss_op_error(cx)
                                    }))
                                    .child("×"),
                            )
                    }))
                    .child(file_list::render(self, cx))
                    .child(status_bar::render(self, cx)),
            )
            .when(self.sidebar_should_render(), |this| {
                this.child(sidebar::resize_handle(
                    cx.entity(),
                    self.sidebar_width,
                    self.sidebar_resize_drag.is_some(),
                    cx,
                ))
            })
            .children(warning_dialog::render(self, cx))
            .children(bulk_progress::render(self, cx))
            .children(properties_window::render(self, cx))
    }
}
