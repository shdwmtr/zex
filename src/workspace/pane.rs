use std::path::PathBuf;

use gpui::{
    AnyElement, ClickEvent, Context, Entity, EventEmitter, IntoElement, ParentElement, Render,
    SharedString, Styled, Window, div, prelude::*, px,
};

use crate::explorer::shared_state::SharedState;
use crate::explorer::{Explorer, item_label};
use crate::settings::{DiskUsageSettings, GitSettings};
use crate::theme;
use crate::ui::path_bar::{self, NavDirection};
use crate::ui::popup_menu::{ContextMenuExt, PopupMenu, PopupMenuItem};

use super::tab_drag::{TabDragGhost, TabDragPayload};

pub enum PaneEvent {
    /// The pane's last tab was closed and it would like to be removed from the split tree.
    /// The workspace vetoes this if the pane is the only one left.
    Empty,
    /// A tab was dropped directly onto a pane's tab strip. This bypasses `Workspace::handle_tab_drop`,
    /// so it's the tab strip's responsibility to tell the workspace the drag ended.
    TabDropped,
}

pub struct Pane {
    pub tabs: Vec<Entity<Explorer>>,
    pub active_index: usize,
    /// Whether this pane should show its tab strip. Driven by `Workspace`, which shows it in
    /// every pane once there's more than one tab open anywhere in the workspace, not just in
    /// this particular pane.
    pub show_tab_strip: bool,
    git_settings: GitSettings,
    disk_usage_settings: DiskUsageSettings,
    shared: Entity<SharedState>,
}

impl EventEmitter<PaneEvent> for Pane {}

fn tab_label(explorer: &Explorer) -> SharedString {
    if explorer.is_trash() {
        return "Trash".into();
    }
    if let Some(state) = &explorer.disk_usage {
        return item_label(&state.current_root);
    }
    item_label(explorer.current_dir())
}

fn tab_path(explorer: &Explorer) -> Option<PathBuf> {
    if explorer.is_trash() {
        return None;
    }
    if let Some(state) = &explorer.disk_usage {
        return Some(state.current_root.clone());
    }
    Some(explorer.current_dir().to_path_buf())
}

fn tab_context_menu(
    pane: Entity<Pane>,
    tab: Entity<Explorer>,
    index: usize,
    is_first: bool,
    is_last: bool,
    menu: PopupMenu,
    _window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    let close_pane = pane.clone();
    let close_others_pane = pane.clone();
    let close_left_pane = pane.clone();
    let close_right_pane = pane.clone();
    let close_all_pane = pane.clone();
    let copy_path_tab = tab.clone();
    let can_copy_path = tab_path(tab.read(cx)).is_some();

    menu.item(
        PopupMenuItem::new("Close").on_click(move |_, _window, cx| {
            close_pane.update(cx, |pane, cx| pane.close_tab(index, cx));
        }),
    )
    .item(
        PopupMenuItem::new("Close Others")
            .disabled(is_first && is_last)
            .on_click(move |_, _window, cx| {
                close_others_pane.update(cx, |pane, cx| pane.close_other_tabs(index, cx));
            }),
    )
    .item(
        PopupMenuItem::new("Close Left")
            .disabled(is_first)
            .on_click(move |_, _window, cx| {
                close_left_pane.update(cx, |pane, cx| pane.close_tabs_to_left(index, cx));
            }),
    )
    .item(
        PopupMenuItem::new("Close Right")
            .disabled(is_last)
            .on_click(move |_, _window, cx| {
                close_right_pane.update(cx, |pane, cx| pane.close_tabs_to_right(index, cx));
            }),
    )
    .item(
        PopupMenuItem::new("Close All").on_click(move |_, _window, cx| {
            close_all_pane.update(cx, |pane, cx| pane.close_all_tabs(cx));
        }),
    )
    .separator()
    .item(
        PopupMenuItem::new("Copy Path")
            .disabled(!can_copy_path)
            .on_click(move |_, _window, cx| {
                copy_path_tab.update(cx, |explorer, cx| {
                    if let Some(path) = tab_path(explorer) {
                        explorer.copy_paths_to_clipboard(&[path], cx);
                    }
                });
            }),
    )
}

