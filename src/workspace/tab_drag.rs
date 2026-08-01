use gpui::{Bounds, Context, Entity, EntityId, IntoElement, ParentElement, Pixels, Point, Render, Styled, Window, div};

use crate::theme;

use super::pane::Pane;

#[derive(Clone)]
pub struct TabDragPayload {
    pub source_pane: Entity<Pane>,
    pub tab_ix: usize,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct DragHover {
    pub pane_id: EntityId,
    pub zone: SplitZone,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ResizeDragPayload {
    pub path: Vec<usize>,
    pub left_index: usize,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SplitZone {
    Left,
    Right,
    Top,
    Bottom,
    Center,
}

const EDGE_FRACTION: f32 = 0.25;

pub fn zone_for_position(bounds: Bounds<Pixels>, position: Point<Pixels>) -> SplitZone {
    let width = f32::from(bounds.size.width);
    let height = f32::from(bounds.size.height);
    if width <= 0.0 || height <= 0.0 {
        return SplitZone::Center;
    }

    let local_x = f32::from(position.x - bounds.origin.x);
    let local_y = f32::from(position.y - bounds.origin.y);
    let fx = local_x / width;
    let fy = local_y / height;

    if fx < EDGE_FRACTION {
        SplitZone::Left
    } else if fx > 1.0 - EDGE_FRACTION {
        SplitZone::Right
    } else if fy < EDGE_FRACTION {
        SplitZone::Top
    } else if fy > 1.0 - EDGE_FRACTION {
        SplitZone::Bottom
    } else {
        SplitZone::Center
    }
}

pub fn zone_highlight(zone: SplitZone) -> impl IntoElement {
    let mut overlay = div().absolute().bg(theme::drop_target_fill());

    overlay = match zone {
        SplitZone::Left => overlay.top_0().bottom_0().left_0().w(gpui::relative(0.5)),
        SplitZone::Right => overlay.top_0().bottom_0().right_0().w(gpui::relative(0.5)),
        SplitZone::Top => overlay.top_0().left_0().right_0().h(gpui::relative(0.5)),
        SplitZone::Bottom => overlay.bottom_0().left_0().right_0().h(gpui::relative(0.5)),
        SplitZone::Center => overlay.inset_0(),
    };

    overlay
}

pub struct TabDragGhost(pub gpui::SharedString);

impl Render for TabDragGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_3()
            .py_1p5()
            .rounded_md()
            .bg(theme::bg_elevated())
            .border_1()
            .border_color(theme::border())
            .text_sm()
            .child(self.0.clone())
    }
}
