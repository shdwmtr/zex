use std::path::{Path, PathBuf};
use std::time::SystemTime;

use rayon::prelude::*;

#[derive(Clone, Debug)]
pub struct FsEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

pub fn read_dir_sorted(dir: &Path, show_hidden: bool) -> std::io::Result<Vec<FsEntry>> {
    let dir_entries: Vec<std::fs::DirEntry> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter(|entry| show_hidden || !entry.file_name().to_string_lossy().starts_with('.'))
        .collect();

    let mut entries: Vec<FsEntry> = dir_entries
        .into_par_iter()
        .filter_map(|entry| {
            let metadata = entry.metadata().ok()?;
            Some(FsEntry {
                name: entry.file_name().to_string_lossy().into_owned(),
                path: entry.path(),
                is_dir: metadata.is_dir(),
                size: metadata.len(),
                modified: metadata.modified().ok(),
            })
        })
        .collect();

    entries.sort_by_cached_key(|entry| (!entry.is_dir, entry.name.to_lowercase()));

    Ok(entries)
}

pub fn sort_entries(
    entries: &mut [FsEntry],
    column: crate::explorer::columns::SortColumn,
    direction: crate::explorer::columns::SortDirection,
) {
    use crate::explorer::columns::{SortColumn, SortDirection};

    entries.sort_by(|a, b| {
        let dir_order = b.is_dir.cmp(&a.is_dir);
        if dir_order != std::cmp::Ordering::Equal {
            return dir_order;
        }

        let name_order = || a.name.to_lowercase().cmp(&b.name.to_lowercase());
        let ordering = match column {
            SortColumn::Name => name_order(),
            SortColumn::Type => type_label(a).cmp(&type_label(b)).then_with(name_order),
            SortColumn::Size => a.size.cmp(&b.size).then_with(name_order),
            SortColumn::Modified => a.modified.cmp(&b.modified).then_with(name_order),
        };

        match direction {
            SortDirection::Ascending => ordering,
            SortDirection::Descending => ordering.reverse(),
        }
    });
}

pub fn type_label(entry: &FsEntry) -> String {
    type_label_for(&entry.name, entry.is_dir)
}

pub fn type_label_for(name: &str, is_dir: bool) -> String {
    if is_dir {
        return "Folder".to_string();
    }

    match Path::new(name).extension().and_then(|ext| ext.to_str()) {
        Some(ext) => format!("{} File", ext.to_uppercase()),
        None => "File".to_string(),
    }
}

pub fn permission_string(mode: u32) -> String {
    const BITS: [(u32, char); 9] = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];
    BITS.iter()
        .map(|&(mask, ch)| if mode & mask != 0 { ch } else { '-' })
        .collect()
}

pub fn octal_permissions(mode: u32) -> String {
    format!("{:o}", mode & 0o7777)
}

pub fn format_modified(modified: Option<SystemTime>) -> String {
    match modified {
        Some(time) => chrono::DateTime::<chrono::Local>::from(time)
            .format("%b %-d, %Y %H:%M")
            .to_string(),
        None => String::new(),
    }
}

pub fn format_size(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = size as f64;
    let mut unit_index = 0;

    while value >= 1024.0 && unit_index < UNITS.len() - 1 {
        value /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{size} {}", UNITS[unit_index])
    } else {
        format!("{value:.1} {}", UNITS[unit_index])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn dirs_sort_before_files_case_insensitive() {
        let tmp = std::env::temp_dir().join(format!("zex_test_{}", std::process::id()));
        fs::create_dir_all(&tmp).unwrap();

        fs::create_dir(tmp.join("zeta_dir")).unwrap();
        fs::write(tmp.join("Alpha_file.txt"), b"hi").unwrap();
        fs::write(tmp.join("beta_file.txt"), b"hi").unwrap();
        fs::create_dir(tmp.join("Beta_dir")).unwrap();

        let entries = read_dir_sorted(&tmp, true).unwrap();
        let names: Vec<&str> = entries.iter().map(|e| e.name.as_str()).collect();

        assert_eq!(
            names,
            vec!["Beta_dir", "zeta_dir", "Alpha_file.txt", "beta_file.txt"]
        );

        fs::remove_dir_all(&tmp).unwrap();
    }
}