impl Pane {
    pub fn new(
        explorer: Entity<Explorer>,
        git_settings: GitSettings,
        disk_usage_settings: DiskUsageSettings,
        shared: Entity<SharedState>,
        cx: &mut Context<Self>,
    ) -> Self {
        let this_pane = cx.entity();
        explorer.update(cx, |explorer, cx| {
            explorer.pane = Some(this_pane);
            cx.notify();
        });
        Self {
            tabs: vec![explorer],
            active_index: 0,
            show_tab_strip: false,
            git_settings,
            disk_usage_settings,
            shared,
        }
    }

    pub fn active_tab(&self) -> Entity<Explorer> {
        self.tabs[self.active_index].clone()
    }

    fn claim(&self, explorer: &Entity<Explorer>, cx: &mut Context<Self>) {
        let this_pane = cx.entity();
        explorer.update(cx, |explorer, cx| {
            explorer.pane = Some(this_pane);
            cx.notify();
        });
    }

    pub fn spawn_new_tab(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let active = self.active_tab();
        let (start_dir, show_hidden) = {
            let active = active.read(cx);
            (active.current_dir().to_path_buf(), active.show_hidden)
        };
        self.spawn_new_tab_at(start_dir, show_hidden, window, cx);
    }

    /// Opens `start_dir` as a brand-new tab in this pane and activates it.
    pub fn spawn_new_tab_at(
        &mut self,
        start_dir: PathBuf,
        show_hidden: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let explorer = cx.new(|cx| {
            Explorer::new_tab(
                window,
                cx,
                start_dir,
                show_hidden,
                self.git_settings.clone(),
                self.disk_usage_settings.clone(),
                self.shared.clone(),
            )
        });
        self.claim(&explorer, cx);
        self.tabs.push(explorer);
        self.active_index = self.tabs.len() - 1;
        cx.notify();
    }

    /// Inserts an already-open tab (dragged in from another pane) at `index` and activates it.
    pub fn insert_tab(&mut self, index: usize, explorer: Entity<Explorer>, cx: &mut Context<Self>) {
        let index = index.min(self.tabs.len());
        self.claim(&explorer, cx);
        self.tabs.insert(index, explorer);
        self.active_index = index;
        cx.notify();
    }

    /// Removes and returns the tab at `index` unconditionally, for relocating it elsewhere.
    /// Emits `PaneEvent::Empty` if this was the pane's last tab.
    pub fn take_tab(&mut self, index: usize, cx: &mut Context<Self>) -> Option<Entity<Explorer>> {
        if index >= self.tabs.len() {
            return None;
        }
        let tab = self.tabs.remove(index);
        if self.tabs.is_empty() {
            cx.emit(PaneEvent::Empty);
        } else {
            if self.active_index >= self.tabs.len() {
                self.active_index = self.tabs.len() - 1;
            } else if index < self.active_index {
                self.active_index -= 1;
            }
            cx.notify();
        }
        Some(tab)
    }

    pub fn close_tab(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.tabs.len() {
            return;
        }
        if self.tabs.len() == 1 {
            cx.emit(PaneEvent::Empty);
            return;
        }
        self.tabs.remove(index);
        if self.active_index >= self.tabs.len() {
            self.active_index = self.tabs.len() - 1;
        } else if index < self.active_index {
            self.active_index -= 1;
        }
        cx.notify();
    }

    pub fn close_active_tab(&mut self, cx: &mut Context<Self>) {
        self.close_tab(self.active_index, cx);
    }

    pub fn close_other_tabs(&mut self, index: usize, cx: &mut Context<Self>) {
        if index >= self.tabs.len() || self.tabs.len() <= 1 {
            return;
        }
        let keep = self.tabs.remove(index);
        self.tabs.clear();
        self.tabs.push(keep);
        self.active_index = 0;
        cx.notify();
    }

