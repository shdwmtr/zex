use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::SystemTime;

use rayon::prelude::*;
use rustc_hash::FxHashMap;

pub type NodeId = usize;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NodeKind {
    File,
    Directory,
    MountBoundary,
    Inaccessible,
}

#[derive(Clone, Debug)]
pub struct DiskUsageNode {
    pub name: String,
    pub path: PathBuf,
    pub kind: NodeKind,
    pub size: u64,
    pub item_count: u64,
    pub modified: Option<SystemTime>,
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
}

#[derive(Default, Debug)]
pub struct DiskUsageTree {
    pub nodes: Vec<DiskUsageNode>,
    pub path_index: FxHashMap<PathBuf, NodeId>,
}

impl DiskUsageTree {
    pub fn root(&self) -> NodeId {
        0
    }

    pub fn get(&self, id: NodeId) -> &DiskUsageNode {
        &self.nodes[id]
    }

    pub fn find(&self, path: &Path) -> Option<NodeId> {
        self.path_index.get(path).copied()
    }
}

struct ScannedNode {
    name: String,
    path: PathBuf,
    kind: NodeKind,
    size: u64,
    item_count: u64,
    modified: Option<SystemTime>,
    children: Vec<ScannedNode>,
}

impl ScannedNode {
    fn leaf(name: String, path: PathBuf, kind: NodeKind, size: u64, modified: Option<SystemTime>) -> Self {
        Self {
            name,
            path,
            kind,
            size,
            item_count: 0,
            modified,
            children: Vec::new(),
        }
    }
}

fn is_mount_boundary(node_dev: u64, root_dev: u64, cross_filesystem_boundaries: bool) -> bool {
    !cross_filesystem_boundaries && node_dev != root_dev
}

pub fn scan_tree(
    root: &Path,
    cross_filesystem_boundaries: bool,
    follow_symlinks: bool,
    cancel: &AtomicBool,
    files_scanned: &AtomicU64,
    bytes_scanned: &AtomicU64,
) -> DiskUsageTree {
    let root_dev = fs::symlink_metadata(root).map(|meta| meta.dev()).unwrap_or(0);
    let scanned = scan_node(
        root,
        root_dev,
        cross_filesystem_boundaries,
        follow_symlinks,
        cancel,
        files_scanned,
        bytes_scanned,
    );
    flatten_tree(scanned)
}

fn scan_node(
    path: &Path,
    root_dev: u64,
    cross_filesystem_boundaries: bool,
    follow_symlinks: bool,
    cancel: &AtomicBool,
    files_scanned: &AtomicU64,
    bytes_scanned: &AtomicU64,
) -> ScannedNode {
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.to_string_lossy().into_owned());

    if cancel.load(Ordering::SeqCst) {
        return ScannedNode::leaf(name, path.to_path_buf(), NodeKind::Inaccessible, 0, None);
    }

    let Ok(meta) = fs::symlink_metadata(path) else {
        return ScannedNode::leaf(name, path.to_path_buf(), NodeKind::Inaccessible, 0, None);
    };

    if meta.is_symlink() {
        if follow_symlinks {
            let Ok(target_meta) = fs::metadata(path) else {
                return ScannedNode::leaf(
                    name,
                    path.to_path_buf(),
                    NodeKind::Inaccessible,
                    0,
                    meta.modified().ok(),
                );
            };
            if target_meta.is_dir() {
                return scan_directory(
                    path,
                    &target_meta,
                    root_dev,
                    cross_filesystem_boundaries,
                    follow_symlinks,
                    cancel,
                    files_scanned,
                    bytes_scanned,
                    name,
                );
            }
            files_scanned.fetch_add(1, Ordering::Relaxed);
            bytes_scanned.fetch_add(target_meta.len(), Ordering::Relaxed);
            return ScannedNode::leaf(
                name,
                path.to_path_buf(),
                NodeKind::File,
                target_meta.len(),
                target_meta.modified().ok(),
            );
        }

        files_scanned.fetch_add(1, Ordering::Relaxed);
        bytes_scanned.fetch_add(meta.len(), Ordering::Relaxed);
        return ScannedNode::leaf(
            name,
            path.to_path_buf(),
            NodeKind::File,
            meta.len(),
            meta.modified().ok(),
        );
    }

    if meta.is_dir() {
        return scan_directory(
            path,
            &meta,
            root_dev,
            cross_filesystem_boundaries,
            follow_symlinks,
            cancel,
            files_scanned,
            bytes_scanned,
            name,
        );
    }

    files_scanned.fetch_add(1, Ordering::Relaxed);
    bytes_scanned.fetch_add(meta.len(), Ordering::Relaxed);
    ScannedNode::leaf(name, path.to_path_buf(), NodeKind::File, meta.len(), meta.modified().ok())
}

