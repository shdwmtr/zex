use gpui::{
    AnyElement, Context, Entity, EntityId, InteractiveElement, IntoElement, MouseButton,
    ParentElement, SharedString, Styled, deferred, div, prelude::*, px, relative,
};

use crate::theme;

use super::Workspace;
use super::pane::Pane;
use super::tab_drag::{DragHover, ResizeDragPayload, TabDragPayload, zone_for_position, zone_highlight};

pub const MIN_PANE_FRACTION: f32 = 0.15;

const DIVIDER_LINE_WIDTH: f32 = 2.0;
const DIVIDER_HIT_WIDTH: f32 = 9.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Axis {
    /// Children side by side (a left/right split).
    Horizontal,
    /// Children stacked (a top/bottom split).
    Vertical,
}

pub enum SplitNode {
    Leaf(Entity<Pane>),
    Split {
        axis: Axis,
        children: Vec<(SplitNode, f32)>,
    },
}

impl SplitNode {
    pub fn leaves(&self) -> Vec<Entity<Pane>> {
        match self {
            SplitNode::Leaf(pane) => vec![pane.clone()],
            SplitNode::Split { children, .. } => {
                children.iter().flat_map(|(child, _)| child.leaves()).collect()
            }
        }
    }

    pub fn is_leaf(&self, id: EntityId) -> bool {
        matches!(self, SplitNode::Leaf(pane) if pane.entity_id() == id)
    }

    pub fn find_leaf_mut(&mut self, id: EntityId) -> Option<&mut SplitNode> {
        match self {
            SplitNode::Leaf(pane) => {
                if pane.entity_id() == id {
                    Some(self)
                } else {
                    None
                }
            }
            SplitNode::Split { children, .. } => {
                for (child, _) in children.iter_mut() {
                    if let Some(found) = child.find_leaf_mut(id) {
                        return Some(found);
                    }
                }
                None
            }
        }
    }

    pub fn at_path_mut(&mut self, path: &[usize]) -> Option<&mut SplitNode> {
        let mut node = self;
        for &ix in path {
            let SplitNode::Split { children, .. } = node else {
                return None;
            };
            node = &mut children.get_mut(ix)?.0;
        }
        Some(node)
    }

    /// Splits the leaf identified by `target` into a new 2-way split along `axis`, placing
    /// `new_pane` before (`new_first`) or after the pane that was there.
    pub fn split_leaf(&mut self, target: EntityId, axis: Axis, new_pane: Entity<Pane>, new_first: bool) -> bool {
        let Some(slot) = self.find_leaf_mut(target) else {
            return false;
        };
        let SplitNode::Leaf(existing) = slot else {
            return false;
        };
        let existing_leaf = SplitNode::Leaf(existing.clone());
        let new_leaf = SplitNode::Leaf(new_pane);
        *slot = if new_first {
            SplitNode::Split {
                axis,
                children: vec![(new_leaf, 0.5), (existing_leaf, 0.5)],
            }
        } else {
            SplitNode::Split {
                axis,
                children: vec![(existing_leaf, 0.5), (new_leaf, 0.5)],
            }
        };
        true
    }

    /// Removes the leaf identified by `id` from this subtree, collapsing any split left with a
    /// single child. Returns true if something was removed. Note: a bare root `Leaf` can't
    /// remove itself this way — callers must check `is_leaf` first and veto in that case.
    pub fn remove_leaf(&mut self, id: EntityId) -> bool {
        let SplitNode::Split { children, .. } = self else {
            return false;
        };
        if let Some(pos) = children
            .iter()
            .position(|(child, _)| matches!(child, SplitNode::Leaf(pane) if pane.entity_id() == id))
        {
            children.remove(pos);
            if children.len() == 1 {
                let (only_child, _) = children.pop().unwrap();
                *self = only_child;
            } else {
                let total: f32 = children.iter().map(|(_, fraction)| *fraction).sum();
                if total > 0.0 {
                    for (_, fraction) in children.iter_mut() {
                        *fraction /= total;
                    }
                }
            }
            return true;
        }
        for (child, _) in children.iter_mut() {
            if child.remove_leaf(id) {
                return true;
            }
        }
        false
    }
}

fn path_id(path: &[usize]) -> String {
    path.iter().map(|ix| ix.to_string()).collect::<Vec<_>>().join("-")
}

pub fn render(state: &Workspace, node: &SplitNode, path: Vec<usize>, workspace: &Entity<Workspace>) -> AnyElement {
    match node {
        SplitNode::Leaf(pane) => render_leaf(state, pane.clone(), workspace),
        SplitNode::Split { axis, children } => render_split(state, *axis, children, path, workspace),
    }
}

fn render_leaf(state: &Workspace, pane: Entity<Pane>, workspace: &Entity<Workspace>) -> AnyElement {
    let pane_id = pane.entity_id();
    let hovered_zone = state
        .drag_hover
        .as_ref()
        .filter(|hover| hover.pane_id == pane_id)
        .map(|hover| hover.zone);

    let drag_move_workspace = workspace.clone();
    let drop_workspace = workspace.clone();
    let drop_pane = pane.clone();

    div()
        .id(("split-leaf", pane_id))
        .relative()
        .size_full()
        .min_w(px(0.0))
        .min_h(px(0.0))
        .on_drag_move::<TabDragPayload>(move |event, _window, cx| {
            drag_move_workspace.update(cx, |workspace, cx| {
                workspace.track_dragging_tab(event.drag(cx).clone());
                if event.bounds.contains(&event.event.position) {
                    let zone = zone_for_position(event.bounds, event.event.position);
                    workspace.drag_hover = Some(DragHover { pane_id, zone });
                } else if workspace.drag_hover.as_ref().is_some_and(|hover| hover.pane_id == pane_id) {
                    workspace.drag_hover = None;
                }
                cx.notify();
            });
        })
        .on_drop::<TabDragPayload>(move |dragged: &TabDragPayload, _window, cx| {
            let dragged = dragged.clone();
            let drop_pane = drop_pane.clone();
            drop_workspace.update(cx, |workspace, cx| {
                workspace.handle_tab_drop(dragged, drop_pane, cx);
            });
        })
        .child(pane.clone())
        .when_some(hovered_zone, |el, zone| el.child(zone_highlight(zone)))
        .into_any_element()
}

