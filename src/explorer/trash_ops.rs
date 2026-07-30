use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use gpui::{Context, SharedString};

use crate::filesystem::operations::trash as trash_ops;
use crate::filesystem::trash_entry;
use crate::filesystem::undo_op::UndoOp;

use super::bulk_op::{self, BulkItem};
use super::{Explorer, describe_bulk_errors, item_label};

impl Explorer {
    fn read_trash_filtered(
        &self,
    ) -> (
        Vec<crate::filesystem::trash_entry::TrashEntry>,
        Option<String>,
    ) {
        match trash_entry::list_sorted(self.show_hidden) {
            Ok(entries) => (entries, None),
            Err(err) => (Vec::new(), Some(format!("Couldn't read Trash: {err:?}"))),
        }
    }

    pub(super) fn load_trash(&mut self) {
        let (entries, error) = self.read_trash_filtered();
        self.trash_entries = entries;
        self.error = error;
        self.rebuild_trash_index();
        self.selected.clear();
        self.focused_path = None;
        self.free_space_label = String::new();
        self.entries.clear();
        self.entry_index.clear();
    }

    fn rebuild_trash_index(&mut self) {
        self.trash_entry_index.clear();
        self.trash_entry_index.extend(
            self.trash_entries
                .iter()
                .enumerate()
                .map(|(ix, entry)| (entry.id_path.clone(), ix)),
        );
    }

    pub(super) fn refresh_trash(&mut self) {
        let (entries, error) = self.read_trash_filtered();
        self.trash_entries = entries;
        self.error = error;
        self.rebuild_trash_index();

        let trash_entry_index = &self.trash_entry_index;
        self.selected
            .retain(|path| trash_entry_index.contains_key(path));
        if let Some(focused) = &self.focused_path
            && !trash_entry_index.contains_key(focused)
        {
            self.focused_path = None;
        }
    }

    pub fn delete_selection(&mut self, cx: &mut Context<Self>) {
        if self.is_trash() {
            self.purge_selection(cx);
            return;
        }
        if self.selected.is_empty() {
            return;
        }

        let message = match self.selected.len() {
            1 => {
                let name = self
                    .selected
                    .iter()
                    .next()
                    .and_then(|path| path.file_name())
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                format!("Are you sure you want to delete \"{name}\"?")
            }
            count => format!("Are you sure you want to delete these {count} items?"),
        };

        self.show_warning(
            "Delete",
            message,
            "Delete",
            |explorer, cx| explorer.delete_selection_confirmed(cx),
            cx,
        );
    }

    fn delete_selection_confirmed(&mut self, cx: &mut Context<Self>) {
        if self.selected.is_empty() {
            return;
        }
        let paths: Vec<PathBuf> = self.selected.iter().cloned().collect();
        let succeeded = Arc::new(Mutex::new(Vec::new()));
        let items = paths
            .iter()
            .map(|path| {
                let path = path.clone();
                let succeeded = succeeded.clone();
                BulkItem::new(item_label(&path), move || {
                    trash_ops::trash_one(&path)
                        .map(|()| succeeded.lock().unwrap().push(path.clone()))
                        .map_err(|err| err.to_string())
                })
            })
            .collect();

        self.selected.clear();
        bulk_op::spawn(
            self,
            cx,
            "Deleting",
            items,
            move |explorer, cx, errors, _cancelled| {
                let succeeded = Arc::try_unwrap(succeeded).unwrap().into_inner().unwrap();
                match trash_ops::capture_trashed_items(&succeeded) {
                    Ok(items) => {
                        if !items.is_empty() {
                            explorer.push_undo(
                                UndoOp::Trash {
                                    original_paths: succeeded,
                                    items,
                                },
                                cx,
                            );
                        }
                        explorer.op_error = describe_bulk_errors(&errors);
                    }
                    Err(err) => explorer.op_error = Some(err.to_string()),
                }
                explorer.refresh_entries();
            },
        );
    }