#[allow(clippy::too_many_arguments)]
fn scan_directory(
    path: &Path,
    meta: &fs::Metadata,
    root_dev: u64,
    cross_filesystem_boundaries: bool,
    follow_symlinks: bool,
    cancel: &AtomicBool,
    files_scanned: &AtomicU64,
    bytes_scanned: &AtomicU64,
    name: String,
) -> ScannedNode {
    if is_mount_boundary(meta.dev(), root_dev, cross_filesystem_boundaries) {
        return ScannedNode::leaf(
            name,
            path.to_path_buf(),
            NodeKind::MountBoundary,
            0,
            meta.modified().ok(),
        );
    }

    let Ok(entries) = fs::read_dir(path) else {
        return ScannedNode::leaf(
            name,
            path.to_path_buf(),
            NodeKind::Inaccessible,
            0,
            meta.modified().ok(),
        );
    };

    let child_paths: Vec<PathBuf> = entries.filter_map(|entry| entry.ok()).map(|entry| entry.path()).collect();

    let mut children: Vec<ScannedNode> = child_paths
        .into_par_iter()
        .map(|child_path| {
            scan_node(
                &child_path,
                root_dev,
                cross_filesystem_boundaries,
                follow_symlinks,
                cancel,
                files_scanned,
                bytes_scanned,
            )
        })
        .collect();

    children.sort_by_key(|child| std::cmp::Reverse(child.size));

    let size = children.iter().map(|child| child.size).sum();
    let item_count = children.iter().map(|child| child.item_count + 1).sum();

    ScannedNode {
        name,
        path: path.to_path_buf(),
        kind: NodeKind::Directory,
        size,
        item_count,
        modified: meta.modified().ok(),
        children,
    }
}

fn flatten_tree(root: ScannedNode) -> DiskUsageTree {
    let mut nodes = Vec::new();
    let mut path_index = FxHashMap::default();
    flatten_node(root, None, &mut nodes, &mut path_index);
    DiskUsageTree { nodes, path_index }
}

