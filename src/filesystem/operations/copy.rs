use std::fs;
use std::path::{Path, PathBuf};

use super::error::{OpError, OpResult};

pub fn copy_into(sources: &[PathBuf], dest_dir: &Path) -> (Vec<PathBuf>, OpResult<()>) {
    let mut created = Vec::new();
    let mut errors = Vec::new();

    for source in sources {
        match copy_one(source, dest_dir) {
            Ok(dest) => created.push(dest),
            Err(err) => errors.push(err),
        }
    }

    let result = if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    };
    (created, result)
}

pub(crate) fn copy_one(source: &Path, dest_dir: &Path) -> Result<PathBuf, OpError> {
    let Some(name) = source.file_name() else {
        return Err(OpError::NameConflict(source.to_path_buf()));
    };
    let dest = dest_dir.join(name);
    if dest.exists() {
        return Err(OpError::NameConflict(dest));
    }

    copy_one_exact(source, &dest)?;
    Ok(dest)
}

pub(super) fn copy_one_exact(source: &Path, dest: &Path) -> Result<(), OpError> {
    if source.is_dir() {
        copy_dir_recursive(source, dest)
    } else {
        fs::copy(source, dest)
            .map(|_| ())
            .map_err(|err| OpError::Io(err, source.to_path_buf()))
    }
}

fn copy_dir_recursive(source: &Path, dest: &Path) -> Result<(), OpError> {
    fs::create_dir(dest).map_err(|err| OpError::Io(err, source.to_path_buf()))?;

    let entries = fs::read_dir(source).map_err(|err| OpError::Io(err, source.to_path_buf()))?;
    for entry in entries {
        let entry = entry.map_err(|err| OpError::Io(err, source.to_path_buf()))?;
        let child_dest = dest.join(entry.file_name());
        let child_source = entry.path();

        if child_source.is_dir() {
            copy_dir_recursive(&child_source, &child_dest)?;
        } else {
            fs::copy(&child_source, &child_dest)
                .map(|_| ())
                .map_err(|err| OpError::Io(err, child_source))?;
        }
    }

    Ok(())
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
    fn copy_into_copies_files_and_directories_recursively() {
        let src_dir = temp_dir("copy_src");
        let dest_dir = temp_dir("copy_dest");

        fs::write(src_dir.join("file.txt"), b"hi").unwrap();
        fs::create_dir(src_dir.join("nested")).unwrap();
        fs::write(src_dir.join("nested").join("inner.txt"), b"world").unwrap();

        let sources = vec![src_dir.join("file.txt"), src_dir.join("nested")];
        copy_into(&sources, &dest_dir).1.unwrap();

        assert!(dest_dir.join("file.txt").is_file());
        assert!(dest_dir.join("nested").join("inner.txt").is_file());
        assert!(
            src_dir.join("file.txt").exists(),
            "source should remain after copy"
        );

        fs::remove_dir_all(&src_dir).unwrap();
        fs::remove_dir_all(&dest_dir).unwrap();
    }

    #[test]
    fn copy_into_reports_name_conflict_without_overwriting() {
        let src_dir = temp_dir("copy_conflict_src");
        let dest_dir = temp_dir("copy_conflict_dest");

        fs::write(src_dir.join("file.txt"), b"new").unwrap();
        fs::write(dest_dir.join("file.txt"), b"existing").unwrap();

        let (_, result) = copy_into(&[src_dir.join("file.txt")], &dest_dir);

        assert!(result.is_err());
        assert_eq!(fs::read(dest_dir.join("file.txt")).unwrap(), b"existing");

        fs::remove_dir_all(&src_dir).unwrap();
        fs::remove_dir_all(&dest_dir).unwrap();
    }
}
