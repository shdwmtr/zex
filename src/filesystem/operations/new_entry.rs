use std::fs;
use std::path::{Path, PathBuf};

use super::error::{OpError, OpResult};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NewEntryKind {
    Folder,
    File,
}

pub fn new_folder(parent: &Path, name: &str) -> OpResult<PathBuf> {
    let dest = parent.join(name);
    if dest.exists() {
        return Err(vec![OpError::NameConflict(dest)]);
    }
    fs::create_dir(&dest).map_err(|err| vec![OpError::Io(err, dest.clone())])?;
    Ok(dest)
}

pub fn new_file(parent: &Path, name: &str) -> OpResult<PathBuf> {
    let dest = parent.join(name);
    if dest.exists() {
        return Err(vec![OpError::NameConflict(dest)]);
    }
    fs::File::create(&dest).map_err(|err| vec![OpError::Io(err, dest.clone())])?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("zex_ops_test_{label}_{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn new_folder_and_new_file_create_entries() {
        let dir = temp_dir("new_entries");

        let folder = new_folder(&dir, "Sub").unwrap();
        let file = new_file(&dir, "note.txt").unwrap();

        assert!(folder.is_dir());
        assert!(file.is_file());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn new_folder_fails_on_name_conflict() {
        let dir = temp_dir("new_folder_conflict");
        fs::create_dir(dir.join("Sub")).unwrap();

        let result = new_folder(&dir, "Sub");

        assert!(result.is_err());

        fs::remove_dir_all(&dir).unwrap();
    }
}