fn flatten_node(
    scanned: ScannedNode,
    parent: Option<NodeId>,
    nodes: &mut Vec<DiskUsageNode>,
    path_index: &mut FxHashMap<PathBuf, NodeId>,
) -> NodeId {
    let id = nodes.len();
    nodes.push(DiskUsageNode {
        name: scanned.name,
        path: scanned.path.clone(),
        kind: scanned.kind,
        size: scanned.size,
        item_count: scanned.item_count,
        modified: scanned.modified,
        parent,
        children: Vec::new(),
    });
    path_index.insert(scanned.path, id);

    let child_ids: Vec<NodeId> = scanned
        .children
        .into_iter()
        .map(|child| flatten_node(child, Some(id), nodes, path_index))
        .collect();
    nodes[id].children = child_ids;

    id
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_tmp(label: &str) -> PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("zex_disk_usage_test_{}_{label}_{n}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn scan(root: &Path, cross_fs: bool, follow_symlinks: bool) -> DiskUsageTree {
        let cancel = AtomicBool::new(false);
        let files = AtomicU64::new(0);
        let bytes = AtomicU64::new(0);
        scan_tree(root, cross_fs, follow_symlinks, &cancel, &files, &bytes)
    }

    #[test]
    fn rolls_up_sizes_and_sorts_children_by_size_descending() {
        let tmp = make_tmp("rollup");
        fs::create_dir_all(tmp.join("small")).unwrap();
        fs::write(tmp.join("small/a.txt"), vec![0u8; 10]).unwrap();
        fs::create_dir_all(tmp.join("big")).unwrap();
        fs::write(tmp.join("big/b.txt"), vec![0u8; 1000]).unwrap();

        let tree = scan(&tmp, false, false);
        let root = tree.get(tree.root());

        assert_eq!(root.children.len(), 2);
        let first = tree.get(root.children[0]);
        let second = tree.get(root.children[1]);
        assert_eq!(first.name, "big");
        assert_eq!(second.name, "small");
        assert_eq!(root.size, first.size + second.size);
        assert_eq!(root.item_count, first.item_count + second.item_count + 2);

        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn unreadable_directory_becomes_inaccessible_without_panicking() {
        use std::os::unix::fs::PermissionsExt;

        let tmp = make_tmp("perm");
        let locked = tmp.join("locked");
        fs::create_dir_all(&locked).unwrap();
        fs::write(locked.join("secret.txt"), b"hi").unwrap();
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();

        let can_still_read = fs::read_dir(&locked).is_ok();

        let tree = scan(&tmp, false, false);

        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();

        if !can_still_read {
            let root = tree.get(tree.root());
            let locked_node = root
                .children
                .iter()
                .map(|&id| tree.get(id))
                .find(|node| node.name == "locked")
                .unwrap();
            assert_eq!(locked_node.kind, NodeKind::Inaccessible);
        }

        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn symlink_not_followed_by_default_uses_link_size() {
        let tmp = make_tmp("symlink");
        fs::write(tmp.join("target.txt"), vec![0u8; 500]).unwrap();
        let link = tmp.join("link.txt");
        std::os::unix::fs::symlink(tmp.join("target.txt"), &link).unwrap();

        let tree = scan(&tmp, false, false);
        let root = tree.get(tree.root());
        let link_node = root
            .children
            .iter()
            .map(|&id| tree.get(id))
            .find(|node| node.name == "link.txt")
            .unwrap();

        assert_eq!(link_node.kind, NodeKind::File);
        assert!(link_node.size < 500);

        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn is_mount_boundary_respects_cross_filesystem_flag() {
        assert!(!is_mount_boundary(1, 1, false));
        assert!(is_mount_boundary(2, 1, false));
        assert!(!is_mount_boundary(2, 1, true));
    }

    #[test]
    fn tree_find_and_parent_links_are_consistent() {
        let tmp = make_tmp("links");
        fs::create_dir_all(tmp.join("sub")).unwrap();
        fs::write(tmp.join("sub/file.txt"), b"hi").unwrap();

        let tree = scan(&tmp, false, false);

        let sub_id = tree.find(&tmp.join("sub")).unwrap();
        let sub = tree.get(sub_id);
        assert_eq!(sub.parent, Some(tree.root()));

        let file_id = tree.find(&tmp.join("sub/file.txt")).unwrap();
        assert_eq!(tree.get(file_id).parent, Some(sub_id));

        fs::remove_dir_all(&tmp).unwrap();
    }

    #[test]
    fn cancellation_returns_near_immediately() {
        let tmp = make_tmp("cancel");
        for i in 0..20 {
            let dir = tmp.join(format!("dir{i}"));
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("file.txt"), vec![0u8; 100]).unwrap();
        }

        let cancel = AtomicBool::new(true);
        let files = AtomicU64::new(0);
        let bytes = AtomicU64::new(0);
        let tree = scan_tree(&tmp, false, false, &cancel, &files, &bytes);

        assert!(tree.get(tree.root()).children.is_empty());

        fs::remove_dir_all(&tmp).unwrap();
    }
}
