use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

#[derive(Default, Clone, Copy)]
pub struct DirStats {
    pub file_count: u64,
    pub total_size: u64,
    pub size_on_disk: u64,
}

impl DirStats {
    pub fn merge(&mut self, other: DirStats) {
        self.file_count += other.file_count;
        self.total_size += other.total_size;
        self.size_on_disk += other.size_on_disk;
    }
}

pub fn scan_stats(path: &Path) -> DirStats {
    let mut stats = DirStats::default();
    scan_stats_into(path, &mut stats);
    stats
}

fn scan_stats_into(path: &Path, stats: &mut DirStats) {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return;
    };

    if metadata.is_dir() {
        let Ok(entries) = fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            scan_stats_into(&entry.path(), stats);
        }
    } else {
        stats.file_count += 1;
        stats.total_size += metadata.len();
        stats.size_on_disk += metadata.blocks() * 512;
    }
}
