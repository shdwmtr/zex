use gpui::Context;

use crate::filesystem::entry;

use super::Explorer;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Column {
    Type,
    Size,
    Modified,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SortColumn {
    Name,
    Type,
    Size,
    Modified,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct ColumnVisibility {
    pub type_: bool,
    pub size: bool,
    pub modified: bool,
}

impl Default for ColumnVisibility {
    fn default() -> Self {
        Self {
            type_: true,
            size: true,
            modified: true,
        }
    }
}

impl ColumnVisibility {
    pub fn get(&self, column: Column) -> bool {
        match column {
            Column::Type => self.type_,
            Column::Size => self.size,
            Column::Modified => self.modified,
        }
    }

    fn toggle(&mut self, column: Column) {
        let flag = match column {
            Column::Type => &mut self.type_,
            Column::Size => &mut self.size,
            Column::Modified => &mut self.modified,
        };
        *flag = !*flag;
    }
}

pub const MIN_COLUMN_WIDTH: f32 = 50.0;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct ColumnWidths {
    pub type_: f32,
    pub size: f32,
    pub modified: f32,
}

impl Default for ColumnWidths {
    fn default() -> Self {
        Self {
            type_: 110.0,
            size: 90.0,
            modified: 170.0,
        }
    }
}

impl ColumnWidths {
    pub fn get(&self, column: Column) -> f32 {
        match column {
            Column::Type => self.type_,
            Column::Size => self.size,
            Column::Modified => self.modified,
        }
    }

    fn set(&mut self, column: Column, value: f32) {
        let field = match column {
            Column::Type => &mut self.type_,
            Column::Size => &mut self.size,
            Column::Modified => &mut self.modified,
        };
        *field = value.max(MIN_COLUMN_WIDTH);
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ColumnResizeDrag {
    pub column: Column,
    pub anchor_x: f32,
    pub start_width: f32,
}

impl Explorer {
    pub fn toggle_column(&mut self, column: Column, cx: &mut Context<Self>) {
        self.column_visibility.toggle(column);
        cx.notify();
    }

    pub fn set_sort(&mut self, column: SortColumn, cx: &mut Context<Self>) {
        if self.sort_column == column {
            self.sort_direction = match self.sort_direction {
                SortDirection::Ascending => SortDirection::Descending,
                SortDirection::Descending => SortDirection::Ascending,
            };
        } else {
            self.sort_column = column;
            self.sort_direction = SortDirection::Ascending;
        }
        entry::sort_entries(&mut self.entries, self.sort_column, self.sort_direction);
        self.rebuild_entry_index();
        cx.notify();
    }

    pub fn begin_column_resize(&mut self, column: Column, anchor_x: f32, cx: &mut Context<Self>) {
        self.column_resize_drag = Some(ColumnResizeDrag {
            column,
            anchor_x,
            start_width: self.column_widths.get(column),
        });
        cx.notify();
    }

    pub fn update_column_resize(&mut self, current_x: f32, cx: &mut Context<Self>) {
        let Some(drag) = self.column_resize_drag else {
            return;
        };
        let new_width = drag.start_width - (current_x - drag.anchor_x);
        self.column_widths.set(drag.column, new_width);
        cx.notify();
    }

    pub fn end_column_resize(&mut self, cx: &mut Context<Self>) {
        if self.column_resize_drag.take().is_some() {
            cx.notify();
        }
    }
}
