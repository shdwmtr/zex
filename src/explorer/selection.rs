use std::path::{Path, PathBuf};

use gpui::{Context, Modifiers, Pixels, Point};
use rustc_hash::FxHashSet;

use super::Explorer;

#[derive(Clone, Debug)]
pub struct BoxSelectDrag {
    pub anchor: Point<Pixels>,
    pub current: Point<Pixels>,
    pub initial_selection: FxHashSet<PathBuf>,
}

impl Explorer {
    fn row_count(&self) -> usize {
        if self.is_trash() {
            self.trash_entries.len()
        } else {
            self.entries.len()
        }
    }

    fn row_path_at(&self, ix: usize) -> Option<PathBuf> {
        if self.is_trash() {
            self.trash_entries
                .get(ix)
                .map(|entry| entry.id_path.clone())
        } else {
            self.entries.get(ix).map(|entry| entry.path.clone())
        }
    }

    fn row_index_of(&self, path: &Path) -> Option<usize> {
        if self.is_trash() {
            self.trash_entry_index.get(path).copied()
        } else {
            self.entry_index.get(path).copied()
        }
    }

    fn row_height_px(&self) -> Pixels {
        let row_count = self.row_count();
        match self.scroll_handle.0.borrow().last_item_size {
            Some(size) if row_count > 0 => {
                gpui::px(f32::from(size.contents.height) / row_count as f32)
            }
            _ => gpui::px(28.0),
        }
    }

    fn row_range_for_screen_y(&self, top: Pixels, bottom: Pixels) -> std::ops::Range<usize> {
        let base = self.scroll_handle.0.borrow().base_handle.clone();
        let viewport = base.bounds();
        let offset = base.offset();
        let row_height = self.row_height_px();

        let content_top = top - viewport.origin.y - offset.y;
        let content_bottom = bottom - viewport.origin.y - offset.y;

        let start_ix = (content_top / row_height).max(0.0).floor() as usize;
        let end_ix = ((content_bottom / row_height).max(0.0).ceil() as usize).min(self.row_count());
        start_ix..end_ix.max(start_ix)
    }

    fn row_index_for_screen_point(&self, y: Pixels) -> Option<usize> {
        let base = self.scroll_handle.0.borrow().base_handle.clone();
        let viewport = base.bounds();
        let offset = base.offset();
        let row_height = self.row_height_px();

        let content_y = y - viewport.origin.y - offset.y;
        if content_y < gpui::px(0.0) {
            return None;
        }
        let ix = (content_y / row_height).floor() as usize;
        (ix < self.row_count()).then_some(ix)
    }

    pub fn begin_box_select(
        &mut self,
        origin: Point<Pixels>,
        modifiers: Modifiers,
        cx: &mut Context<Self>,
    ) {
        let initial_selection = if modifiers.control {
            self.selected.clone()
        } else {
            self.selected.clear();
            FxHashSet::default()
        };
        self.box_select = Some(BoxSelectDrag {
            anchor: origin,
            current: origin,
            initial_selection,
        });
        cx.notify();
    }

    pub fn update_box_select(&mut self, current: Point<Pixels>, cx: &mut Context<Self>) {
        let Some(drag) = self.box_select.as_mut() else {
            return;
        };
        drag.current = current;
        let anchor = drag.anchor;
        let initial_selection = drag.initial_selection.clone();

        let screen_top = anchor.y.min(current.y);
        let screen_bottom = anchor.y.max(current.y);
        let range = self.row_range_for_screen_y(screen_top, screen_bottom);

        let mut selected = initial_selection;
        let mut last_path = None;
        for ix in range {
            if let Some(path) = self.row_path_at(ix) {
                selected.insert(path.clone());
                last_path = Some(path);
            }
        }
        self.selected = selected;
        if let Some(path) = last_path {
            self.focused_path = Some(path);
        }
        cx.notify();
    }

    pub fn end_box_select(&mut self, cx: &mut Context<Self>) {
        if self.box_select.take().is_some() {
            cx.notify();
        }
    }

    pub fn click_empty_space(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        if self.row_index_for_screen_point(position.y).is_some() {
            return;
        }
        if self.selected.is_empty() && self.focused_path.is_none() {
            return;
        }
        self.selected.clear();
        self.focused_path = None;
        cx.notify();
    }

    pub fn mouse_down_select(
        &mut self,
        path: PathBuf,
        modifiers: Modifiers,
        click_count: usize,
        cx: &mut Context<Self>,
    ) {
        if modifiers.control {
            if !self.selected.remove(&path) {
                self.selected.insert(path.clone());
            }
            self.focused_path = Some(path);
        } else if modifiers.shift {
            let anchor = self.focused_path.clone().unwrap_or_else(|| path.clone());
            let positions = (self.row_index_of(&anchor), self.row_index_of(&path));
            match positions {
                (Some(from), Some(to)) => {
                    let (lo, hi) = if from <= to { (from, to) } else { (to, from) };
                    self.selected = (lo..=hi).filter_map(|ix| self.row_path_at(ix)).collect();
                }
                _ => {
                    self.selected = [path].into_iter().collect();
                }
            }
        } else if click_count >= 2 {
            self.open_entry(&path, cx);
            cx.notify();
            return;
        } else if !(self.selected.len() > 1 && self.selected.contains(&path)) {
            self.selected = [path.clone()].into_iter().collect();
            self.focused_path = Some(path);
        }
        cx.notify();
    }

    pub fn mouse_up_select(
        &mut self,
        path: PathBuf,
        modifiers: Modifiers,
        click_count: usize,
        cx: &mut Context<Self>,
    ) {
        if modifiers.control || modifiers.shift || click_count >= 2 || self.box_select.is_some() {
            return;
        }
        if self.selected.len() > 1 && self.selected.contains(&path) {
            self.selected = [path.clone()].into_iter().collect();
            self.focused_path = Some(path);
            cx.notify();
        }
    }

    pub fn select_all(&mut self, cx: &mut Context<Self>) {
        self.selected = if self.is_trash() {
            self.trash_entries
                .iter()
                .map(|entry| entry.id_path.clone())
                .collect()
        } else {
            self.entries
                .iter()
                .map(|entry| entry.path.clone())
                .collect()
        };
        cx.notify();
    }

    pub fn move_selection(&mut self, delta: isize, cx: &mut Context<Self>) {
        let len = self.row_count();
        if len == 0 {
            return;
        }
        let len = len as isize;
        let current = self
            .focused_path
            .as_ref()
            .and_then(|path| self.row_index_of(path))
            .map(|ix| ix as isize);
        let next = match current {
            Some(ix) => (ix + delta).clamp(0, len - 1),
            None => 0,
        };
        let Some(path) = self.row_path_at(next as usize) else {
            return;
        };
        self.selected = [path.clone()].into_iter().collect();
        self.focused_path = Some(path);
        cx.notify();
    }
}
