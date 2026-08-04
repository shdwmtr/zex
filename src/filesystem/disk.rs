use std::ffi::CString;
use std::fs;
use std::mem::MaybeUninit;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

pub struct DiskInfo {
    pub mount_point: PathBuf,
    pub available_space: u64,
    pub total_space: u64,
}

fn unescape_mount_field(field: &str) -> String {
    field
        .replace("\\134", "\\")
        .replace("\\040", " ")
        .replace("\\011", "\t")
        .replace("\\012", "\n")
}

fn is_ignored_mount(fs_spec: &str, fs_file: &str, fs_vfstype: &str) -> bool {
    let filtered = matches!(
        fs_vfstype,
        "rootfs"
            | "sysfs"
            | "proc"
            | "devtmpfs"
            | "cgroup"
            | "cgroup2"
            | "pstore"
            | "squashfs"
            | "rpc_pipefs"
            | "iso9660"
            | "tmpfs"
            | "cifs"
            | "nfs"
            | "nfs4"
    );

    filtered
        || fs_file.starts_with("/sys")
        || fs_file.starts_with("/proc")
        || (fs_file.starts_with("/run") && !fs_file.starts_with("/run/media"))
        || fs_spec.starts_with("sunrpc")
}

fn mount_points() -> Vec<PathBuf> {
    let content = fs::read_to_string("/proc/mounts").unwrap_or_default();
    content
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let fs_spec = fields.next()?;
            let fs_file = unescape_mount_field(fields.next()?);
            let fs_vfstype = fields.next()?;
            if is_ignored_mount(fs_spec, &fs_file, fs_vfstype) {
                None
            } else {
                Some(PathBuf::from(fs_file))
            }
        })
        .collect()
}

fn statvfs_space(mount_point: &Path) -> Option<(u64, u64)> {
    let c_path = CString::new(mount_point.as_os_str().as_bytes()).ok()?;
    let mut stat = MaybeUninit::<libc::statvfs>::zeroed();
    let ret = loop {
        let ret = unsafe { libc::statvfs(c_path.as_ptr(), stat.as_mut_ptr()) };
        if ret == -1 && std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
            continue;
        }
        break ret;
    };
    if ret != 0 {
        return None;
    }

    let stat = unsafe { stat.assume_init() };
    let block_size = stat.f_bsize as u64;
    let total = block_size.saturating_mul(stat.f_blocks as u64);
    let available = block_size.saturating_mul(stat.f_bavail as u64);
    if total == 0 { None } else { Some((total, available)) }
}

pub fn resolve_disk_for(dir: &Path) -> Option<DiskInfo> {
    let mount_point = mount_points()
        .into_iter()
        .filter(|mount_point| dir.starts_with(mount_point))
        .max_by_key(|mount_point| mount_point.as_os_str().len())?;

    let (total_space, available_space) = statvfs_space(&mount_point)?;

    Some(DiskInfo { mount_point, available_space, total_space })
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
