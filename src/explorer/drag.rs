use gpui::{Context, Pixels};

use super::Explorer;

pub const MIN_SIDEBAR_WIDTH: f32 = 140.0;
pub const MAX_SIDEBAR_WIDTH: f32 = 480.0;
pub const DEFAULT_SIDEBAR_WIDTH: f32 = 300.0;

#[derive(Clone, Copy, Debug)]
pub struct SidebarResizeDrag {
    pub anchor_x: f32,
    pub start_width: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ScrollbarId {
    FileList,
    Sidebar,
}

#[derive(Clone, Copy, Debug)]
pub struct ScrollbarDrag {
    pub id: ScrollbarId,
    pub grab_offset: Pixels,
}

impl Explorer {
    pub fn begin_sidebar_resize(&mut self, anchor_x: f32, cx: &mut Context<Self>) {
        self.sidebar_resize_drag = Some(SidebarResizeDrag {
            anchor_x,
            start_width: self.sidebar_width,
        });
        cx.notify();
    }

    pub fn update_sidebar_resize(&mut self, current_x: f32, cx: &mut Context<Self>) {
        let Some(drag) = self.sidebar_resize_drag else {
            return;
        };
        let new_width = drag.start_width + (current_x - drag.anchor_x);
        self.sidebar_width = new_width.clamp(MIN_SIDEBAR_WIDTH, MAX_SIDEBAR_WIDTH);
        cx.notify();
    }

    pub fn end_sidebar_resize(&mut self, cx: &mut Context<Self>) {
        if self.sidebar_resize_drag.take().is_some() {
            cx.notify();
        }
    }

    pub fn begin_scrollbar_drag(
        &mut self,
        id: ScrollbarId,
        grab_offset: Pixels,
        cx: &mut Context<Self>,
    ) {
        self.scrollbar_drag = Some(ScrollbarDrag { id, grab_offset });
        cx.notify();
    }

    pub fn scrollbar_grab_offset(&self, id: ScrollbarId) -> Option<Pixels> {
        self.scrollbar_drag
            .filter(|drag| drag.id == id)
            .map(|drag| drag.grab_offset)
    }

    pub fn end_scrollbar_drag(&mut self, cx: &mut Context<Self>) {
        if self.scrollbar_drag.take().is_some() {
            cx.notify();
        }
    }
}
