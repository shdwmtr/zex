use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::copy::copy_one_exact;
use super::error::{OpError, OpResult};

pub fn rename(path: &Path, new_name: &str) -> OpResult<PathBuf> {
    let Some(parent) = path.parent() else {
        return Err(vec![OpError::NameConflict(path.to_path_buf())]);
    };
    let dest = parent.join(new_name);

    if dest.exists() {
        return Err(vec![OpError::NameConflict(dest)]);
    }

    fs::rename(path, &dest).map_err(|err| vec![OpError::Io(err, path.to_path_buf())])?;
    Ok(dest)
}

pub(crate) fn move_one(source: &Path, dest_dir: &Path) -> Result<PathBuf, OpError> {
    let Some(name) = source.file_name() else {
        return Err(OpError::NameConflict(source.to_path_buf()));
    };
    let dest = dest_dir.join(name);
    move_exact(source, &dest)?;
    Ok(dest)
}

pub fn move_exact(source: &Path, dest: &Path) -> Result<(), OpError> {
    if dest.exists() {
        return Err(OpError::NameConflict(dest.to_path_buf()));
    }

    match fs::rename(source, dest) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::CrossesDevices => {
            copy_one_exact(source, dest)?;
            if source.is_dir() {
                fs::remove_dir_all(source)
            } else {
                fs::remove_file(source)
            }
            .map_err(|err| OpError::Io(err, source.to_path_buf()))
        }
        Err(err) => Err(OpError::Io(err, source.to_path_buf())),
    }
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
    fn renames_a_file() {
        let dir = temp_dir("rename");
        let original = dir.join("a.txt");
        fs::write(&original, b"hi").unwrap();

        let new_path = rename(&original, "b.txt").unwrap();

        assert_eq!(new_path, dir.join("b.txt"));
        assert!(!original.exists());
        assert!(new_path.exists());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rename_fails_on_name_conflict() {
        let dir = temp_dir("rename_conflict");
        fs::write(dir.join("a.txt"), b"hi").unwrap();
        fs::write(dir.join("b.txt"), b"there").unwrap();

        let result = rename(&dir.join("a.txt"), "b.txt");

        assert!(
            matches!(result, Err(errors) if matches!(errors.as_slice(), [OpError::NameConflict(_)]))
        );

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn move_one_relocates_a_file_and_removes_source() {
        let src_dir = temp_dir("move_src");
        let dest_dir = temp_dir("move_dest");

        fs::write(src_dir.join("file.txt"), b"hi").unwrap();

        move_one(&src_dir.join("file.txt"), &dest_dir).unwrap();

        assert!(dest_dir.join("file.txt").is_file());
        assert!(!src_dir.join("file.txt").exists());

        fs::remove_dir_all(&src_dir).unwrap();
        fs::remove_dir_all(&dest_dir).unwrap();
    }
}
