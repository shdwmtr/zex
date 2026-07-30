use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use gpui::Context;

use crate::filesystem::operations::{copy, error, mv};
use crate::filesystem::undo_op::UndoOp;

use super::bulk_op::{self, BulkItem};
use super::{Explorer, describe_bulk_errors, item_label};

const MAX_UNDO_HISTORY: usize = 100;

impl Explorer {
    pub(super) fn push_undo(&mut self, op: UndoOp, cx: &mut Context<Self>) {
        self.undo_stack.push(op);
        if self.undo_stack.len() > MAX_UNDO_HISTORY {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
        cx.notify();
    }

    pub fn undo_label(&self) -> Option<&'static str> {
        self.undo_stack.last().map(UndoOp::label)
    }

    pub fn redo_label(&self) -> Option<&'static str> {
        self.redo_stack.last().map(UndoOp::label)
    }

    pub fn undo(&mut self, cx: &mut Context<Self>) {
        let Some(op) = self.undo_stack.pop() else {
            return;
        };
        match op {
            UndoOp::Rename { .. } | UndoOp::NewEntry { .. } => match op.undo() {
                Ok(redo_op) => {
                    self.op_error = None;
                    self.redo_stack.push(redo_op);
                    self.refresh_entries();
                }
                Err(errors) => self.op_error = Some(error::describe(&errors)),
            },
            UndoOp::Move { pairs } => self.spawn_undo_move(pairs, cx),
            UndoOp::Copy {
                sources,
                dest_dir,
                created,
            } => self.spawn_undo_copy(sources, dest_dir, created, cx),
            UndoOp::Trash {
                original_paths,
                items,
            } => self.spawn_undo_trash(original_paths, items, cx),
            UndoOp::Restore { original_paths, .. } => self.spawn_undo_restore(original_paths, cx),
        }
        cx.notify();
    }

    pub fn redo(&mut self, cx: &mut Context<Self>) {
        let Some(op) = self.redo_stack.pop() else {
            return;
        };
        match op {
            UndoOp::Rename { .. } | UndoOp::NewEntry { .. } => match op.redo() {
                Ok(undo_op) => {
                    self.op_error = None;
                    self.undo_stack.push(undo_op);
                    self.refresh_entries();
                }
                Err(errors) => self.op_error = Some(error::describe(&errors)),
            },
            UndoOp::Move { pairs } => self.spawn_redo_move(pairs, cx),
            UndoOp::Copy {
                sources, dest_dir, ..
            } => self.spawn_redo_copy(sources, dest_dir, cx),
            UndoOp::Trash { original_paths, .. } => self.spawn_redo_trash(original_paths, cx),
            UndoOp::Restore {
                original_paths,
                items,
            } => self.spawn_redo_restore(original_paths, items, cx),
        }
        cx.notify();
    }

    fn spawn_undo_move(&mut self, pairs: Vec<(PathBuf, PathBuf)>, cx: &mut Context<Self>) {
        let succeeded = Arc::new(Mutex::new(Vec::new()));
        let items = pairs
            .into_iter()
            .map(|(from, to)| {
                let succeeded = succeeded.clone();
                BulkItem::new(item_label(&to), move || {
                    mv::move_exact(&to, &from)
                        .map(|()| succeeded.lock().unwrap().push((from.clone(), to.clone())))
                        .map_err(|err| err.to_string())
                })
            })
            .collect();

        bulk_op::spawn(
            self,
            cx,
            "Undoing Move",
            items,
            move |explorer, _cx, errors, _cancelled| {
                let succeeded = Arc::try_unwrap(succeeded).unwrap().into_inner().unwrap();
                if !succeeded.is_empty() {
                    explorer.redo_stack.push(UndoOp::Move { pairs: succeeded });
                }
                explorer.op_error = describe_bulk_errors(&errors);
                explorer.refresh_entries();
            },
        );
    }

    fn spawn_redo_move(&mut self, pairs: Vec<(PathBuf, PathBuf)>, cx: &mut Context<Self>) {
        let succeeded = Arc::new(Mutex::new(Vec::new()));
        let items = pairs
            .into_iter()
            .map(|(from, to)| {
                let succeeded = succeeded.clone();
                BulkItem::new(item_label(&from), move || {
                    mv::move_exact(&from, &to)
                        .map(|()| succeeded.lock().unwrap().push((from.clone(), to.clone())))
                        .map_err(|err| err.to_string())
                })
            })
            .collect();

        bulk_op::spawn(
            self,
            cx,
            "Redoing Move",
            items,
            move |explorer, _cx, errors, _cancelled| {
                let succeeded = Arc::try_unwrap(succeeded).unwrap().into_inner().unwrap();
                if !succeeded.is_empty() {
                    explorer.undo_stack.push(UndoOp::Move { pairs: succeeded });
                }
                explorer.op_error = describe_bulk_errors(&errors);
                explorer.refresh_entries();
            },
        );
    }

    fn spawn_undo_copy(
        &mut self,
        sources: Vec<PathBuf>,
        dest_dir: PathBuf,
        created: Vec<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        let succeeded = Arc::new(Mutex::new(Vec::new()));
        let items = sources
            .into_iter()
            .zip(created)
            .map(|(source, created_path)| {
                let succeeded = succeeded.clone();
                let target = created_path.clone();
                BulkItem::new(item_label(&created_path), move || {
                    crate::filesystem::undo_op::remove_path(&target)
                        .map(|()| {
                            succeeded
                                .lock()
                                .unwrap()
                                .push((source.clone(), created_path.clone()))
                        })
                        .map_err(|err| err.to_string())
                })
            })
            .collect();

        bulk_op::spawn(
            self,
            cx,
            "Undoing Copy",
            items,
            move |explorer, _cx, errors, _cancelled| {
                let succeeded = Arc::try_unwrap(succeeded).unwrap().into_inner().unwrap();
                if !succeeded.is_empty() {
                    let (sources, created) = succeeded.into_iter().unzip();
                    explorer.redo_stack.push(UndoOp::Copy {
                        sources,
                        dest_dir,
                        created,
                    });
                }
                explorer.op_error = describe_bulk_errors(&errors);
                explorer.refresh_entries();
            },
        );
    }

    fn spawn_redo_copy(
        &mut self,
        sources: Vec<PathBuf>,
        dest_dir: PathBuf,
        cx: &mut Context<Self>,
    ) {
        let succeeded = Arc::new(Mutex::new(Vec::new()));
        let items = sources
            .into_iter()
            .map(|source| {
                let dest_dir = dest_dir.clone();
                let succeeded = succeeded.clone();
                BulkItem::new(item_label(&source), move || {
                    copy::copy_one(&source, &dest_dir)
                        .map(|dest| succeeded.lock().unwrap().push((source.clone(), dest)))
                        .map_err(|err| err.to_string())
                })
            })
            .collect();

        bulk_op::spawn(
            self,
            cx,
            "Redoing Copy",
            items,
            move |explorer, _cx, errors, _cancelled| {
                let succeeded = Arc::try_unwrap(succeeded).unwrap().into_inner().unwrap();
                if !succeeded.is_empty() {
                    let (sources, created) = succeeded.into_iter().unzip();
                    explorer.undo_stack.push(UndoOp::Copy {
                        sources,
                        dest_dir,
                        created,
                    });
                }
                explorer.op_error = describe_bulk_errors(&errors);
                explorer.refresh_entries();
            },
        );
    }
}
