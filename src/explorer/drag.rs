use gpui::{Context, Pixels};

use super::Explorer;

#[derive(Clone, Copy, Debug)]
pub struct WidthResizeDrag {
    pub anchor_x: f32,
    pub start_width: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ScrollbarId {
    FileList,
    Sidebar,
    DiskUsageTree,
}

#[derive(Clone, Copy, Debug)]
pub struct ScrollbarDrag {
    pub id: ScrollbarId,
    pub grab_offset: Pixels,
}

pub trait ScrollDragHost: Sized {
    fn begin_scrollbar_drag(&mut self, id: ScrollbarId, grab_offset: Pixels, cx: &mut Context<Self>);
    fn scrollbar_grab_offset(&self, id: ScrollbarId) -> Option<Pixels>;
    fn end_scrollbar_drag(&mut self, cx: &mut Context<Self>);
}

impl ScrollDragHost for Explorer {
    fn begin_scrollbar_drag(&mut self, id: ScrollbarId, grab_offset: Pixels, cx: &mut Context<Self>) {
        self.scrollbar_drag = Some(ScrollbarDrag { id, grab_offset });
        cx.notify();
    }

    fn scrollbar_grab_offset(&self, id: ScrollbarId) -> Option<Pixels> {
        self.scrollbar_drag
            .filter(|drag| drag.id == id)
            .map(|drag| drag.grab_offset)
    }

    fn end_scrollbar_drag(&mut self, cx: &mut Context<Self>) {
        if self.scrollbar_drag.take().is_some() {
            cx.notify();
        }
    }
}
