use crate::filesystem::undo_op::UndoOp;

use super::clipboard_ops::FileClipboard;

const MAX_UNDO_HISTORY: usize = 100;

pub struct SharedState {
    pub clipboard: Option<FileClipboard>,
    pub undo_stack: Vec<UndoOp>,
    pub redo_stack: Vec<UndoOp>,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            clipboard: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn push_undo(&mut self, op: UndoOp) {
        self.undo_stack.push(op);
        if self.undo_stack.len() > MAX_UNDO_HISTORY {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }

    pub fn undo_label(&self) -> Option<&'static str> {
        self.undo_stack.last().map(UndoOp::label)
    }

    pub fn redo_label(&self) -> Option<&'static str> {
        self.redo_stack.last().map(UndoOp::label)
    }
}
