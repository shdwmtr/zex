use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct TrashEntry {
    pub id_path: PathBuf,
    pub name: String,
    pub original_path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub deleted_at: i64,
    pub item: trash::TrashItem,
}

/// macOS has no public API for enumerating Trash contents with metadata
/// (`trash::os_limited` doesn't exist there); items still land in Trash
/// fine, they just can't be browsed, restored, or purged from within zex.
/// See `filesystem::operations::trash` for the matching operation-side gate.
#[cfg(target_os = "macos")]
pub fn list_sorted(_show_hidden: bool) -> Result<Vec<TrashEntry>, trash::Error> {
    Err(trash::Error::Unknown {
        description: "Browsing Trash isn't supported on macOS; use Finder's Trash instead."
            .to_string(),
    })
}

#[cfg(not(target_os = "macos"))]
pub fn list_sorted(show_hidden: bool) -> Result<Vec<TrashEntry>, trash::Error> {
    let items = trash::os_limited::list()?;

    let mut entries: Vec<TrashEntry> = items
        .into_iter()
        .filter(|item| show_hidden || !item.name.to_string_lossy().starts_with('.'))
        .map(|item| {
            let (is_dir, size) = match trash::os_limited::metadata(&item) {
                Ok(meta) => match meta.size {
                    trash::TrashItemSize::Bytes(bytes) => (false, bytes),
                    trash::TrashItemSize::Entries(_) => (true, 0),
                },
                Err(_) => (false, 0),
            };

            TrashEntry {
                id_path: PathBuf::from(&item.id),
                name: item.name.to_string_lossy().into_owned(),
                original_path: item.original_path(),
                is_dir,
                size,
                deleted_at: item.time_deleted,
                item,
            }
        })
        .collect();

    entries.sort_by_key(|entry| std::cmp::Reverse(entry.deleted_at));
    Ok(entries)
}