fn render_split(
    state: &Workspace,
    axis: Axis,
    children: &[(SplitNode, f32)],
    path: Vec<usize>,
    workspace: &Entity<Workspace>,
) -> AnyElement {
    let mut container = div()
        .id(SharedString::from(format!("split-container-{}", path_id(&path))))
        .relative()
        .size_full()
        .flex();
    container = match axis {
        Axis::Horizontal => container.flex_row(),
        Axis::Vertical => container.flex_col(),
    };

    let move_workspace = workspace.clone();
    let container_path = path.clone();
    container = container.on_drag_move::<ResizeDragPayload>(move |event, _window, cx| {
        let payload = event.drag(cx);
        if payload.path != container_path {
            return;
        }
        let left_index = payload.left_index;
        let total = match axis {
            Axis::Horizontal => f32::from(event.bounds.size.width),
            Axis::Vertical => f32::from(event.bounds.size.height),
        };
        let mouse = match axis {
            Axis::Horizontal => f32::from(event.event.position.x),
            Axis::Vertical => f32::from(event.event.position.y),
        };
        move_workspace.update(cx, |workspace, cx| {
            workspace.update_split_resize(left_index, mouse, total, cx);
        });
    });

    let child_count = children.len();
    for (ix, (child, fraction)) in children.iter().enumerate() {
        let mut child_path = path.clone();
        child_path.push(ix);
        let child_el = render(state, child, child_path, workspace);

        let mut wrapper = div().relative().min_w(px(0.0)).min_h(px(0.0));
        wrapper = match axis {
            Axis::Horizontal => wrapper.w(relative(*fraction)).h_full(),
            Axis::Vertical => wrapper.h(relative(*fraction)).w_full(),
        };
        container = container.child(wrapper.child(child_el));

        if ix + 1 < child_count {
            container = container.child(render_divider(state, axis, path.clone(), ix, workspace));
        }
    }

    container.into_any_element()
}

fn render_divider(
    state: &Workspace,
    axis: Axis,
    path: Vec<usize>,
    left_index: usize,
    workspace: &Entity<Workspace>,
) -> AnyElement {
    let is_dragging = state
        .split_resize
        .as_ref()
        .is_some_and(|resize| resize.path == path && resize.left_index == left_index);

    let payload = ResizeDragPayload {
        path: path.clone(),
        left_index,
    };
    let begin_workspace = workspace.clone();
    let end_workspace = workspace.clone();

    let line_color = if is_dragging {
        theme::bg_selected()
    } else {
        theme::border()
    };

    let hit_inset = px(-(DIVIDER_HIT_WIDTH - DIVIDER_LINE_WIDTH) / 2.0);

    let mut hit_area = div()
        .id(SharedString::from(format!(
            "split-divider-{}-{left_index}",
            path_id(&path)
        )))
        .absolute()
        .occlude();

    hit_area = match axis {
        Axis::Horizontal => hit_area
            .top_0()
            .bottom_0()
            .left(hit_inset)
            .w(px(DIVIDER_HIT_WIDTH))
            .cursor_col_resize(),
        Axis::Vertical => hit_area
            .left_0()
            .right_0()
            .top(hit_inset)
            .h(px(DIVIDER_HIT_WIDTH))
            .cursor_row_resize(),
    };

    hit_area = hit_area
        .on_drag(payload, move |_, _point, window, cx| {
            let anchor = match axis {
                Axis::Horizontal => f32::from(window.mouse_position().x),
                Axis::Vertical => f32::from(window.mouse_position().y),
            };
            begin_workspace.update(cx, |workspace, cx| {
                workspace.begin_split_resize(path.clone(), left_index, anchor, cx);
            });
            cx.new(|_| SplitResizeGhost)
        })
        .on_mouse_up(MouseButton::Left, {
            let end_workspace = end_workspace.clone();
            move |_event, _window, cx| {
                end_workspace.update(cx, |workspace, cx| {
                    workspace.end_split_resize(cx);
                });
            }
        })
        .on_mouse_up_out(MouseButton::Left, move |_event, _window, cx| {
            end_workspace.update(cx, |workspace, cx| {
                workspace.end_split_resize(cx);
            });
        });

    let mut handle = div().relative().flex_shrink_0().bg(line_color);

    handle = match axis {
        Axis::Horizontal => handle.w(px(DIVIDER_LINE_WIDTH)).h_full(),
        Axis::Vertical => handle.h(px(DIVIDER_LINE_WIDTH)).w_full(),
    };

    handle
        .child(deferred(hit_area).with_priority(1))
        .into_any_element()
}

struct SplitResizeGhost;

impl gpui::Render for SplitResizeGhost {
    fn render(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}