    pub fn restore_selection(&mut self, cx: &mut Context<Self>) {
        if !self.is_trash() || self.selected.is_empty() {
            return;
        }

        let entries: Vec<(PathBuf, trash::TrashItem)> = self
            .selected
            .iter()
            .filter_map(|path| self.trash_entry_index.get(path).copied())
            .map(|ix| {
                let entry = &self.trash_entries[ix];
                (entry.original_path.clone(), entry.item.clone())
            })
            .collect();

        let succeeded = Arc::new(Mutex::new(Vec::new()));
        let items = entries
            .iter()
            .map(|(original_path, item)| {
                let original_path = original_path.clone();
                let item = item.clone();
                let succeeded = succeeded.clone();
                BulkItem::new(item_label(&original_path), move || {
                    trash_ops::restore_one(item.clone())
                        .map(|()| succeeded.lock().unwrap().push(original_path.clone()))
                        .map_err(|err| err.to_string())
                })
            })
            .collect();

        self.selected.clear();
        bulk_op::spawn(
            self,
            cx,
            "Restoring",
            items,
            move |explorer, cx, errors, _cancelled| {
                let succeeded = Arc::try_unwrap(succeeded).unwrap().into_inner().unwrap();
                if !succeeded.is_empty() {
                    explorer.push_undo(
                        UndoOp::Restore {
                            original_paths: succeeded,
                            items: Vec::new(),
                        },
                        cx,
                    );
                }
                explorer.op_error = describe_bulk_errors(&errors);
                explorer.refresh_trash();
            },
        );
    }

    pub fn purge_selection(&mut self, cx: &mut Context<Self>) {
        if !self.is_trash() || self.selected.is_empty() {
            return;
        }

        let message = match self.selected.len() {
            1 => {
                let name = self
                    .selected
                    .iter()
                    .next()
                    .and_then(|path| self.trash_entry_index.get(path))
                    .map(|&ix| self.trash_entries[ix].name.clone())
                    .unwrap_or_default();
                format!("\"{name}\" will be permanently deleted. This action cannot be undone.")
            }
            count => {
                format!("{count} items will be permanently deleted. This action cannot be undone.")
            }
        };

        self.show_warning(
            "Delete Permanently",
            message,
            "Delete Permanently",
            |explorer, cx| explorer.purge_selection_confirmed(cx),
            cx,
        );
    }

    fn purge_selection_confirmed(&mut self, cx: &mut Context<Self>) {
        let entries: Vec<(SharedString, trash::TrashItem)> = self
            .selected
            .iter()
            .filter_map(|path| self.trash_entry_index.get(path).copied())
            .map(|ix| {
                let entry = &self.trash_entries[ix];
                (SharedString::from(entry.name.clone()), entry.item.clone())
            })
            .collect();

        self.selected.clear();
        self.spawn_purge(entries, cx);
    }

    pub fn empty_trash(&mut self, cx: &mut Context<Self>) {
        if !self.is_trash() || self.trash_entries.is_empty() {
            return;
        }

        let count = self.trash_entries.len();
        let message = format!(
            "All {count} items in the Trash will be permanently deleted. This action cannot be undone."
        );

        self.show_warning(
            "Empty Trash",
            message,
            "Empty Trash",
            |explorer, cx| explorer.empty_trash_confirmed(cx),
            cx,
        );
    }

    fn empty_trash_confirmed(&mut self, cx: &mut Context<Self>) {
        let entries: Vec<(SharedString, trash::TrashItem)> = self
            .trash_entries
            .iter()
            .map(|entry| (SharedString::from(entry.name.clone()), entry.item.clone()))
            .collect();

        self.selected.clear();
        self.spawn_purge(entries, cx);
    }

    fn spawn_purge(
        &mut self,
        entries: Vec<(SharedString, trash::TrashItem)>,
        cx: &mut Context<Self>,
    ) {
        let items = entries
            .into_iter()
            .map(|(name, item)| {
                BulkItem::new(name, move || {
                    trash_ops::purge_one(item.clone()).map_err(|err| err.to_string())
                })
            })
            .collect();

        bulk_op::spawn(
            self,
            cx,
            "Deleting Permanently",
            items,
            |explorer, _cx, errors, _cancelled| {
                explorer.op_error = describe_bulk_errors(&errors);
                explorer.refresh_trash();
            },
        );
    }

