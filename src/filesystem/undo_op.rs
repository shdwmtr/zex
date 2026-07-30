use std::path::PathBuf;

use super::operations::copy;
use super::operations::error::{OpError, OpResult};
use super::operations::mv;
use super::operations::new_entry::{self, NewEntryKind};
use super::operations::trash as trash_ops;

#[derive(Clone)]
pub enum UndoOp {
    Move {
        pairs: Vec<(PathBuf, PathBuf)>,
    },
    Copy {
        sources: Vec<PathBuf>,
        dest_dir: PathBuf,
        created: Vec<PathBuf>,
    },
    Trash {
        original_paths: Vec<PathBuf>,
        items: Vec<trash::TrashItem>,
    },
    Restore {
        original_paths: Vec<PathBuf>,
        items: Vec<trash::TrashItem>,
    },
    Rename {
        from: PathBuf,
        to: PathBuf,
    },
    NewEntry {
        path: PathBuf,
        kind: NewEntryKind,
    },
}

fn move_pairs(pairs: &[(PathBuf, PathBuf)], forward: bool) -> OpResult<()> {
    let mut errors = Vec::new();
    for (from, to) in pairs {
        let (source, dest) = if forward { (from, to) } else { (to, from) };
        if let Err(err) = mv::move_exact(source, dest) {
            errors.push(err);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub(crate) fn remove_path(path: &PathBuf) -> Result<(), OpError> {
    if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
    .map_err(|err| OpError::Io(err, path.clone()))
}

fn remove_paths(paths: &[PathBuf]) -> OpResult<()> {
    let mut errors = Vec::new();
    for path in paths {
        if let Err(err) = remove_path(path) {
            errors.push(err);
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

impl UndoOp {
    pub fn label(&self) -> &'static str {
        match self {
            UndoOp::Move { .. } => "Move",
            UndoOp::Copy { .. } => "Copy",
            UndoOp::Trash { .. } => "Delete",
            UndoOp::Restore { .. } => "Restore",
            UndoOp::Rename { .. } => "Rename",
            UndoOp::NewEntry { .. } => "Create",
        }
    }

    pub fn undo(&self) -> OpResult<UndoOp> {
        match self {
            UndoOp::Move { pairs } => {
                move_pairs(pairs, false)?;
                Ok(self.clone())
            }
            UndoOp::Rename { from, to } => {
                mv::move_exact(to, from).map_err(|err| vec![err])?;
                Ok(self.clone())
            }
            UndoOp::Copy { created, .. } => {
                remove_paths(created)?;
                Ok(self.clone())
            }
            UndoOp::NewEntry { path, kind } => {
                match kind {
                    NewEntryKind::Folder => std::fs::remove_dir(path)
                        .map_err(|err| vec![OpError::Io(err, path.clone())])?,
                    NewEntryKind::File => std::fs::remove_file(path)
                        .map_err(|err| vec![OpError::Io(err, path.clone())])?,
                }
                Ok(self.clone())
            }
            UndoOp::Trash {
                original_paths,
                items,
            } => {
                trash_ops::restore(items.clone())?;
                Ok(UndoOp::Trash {
                    original_paths: original_paths.clone(),
                    items: Vec::new(),
                })
            }
            UndoOp::Restore { original_paths, .. } => {
                let items = trash_ops::trash_capturing(original_paths)?;
                Ok(UndoOp::Restore {
                    original_paths: original_paths.clone(),
                    items,
                })
            }
        }
    }

    pub fn redo(&self) -> OpResult<UndoOp> {
        match self {
            UndoOp::Move { pairs } => {
                move_pairs(pairs, true)?;
                Ok(self.clone())
            }
            UndoOp::Rename { from, to } => {
                mv::move_exact(from, to).map_err(|err| vec![err])?;
                Ok(self.clone())
            }
            UndoOp::Copy {
                sources, dest_dir, ..
            } => {
                let (_, result) = copy::copy_into(sources, dest_dir);
                result?;
                Ok(self.clone())
            }
            UndoOp::NewEntry { path, kind } => {
                let parent = path.parent().unwrap_or_else(|| std::path::Path::new("/"));
                let name = path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default();
                match kind {
                    NewEntryKind::Folder => {
                        new_entry::new_folder(parent, name)?;
                    }
                    NewEntryKind::File => {
                        new_entry::new_file(parent, name)?;
                    }
                }
                Ok(self.clone())
            }
            UndoOp::Trash { original_paths, .. } => {
                let items = trash_ops::trash_capturing(original_paths)?;
                Ok(UndoOp::Trash {
                    original_paths: original_paths.clone(),
                    items,
                })
            }
            UndoOp::Restore {
                original_paths,
                items,
            } => {
                trash_ops::restore(items.clone())?;
                Ok(UndoOp::Restore {
                    original_paths: original_paths.clone(),
                    items: Vec::new(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(label: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("zex_undo_test_{label}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn move_undo_and_redo_round_trip() {
        let src_dir = temp_dir("move_src");
        let dest_dir = temp_dir("move_dest");
        let source = src_dir.join("file.txt");
        fs::write(&source, b"hi").unwrap();

        let dest = dest_dir.join("file.txt");
        let op = UndoOp::Move {
            pairs: vec![(source.clone(), dest.clone())],
        };
        mv::move_exact(&source, &dest).unwrap();
        assert!(dest.exists() && !source.exists());

        let redo_op = op.undo().unwrap();
        assert!(source.exists() && !dest.exists());

        redo_op.redo().unwrap();
        assert!(dest.exists() && !source.exists());

        fs::remove_dir_all(&src_dir).unwrap();
        fs::remove_dir_all(&dest_dir).unwrap();
    }

    #[test]
    fn copy_undo_removes_created_and_redo_recreates_it() {
        let src_dir = temp_dir("copy_src");
        let dest_dir = temp_dir("copy_dest");
        let source = src_dir.join("file.txt");
        fs::write(&source, b"hi").unwrap();

        let (created, result) = copy::copy_into(&[source.clone()], &dest_dir);
        result.unwrap();
        let op = UndoOp::Copy {
            sources: vec![source],
            dest_dir: dest_dir.clone(),
            created,
        };
        assert!(dest_dir.join("file.txt").exists());

        op.undo().unwrap();
        assert!(!dest_dir.join("file.txt").exists());

        op.redo().unwrap();
        assert!(dest_dir.join("file.txt").exists());

        fs::remove_dir_all(&src_dir).unwrap();
        fs::remove_dir_all(&dest_dir).unwrap();
    }

    #[test]
    fn rename_undo_and_redo_round_trip() {
        let dir = temp_dir("rename");
        let from = dir.join("a.txt");
        fs::write(&from, b"hi").unwrap();
        let to = mv::rename(&from, "b.txt").unwrap();

        let op = UndoOp::Rename {
            from: from.clone(),
            to: to.clone(),
        };

        op.undo().unwrap();
        assert!(from.exists() && !to.exists());

        op.redo().unwrap();
        assert!(to.exists() && !from.exists());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn new_entry_undo_deletes_and_redo_recreates() {
        let dir = temp_dir("new_entry");
        let path = new_entry::new_folder(&dir, "Sub").unwrap();

        let op = UndoOp::NewEntry {
            path: path.clone(),
            kind: NewEntryKind::Folder,
        };

        op.undo().unwrap();
        assert!(!path.exists());

        op.redo().unwrap();
        assert!(path.is_dir());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn new_entry_undo_fails_gracefully_if_folder_gained_content() {
        let dir = temp_dir("new_entry_populated");
        let path = new_entry::new_folder(&dir, "Sub").unwrap();
        fs::write(path.join("added.txt"), b"user data").unwrap();

        let op = UndoOp::NewEntry {
            path: path.clone(),
            kind: NewEntryKind::Folder,
        };

        assert!(op.undo().is_err());
        assert!(path.join("added.txt").exists());

        fs::remove_dir_all(&dir).unwrap();
    }
}