    pub fn close_tabs_to_left(&mut self, index: usize, cx: &mut Context<Self>) {
        if index == 0 || index >= self.tabs.len() {
            return;
        }
        let active_id = self.tabs[self.active_index].entity_id();
        self.tabs.drain(0..index);
        self.active_index = self
            .tabs
            .iter()
            .position(|tab| tab.entity_id() == active_id)
            .unwrap_or(0);
        cx.notify();
    }

    pub fn close_tabs_to_right(&mut self, index: usize, cx: &mut Context<Self>) {
        if index + 1 >= self.tabs.len() {
            return;
        }
        let active_id = self.tabs[self.active_index].entity_id();
        self.tabs.truncate(index + 1);
        self.active_index = self
            .tabs
            .iter()
            .position(|tab| tab.entity_id() == active_id)
            .unwrap_or(self.tabs.len() - 1);
        cx.notify();
    }

    /// Closes every tab in this pane. Mirrors `close_tab`'s handling of the last remaining
    /// tab: the pane doesn't touch its own state and instead defers to the workspace, which
    /// removes the pane outright or vetoes the close if it's the only pane left.
    pub fn close_all_tabs(&mut self, cx: &mut Context<Self>) {
        cx.emit(PaneEvent::Empty);
    }

    pub fn activate(&mut self, index: usize, cx: &mut Context<Self>) {
        if index < self.tabs.len() && index != self.active_index {
            self.active_index = index;
            cx.notify();
        }
    }

    pub fn next_tab(&mut self, cx: &mut Context<Self>) {
        if self.tabs.len() > 1 {
            self.active_index = (self.active_index + 1) % self.tabs.len();
            cx.notify();
        }
    }

    pub fn prev_tab(&mut self, cx: &mut Context<Self>) {
        if self.tabs.len() > 1 {
            self.active_index = (self.active_index + self.tabs.len() - 1) % self.tabs.len();
            cx.notify();
        }
    }

    pub fn reorder_tab(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        if from == to || from >= self.tabs.len() || to >= self.tabs.len() {
            return;
        }
        let active_id = self.tabs[self.active_index].entity_id();
        let tab = self.tabs.remove(from);
        self.tabs.insert(to, tab);
        if let Some(new_active) = self
            .tabs
            .iter()
            .position(|tab| tab.entity_id() == active_id)
        {
            self.active_index = new_active;
        }
        cx.notify();
    }

    fn render_tab(&self, index: usize, tab: &Entity<Explorer>, cx: &Context<Self>) -> AnyElement {
        let is_active = index == self.active_index;
        let is_first = index == 0;
        let is_last = index == self.tabs.len() - 1;
        let label = tab_label(tab.read(cx));
        let ghost_label = label.clone();
        let self_pane = cx.entity();
        let menu_pane = self_pane.clone();
        let menu_tab = tab.clone();

        div()
            .id(("tab", index))
            .group(format!("tab-{index}"))
            .relative()
            .h_full()
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .px_2()
            .max_w(px(280.0))
            .border_l_1()
            .when(is_last, |el| el.border_r_1())
            .border_color(theme::border())
            .when(is_active, |el| el.bg(theme::bg_root()))
            .on_click(cx.listener(move |pane, _event: &ClickEvent, _window, cx| {
                pane.activate(index, cx);
            }))
            .on_drag(
                TabDragPayload {
                    source_pane: self_pane.clone(),
                    tab_ix: index,
                },
                move |_, _point, _window, cx| cx.new(|_| TabDragGhost(ghost_label.clone())),
            )
            .drag_over::<TabDragPayload>(|style, _dragged, _window, _cx| {
                style.border_color(theme::drop_target_border())
            })
            .context_menu(move |menu, window, cx| {
                tab_context_menu(
                    menu_pane.clone(),
                    menu_tab.clone(),
                    index,
                    is_first,
                    is_last,
                    menu,
                    window,
                    cx,
                )
            })
            .on_drop::<TabDragPayload>(cx.listener(
                move |pane, dragged: &TabDragPayload, _window, cx| {
                    if dragged.source_pane.entity_id() == cx.entity().entity_id() {
                        pane.reorder_tab(dragged.tab_ix, index, cx);
                    } else if let Some(explorer) = dragged
                        .source_pane
                        .update(cx, |source, cx| source.take_tab(dragged.tab_ix, cx))
                    {
                        pane.insert_tab(index, explorer, cx);
                    }
                    cx.emit(PaneEvent::TabDropped);
                    cx.notify();
                },
            ))
            .child(div().w(px(18.0)).flex_shrink_0())
            .child(
                div()
                    .flex_1()
                    .min_w(px(0.0))
                    .truncate()
                    .text_center()
                    .text_base()
                    .text_color(theme::text_primary())
                    .child(label),
            )
            .child(
                div()
                    .id(("close-tab", index))
                    .flex_shrink_0()
                    .size(px(18.0))
                    .flex()
                    .items_center()
                    .justify_center()
                    .overflow_hidden()
                    .cursor_pointer()
                    .rounded_md()
                    .text_2xl()
                    .text_color(theme::text_faint())
                    .opacity(0.0)
                    .group_hover(format!("tab-{index}"), |style| style.opacity(1.0))
                    .hover(|style| {
                        style
                            .bg(theme::bg_hover())
                            .text_color(theme::text_primary())
                    })
                    .on_click(cx.listener(move |pane, _event: &ClickEvent, _window, cx| {
                        cx.stop_propagation();
                        pane.close_tab(index, cx);
                    }))
                    .child("×"),
            )
            .when(!is_active, |el| {
                el.child(
                    div()
                        .absolute()
                        .bottom_0()
                        .left_0()
                        .right_0()
                        .h(px(1.0))
                        .bg(theme::border()),
                )
            })
            .into_any_element()
    }