    pub(super) fn spawn_undo_trash(
        &mut self,
        original_paths: Vec<PathBuf>,
        items: Vec<trash::TrashItem>,
        cx: &mut Context<Self>,
    ) {
        let succeeded = Arc::new(Mutex::new(Vec::new()));
        let bulk_items = original_paths
            .into_iter()
            .zip(items)
            .map(|(original_path, item)| {
                let succeeded = succeeded.clone();
                BulkItem::new(item_label(&original_path), move || {
                    trash_ops::restore_one(item.clone())
                        .map(|()| succeeded.lock().unwrap().push(original_path.clone()))
                        .map_err(|err| err.to_string())
                })
            })
            .collect();

        bulk_op::spawn(
            self,
            cx,
            "Undoing Delete",
            bulk_items,
            move |explorer, _cx, errors, _cancelled| {
                let succeeded = Arc::try_unwrap(succeeded).unwrap().into_inner().unwrap();
                if !succeeded.is_empty() {
                    explorer.redo_stack.push(UndoOp::Trash {
                        original_paths: succeeded,
                        items: Vec::new(),
                    });
                }
                explorer.op_error = describe_bulk_errors(&errors);
                explorer.refresh_entries();
            },
        );
    }

    pub(super) fn spawn_redo_trash(
        &mut self,
        original_paths: Vec<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        let succeeded = Arc::new(Mutex::new(Vec::new()));
        let items = original_paths
            .iter()
            .map(|path| {
                let path = path.clone();
                let succeeded = succeeded.clone();
                BulkItem::new(item_label(&path), move || {
                    trash_ops::trash_one(&path)
                        .map(|()| succeeded.lock().unwrap().push(path.clone()))
                        .map_err(|err| err.to_string())
                })
            })
            .collect();

        bulk_op::spawn(
            self,
            cx,
            "Redoing Delete",
            items,
            move |explorer, _cx, errors, _cancelled| {
                let succeeded = Arc::try_unwrap(succeeded).unwrap().into_inner().unwrap();
                match trash_ops::capture_trashed_items(&succeeded) {
                    Ok(items) => {
                        if !items.is_empty() {
                            explorer.undo_stack.push(UndoOp::Trash {
                                original_paths: succeeded,
                                items,
                            });
                        }
                        explorer.op_error = describe_bulk_errors(&errors);
                    }
                    Err(err) => explorer.op_error = Some(err.to_string()),
                }
                explorer.refresh_entries();
            },
        );
    }

    pub(super) fn spawn_undo_restore(
        &mut self,
        original_paths: Vec<PathBuf>,
        cx: &mut Context<Self>,
    ) {
        let succeeded = Arc::new(Mutex::new(Vec::new()));
        let items = original_paths
            .iter()
            .map(|path| {
                let path = path.clone();
                let succeeded = succeeded.clone();
                BulkItem::new(item_label(&path), move || {
                    trash_ops::trash_one(&path)
                        .map(|()| succeeded.lock().unwrap().push(path.clone()))
                        .map_err(|err| err.to_string())
                })
            })
            .collect();

        bulk_op::spawn(
            self,
            cx,
            "Undoing Restore",
            items,
            move |explorer, _cx, errors, _cancelled| {
                let succeeded = Arc::try_unwrap(succeeded).unwrap().into_inner().unwrap();
                match trash_ops::capture_trashed_items(&succeeded) {
                    Ok(items) => {
                        if !items.is_empty() {
                            explorer.redo_stack.push(UndoOp::Restore {
                                original_paths: succeeded,
                                items,
                            });
                        }
                        explorer.op_error = describe_bulk_errors(&errors);
                    }
                    Err(err) => explorer.op_error = Some(err.to_string()),
                }
                explorer.refresh_entries();
            },
        );
    }

    pub(super) fn spawn_redo_restore(
        &mut self,
        original_paths: Vec<PathBuf>,
        items: Vec<trash::TrashItem>,
        cx: &mut Context<Self>,
    ) {
        let succeeded = Arc::new(Mutex::new(Vec::new()));
        let bulk_items = original_paths
            .into_iter()
            .zip(items)
            .map(|(original_path, item)| {
                let succeeded = succeeded.clone();
                BulkItem::new(item_label(&original_path), move || {
                    trash_ops::restore_one(item.clone())
                        .map(|()| succeeded.lock().unwrap().push(original_path.clone()))
                        .map_err(|err| err.to_string())
                })
            })
            .collect();

        bulk_op::spawn(
            self,
            cx,
            "Redoing Restore",
            bulk_items,
            move |explorer, _cx, errors, _cancelled| {
                let succeeded = Arc::try_unwrap(succeeded).unwrap().into_inner().unwrap();
                if !succeeded.is_empty() {
                    explorer.undo_stack.push(UndoOp::Restore {
                        original_paths: succeeded,
                        items: Vec::new(),
                    });
                }
                explorer.op_error = describe_bulk_errors(&errors);
                explorer.refresh_entries();
            },
        );
    }
}
