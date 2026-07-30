use gpui::{
    App, AppContext, DragMoveEvent, Entity, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Pixels, Render, RenderOnce, ScrollHandle, StatefulInteractiveElement, Styled,
    UniformListScrollHandle, Window, div, point, prelude::FluentBuilder as _, px,
};

use crate::explorer::Explorer;
use crate::explorer::drag::ScrollbarId;
use crate::theme;

const TRACK_WIDTH: Pixels = px(10.0);
const THUMB_INSET: Pixels = px(2.0);
const THUMB_MIN_HEIGHT: Pixels = px(24.0);

#[derive(IntoElement)]
pub struct Scrollbar {
    handle: ScrollHandle,
    explorer: Entity<Explorer>,
    id: ScrollbarId,
}

impl Scrollbar {
    pub fn vertical(handle: &ScrollHandle, explorer: Entity<Explorer>, id: ScrollbarId) -> Self {
        Self {
            handle: handle.clone(),
            explorer,
            id,
        }
    }

    pub fn vertical_for_uniform_list(
        handle: &UniformListScrollHandle,
        explorer: Entity<Explorer>,
        id: ScrollbarId,
    ) -> Self {
        Self::vertical(&handle.0.borrow().base_handle.clone(), explorer, id)
    }
}

struct ScrollDragGhost;

impl Render for ScrollDragGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut gpui::Context<Self>) -> impl IntoElement {
        div()
    }
}

fn target_offset_y(
    mouse_y: Pixels,
    track_top: Pixels,
    track_height: Pixels,
    thumb_height: Pixels,
    max_offset_height: Pixels,
    grab_offset: Pixels,
) -> Pixels {
    let usable = (track_height - thumb_height).max(px(1.0));
    let local_y = (mouse_y - track_top - grab_offset).clamp(px(0.0), usable);
    let fraction = local_y / usable;
    -(max_offset_height * fraction)
}

impl RenderOnce for Scrollbar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let id = self.id;
        let base = self.handle;
        let viewport = base.bounds();
        let max_offset_height = base.max_offset().height;
        let has_scroll = max_offset_height > px(0.0);

        let track_height = viewport.size.height;
        let content_height = track_height + max_offset_height;
        let size_ratio = if content_height > px(0.0) {
            track_height / content_height
        } else {
            1.0
        };
        let thumb_height = (track_height * size_ratio)
            .max(THUMB_MIN_HEIGHT)
            .min(track_height);

        let scroll_progress = if has_scroll {
            (-base.offset().y / max_offset_height).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let thumb_top = scroll_progress * (track_height - thumb_height);
        let track_top = viewport.origin.y;

        let jump_handle = base.clone();
        let drag_handle = base.clone();
        let mouse_down_explorer = self.explorer.clone();
        let drag_move_explorer = self.explorer.clone();
        let mouse_up_explorer = self.explorer.clone();

        let dragging = self.explorer.read(cx).scrollbar_grab_offset(id).is_some();

        div()
            .id("scrollbar-track")
            .absolute()
            .top_0()
            .bottom_0()
            .right_0()
            .w(TRACK_WIDTH)
            .opacity(if dragging { 1.0 } else { 0.0 })
            .hover(|style| style.opacity(1.0))
            .active(|style| style.opacity(1.0))
            .when(has_scroll, |el| {
                el.on_mouse_down(MouseButton::Left, move |event, _window, cx| {
                    let local_y = event.position.y - track_top;
                    let grab = if local_y >= thumb_top && local_y <= thumb_top + thumb_height {
                        local_y - thumb_top
                    } else {
                        let center = thumb_height * 0.5;
                        let offset_y = target_offset_y(
                            event.position.y,
                            track_top,
                            track_height,
                            thumb_height,
                            max_offset_height,
                            center,
                        );
                        jump_handle.set_offset(point(jump_handle.offset().x, offset_y));
                        center
                    };
                    mouse_down_explorer.update(cx, |explorer, cx| {
                        explorer.begin_scrollbar_drag(id, grab, cx);
                    });
                })
                .on_drag(id, move |_, _, _window, cx| cx.new(|_| ScrollDragGhost))
                .on_drag_move::<ScrollbarId>(
                    move |event: &DragMoveEvent<ScrollbarId>, _window, cx| {
                        if *event.drag(cx) != id {
                            return;
                        }
                        let Some(grab_offset) =
                            drag_move_explorer.read(cx).scrollbar_grab_offset(id)
                        else {
                            return;
                        };
                        let offset_y = target_offset_y(
                            event.event.position.y,
                            track_top,
                            track_height,
                            thumb_height,
                            max_offset_height,
                            grab_offset,
                        );
                        drag_handle.set_offset(point(drag_handle.offset().x, offset_y));
                    },
                )
                .on_mouse_up(MouseButton::Left, move |_event, _window, cx| {
                    mouse_up_explorer.update(cx, |explorer, cx| {
                        explorer.end_scrollbar_drag(cx);
                    });
                })
                .child(
                    div()
                        .absolute()
                        .top(thumb_top)
                        .left(THUMB_INSET)
                        .right(THUMB_INSET)
                        .h(thumb_height)
                        .rounded_full()
                        .bg(theme::text_faint()),
                )
            })
    }
}
