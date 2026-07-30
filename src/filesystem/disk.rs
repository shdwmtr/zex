use std::path::{Path, PathBuf};

pub struct DiskInfo {
    pub mount_point: PathBuf,
    pub available_space: u64,
    pub total_space: u64,
}

pub fn resolve_disk_for(dir: &Path) -> Option<DiskInfo> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let best = disks
        .list()
        .iter()
        .filter(|disk| dir.starts_with(disk.mount_point()))
        .max_by_key(|disk| disk.mount_point().as_os_str().len())?;

    Some(DiskInfo {
        mount_point: best.mount_point().to_path_buf(),
        available_space: best.available_space(),
        total_space: best.total_space(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_disk_owning_temp_dir() {
        let dir = std::env::temp_dir();
        let disk = resolve_disk_for(&dir).expect("expected a disk to own the temp dir");
        assert!(dir.starts_with(&disk.mount_point));
    }

    #[test]
    fn returns_none_for_empty_path() {
        assert!(resolve_disk_for(Path::new("")).is_none());
    }
}
