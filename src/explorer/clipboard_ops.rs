use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use gpui::{ClipboardItem, ClipboardPaths, ClipboardPathsOp, Context};

use crate::filesystem::operations::{copy, mv};
use crate::filesystem::undo_op::UndoOp;

use super::bulk_op::{self, BulkItem};
use super::{Explorer, describe_bulk_errors, item_label};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ClipboardOp {
    Copy,
    Cut,
}

impl From<ClipboardOp> for ClipboardPathsOp {
    fn from(op: ClipboardOp) -> Self {
        match op {
            ClipboardOp::Copy => ClipboardPathsOp::Copy,
            ClipboardOp::Cut => ClipboardPathsOp::Cut,
        }
    }
}

impl From<ClipboardPathsOp> for ClipboardOp {
    fn from(op: ClipboardPathsOp) -> Self {
        match op {
            ClipboardPathsOp::Copy => ClipboardOp::Copy,
            ClipboardPathsOp::Cut => ClipboardOp::Cut,
        }
    }
}

#[derive(Clone, Debug)]
pub struct FileClipboard {
    pub op: ClipboardOp,
    pub paths: Vec<PathBuf>,
}

impl Explorer {
    pub fn copy_selection(&mut self, cx: &mut Context<Self>) {
        if self.is_trash() || self.selected.is_empty() {
            return;
        }
        let paths: Vec<PathBuf> = self.selected.iter().cloned().collect();
        cx.write_to_clipboard(ClipboardItem::new_paths(ClipboardPaths {
            paths: paths.clone(),
            op: ClipboardPathsOp::Copy,
        }));
        self.clipboard = Some(FileClipboard {
            op: ClipboardOp::Copy,
            paths,
        });
        cx.notify();
    }

    pub fn cut_selection(&mut self, cx: &mut Context<Self>) {
        if self.is_trash() || self.selected.is_empty() {
            return;
        }
        let paths: Vec<PathBuf> = self.selected.iter().cloned().collect();
        cx.write_to_clipboard(ClipboardItem::new_paths(ClipboardPaths {
            paths: paths.clone(),
            op: ClipboardPathsOp::Cut,
        }));
        self.clipboard = Some(FileClipboard {
            op: ClipboardOp::Cut,
            paths,
        });
        cx.notify();
    }

    pub fn copy_paths_to_clipboard(&self, paths: &[PathBuf], cx: &Context<Self>) {
        let text = paths
            .iter()
            .map(|path| path.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join("\n");
        cx.write_to_clipboard(ClipboardItem::new_string(text));
    }

    pub fn paste(&mut self, cx: &mut Context<Self>) {
        if self.is_trash() {
            return;
        }
        let os_clipboard = cx
            .read_from_clipboard()
            .and_then(|item| item.paths().cloned())
            .map(|ClipboardPaths { paths, op }| FileClipboard {
                op: op.into(),
                paths,
            });
        let Some(clipboard) = os_clipboard.or_else(|| self.clipboard.take()) else {
            return;
        };
        self.clipboard = None;
        let dest_dir = self.current_dir().to_path_buf();

        match clipboard.op {
            ClipboardOp::Copy => {
                let sources = clipboard.paths;
                let created = Arc::new(Mutex::new(Vec::new()));
                let items = sources
                    .iter()
                    .map(|source| {
                        let source = source.clone();
                        let dest_dir = dest_dir.clone();
                        let created = created.clone();
                        BulkItem::new(item_label(&source), move || {
                            copy::copy_one(&source, &dest_dir)
                                .map(|dest| created.lock().unwrap().push(dest))
                                .map_err(|err| err.to_string())
                        })
                    })
                    .collect();

                bulk_op::spawn(
                    self,
                    cx,
                    "Copying",
                    items,
                    move |explorer, cx, errors, _cancelled| {
                        let created = Arc::try_unwrap(created).unwrap().into_inner().unwrap();
                        if !created.is_empty() {
                            explorer.push_undo(
                                UndoOp::Copy {
                                    sources,
                                    dest_dir,
                                    created,
                                },
                                cx,
                            );
                        }
                        explorer.op_error = describe_bulk_errors(&errors);
                        explorer.refresh_entries();
                    },
                );
            }
            ClipboardOp::Cut => {
                let sources = clipboard.paths;
                let moved = Arc::new(Mutex::new(Vec::new()));
                let items = sources
                    .iter()
                    .map(|source| {
                        let source = source.clone();
                        let dest_dir = dest_dir.clone();
                        let moved = moved.clone();
                        BulkItem::new(item_label(&source), move || {
                            mv::move_one(&source, &dest_dir)
                                .map(|dest| moved.lock().unwrap().push((source.clone(), dest)))
                                .map_err(|err| err.to_string())
                        })
                    })
                    .collect();

                bulk_op::spawn(
                    self,
                    cx,
                    "Moving",
                    items,
                    move |explorer, cx, errors, _cancelled| {
                        let moved = Arc::try_unwrap(moved).unwrap().into_inner().unwrap();
                        if !moved.is_empty() {
                            explorer.push_undo(UndoOp::Move { pairs: moved }, cx);
                        }
                        explorer.op_error = describe_bulk_errors(&errors);
                        explorer.refresh_entries();
                    },
                );
            }
        };
    }

    pub fn move_paths_into(
        &mut self,
        paths: Vec<PathBuf>,
        dest_dir: PathBuf,
        cx: &mut Context<Self>,
    ) {
        if paths.is_empty() || paths.iter().any(|path| *path == dest_dir) {
            return;
        }

        let dest_name = dest_dir
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| dest_dir.to_string_lossy().into_owned());

        let message = match paths.len() {
            1 => {
                let name = paths[0]
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                format!("Are you sure you want to move \"{name}\" to \"{dest_name}\"?")
            }
            count => {
                format!("Are you sure you want to move these {count} items to \"{dest_name}\"?")
            }
        };

        self.show_warning(
            "Move",
            message,
            "Move",
            move |explorer, cx| {
                explorer.move_paths_into_confirmed(paths.clone(), dest_dir.clone(), cx);
            },
            cx,
        );
    }

    fn move_paths_into_confirmed(
        &mut self,
        paths: Vec<PathBuf>,
        dest_dir: PathBuf,
        cx: &mut Context<Self>,
    ) {
        let moved = Arc::new(Mutex::new(Vec::new()));
        let items = paths
            .iter()
            .map(|source| {
                let source = source.clone();
                let dest_dir = dest_dir.clone();
                let moved = moved.clone();
                BulkItem::new(item_label(&source), move || {
                    mv::move_one(&source, &dest_dir)
                        .map(|dest| moved.lock().unwrap().push((source.clone(), dest)))
                        .map_err(|err| err.to_string())
                })
            })
            .collect();

        bulk_op::spawn(
            self,
            cx,
            "Moving",
            items,
            move |explorer, cx, errors, _cancelled| {
                let moved = Arc::try_unwrap(moved).unwrap().into_inner().unwrap();
                if !moved.is_empty() {
                    explorer.push_undo(UndoOp::Move { pairs: moved }, cx);
                }
                explorer.op_error = describe_bulk_errors(&errors);
                explorer.refresh_entries();
            },
        );
    }
}