    fn render_tab_strip(
        &self,
        active: &Entity<Explorer>,
        window: &Window,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let can_go_back = active.read(cx).can_go_back();
        let can_go_forward = active.read(cx).can_go_forward();
        let bg = if window.is_window_active() {
            theme::bg_title_bar()
        } else {
            theme::bg_title_bar_inactive()
        };

        let bordered_run = |children: Vec<AnyElement>| {
            div()
                .h_full()
                .flex()
                .flex_row()
                .items_center()
                .border_b_1()
                .border_color(theme::border())
                .children(children)
        };

        let tabs: Vec<AnyElement> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(ix, tab)| self.render_tab(ix, tab, cx))
            .collect();

        div()
            .id("tab-strip")
            .w_full()
            .h(px(36.0))
            .flex()
            .flex_row()
            .items_center()
            .bg(bg)
            .child(
                bordered_run(vec![
                    path_bar::nav_button(
                        "go-back",
                        "icons/arrow-left.svg",
                        can_go_back,
                        NavDirection::Back,
                        active,
                    ),
                    path_bar::nav_button(
                        "go-forward",
                        "icons/arrow-right.svg",
                        can_go_forward,
                        NavDirection::Forward,
                        active,
                    ),
                ])
                .gap_1()
                .px_2(),
            )
            .child(
                div()
                    .id("tabs")
                    .flex()
                    .flex_row()
                    .items_center()
                    .flex_1()
                    .h_full()
                    .overflow_x_scroll()
                    .children(tabs)
                    .child(
                        div()
                            .flex_1()
                            .h_full()
                            .border_b_1()
                            .border_color(theme::border()),
                    ),
            )
    }
}

impl Render for Pane {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active = self.active_tab();
        let show_tab_strip = self.show_tab_strip;

        let want_nav = !show_tab_strip;
        for tab in &self.tabs {
            tab.update(cx, |explorer, cx| {
                if explorer.show_path_bar_nav != want_nav {
                    explorer.show_path_bar_nav = want_nav;
                    cx.notify();
                }
            });
        }

        div()
            .id("pane")
            .size_full()
            .flex()
            .flex_col()
            .bg(theme::bg_window())
            .when(show_tab_strip, |el| {
                el.child(self.render_tab_strip(&active, window, cx))
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_h(px(0.0))
                    .w_full()
                    .child(active),
            )
    }
}
