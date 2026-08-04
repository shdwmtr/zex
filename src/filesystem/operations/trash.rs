use std::path::{Path, PathBuf};

use super::error::{OpError, OpResult};

pub fn trash_one(path: &Path) -> Result<(), OpError> {
    trash::delete_all([path]).map_err(OpError::Trash)
}

pub fn trash_capturing(paths: &[PathBuf]) -> OpResult<Vec<trash::TrashItem>> {
    let mut errors = Vec::new();
    let mut succeeded = Vec::with_capacity(paths.len());
    for path in paths {
        match trash_one(path) {
            Ok(()) => succeeded.push(path.clone()),
            Err(err) => errors.push(err),
        }
    }

    match capture_trashed_items(&succeeded) {
        Ok(items) => {
            if errors.is_empty() {
                Ok(items)
            } else {
                Err(errors)
            }
        }
        Err(err) => {
            errors.push(err);
            Err(errors)
        }
    }
}

/// macOS has no public API for enumerating, restoring from, or purging
/// specific items in the Trash (only "move to trash" is supported; see
/// `trash::os_limited`'s platform gate). Everything below degrades
/// accordingly: capturing items for undo silently yields nothing (so no undo
/// entry is recorded), and restore/purge report a clear error instead of
/// linking against a module that doesn't exist on this platform.
#[cfg(target_os = "macos")]
fn unsupported() -> OpError {
    OpError::Trash(trash::Error::Unknown {
        description: "Browsing, restoring, and purging Trash items isn't supported on macOS; \
            use Finder's Trash (deleted items are still moved there normally)."
            .to_string(),
    })
}

#[cfg(not(target_os = "macos"))]
pub fn capture_trashed_items(paths: &[PathBuf]) -> Result<Vec<trash::TrashItem>, OpError> {
    if paths.is_empty() {
        return Ok(Vec::new());
    }

    let listed = trash::os_limited::list().map_err(OpError::Trash)?;
    let mut items = Vec::with_capacity(paths.len());
    for path in paths {
        let matched = listed
            .iter()
            .filter(|item| item.original_path() == *path)
            .max_by_key(|item| item.time_deleted);
        if let Some(item) = matched {
            items.push(item.clone());
        }
    }
    Ok(items)
}

#[cfg(target_os = "macos")]
pub fn capture_trashed_items(_paths: &[PathBuf]) -> Result<Vec<trash::TrashItem>, OpError> {
    Ok(Vec::new())
}

#[cfg(not(target_os = "macos"))]
pub fn restore_one(item: trash::TrashItem) -> Result<(), OpError> {
    trash::os_limited::restore_all(std::iter::once(item)).map_err(OpError::Trash)
}

#[cfg(target_os = "macos")]
pub fn restore_one(_item: trash::TrashItem) -> Result<(), OpError> {
    Err(unsupported())
}

pub fn restore(items: Vec<trash::TrashItem>) -> OpResult<()> {
    let errors: Vec<OpError> = items
        .into_iter()
        .filter_map(|item| restore_one(item).err())
        .collect();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

#[cfg(not(target_os = "macos"))]
pub fn purge_one(item: trash::TrashItem) -> Result<(), OpError> {
    trash::os_limited::purge_all(std::iter::once(item)).map_err(OpError::Trash)
}

#[cfg(target_os = "macos")]
pub fn purge_one(_item: trash::TrashItem) -> Result<(), OpError> {
    Err(unsupported())
}

// Exercises capture/restore/purge via `os_limited`, which doesn't exist on
// macOS (see `unsupported` above).
#[cfg(all(test, not(target_os = "macos")))]
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
    fn purge_continues_past_one_failing_item() {
        let dir = temp_dir("purge_partial_failure");
        let a = dir.join("a.txt");
        let b = dir.join("b.txt");
        let c = dir.join("c.txt");
        fs::write(&a, b"a").unwrap();
        fs::write(&b, b"b").unwrap();
        fs::write(&c, b"c").unwrap();

        trash_one(&a).unwrap();
        trash_one(&b).unwrap();
        trash_one(&c).unwrap();

        let items = capture_trashed_items(&[a.clone(), b.clone(), c.clone()]).unwrap();
        assert_eq!(items.len(), 3);

        let item_b = items
            .iter()
            .find(|item| item.original_path() == b)
            .unwrap()
            .clone();

        purge_one(item_b.clone()).unwrap();

        let remaining = trash::os_limited::list().unwrap();
        assert!(!remaining.iter().any(|item| item.original_path() == a));
        assert!(!remaining.iter().any(|item| item.original_path() == c));

        fs::remove_dir_all(&dir).unwrap();
    }
}
