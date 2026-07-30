use std::f32::consts::PI;

use crate::filesystem::disk_usage::{DiskUsageTree, NodeId};

pub const RING_DEPTH: u32 = 3;
pub const CENTER_HOLE_FRAC: f32 = 0.22;
const RING_THICKNESS_FRAC: f32 = (1.0 - CENTER_HOLE_FRAC) / RING_DEPTH as f32;

/// Wedges thinner than this (radians) are folded into a single aggregate
/// wedge instead of being individually laid out and painted. Keeps both the
/// wedge count and the tessellation cost bounded regardless of how many
/// children a directory has.
pub const MIN_WEDGE_ANGLE: f32 = 0.025;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WedgeTarget {
    Node(NodeId),
    Aggregate { size: u64, count: usize },
}

#[derive(Clone, Debug, PartialEq)]
pub struct WedgeGeometry {
    pub target: WedgeTarget,
    pub depth: u32,
    pub branch_index: usize,
    pub start_angle: f32,
    pub end_angle: f32,
    pub inner_radius_frac: f32,
    pub outer_radius_frac: f32,
    pub parent_total: u64,
}

pub fn layout_wedges(tree: &DiskUsageTree, root: NodeId, max_depth: u32) -> Vec<WedgeGeometry> {
    let mut out = Vec::new();
    layout_children(tree, root, None, 0.0, 2.0 * PI, 1, max_depth, &mut out);
    out
}

#[allow(clippy::too_many_arguments)]
fn layout_children(
    tree: &DiskUsageTree,
    node_id: NodeId,
    branch_index: Option<usize>,
    start_angle: f32,
    end_angle: f32,
    depth: u32,
    max_depth: u32,
    out: &mut Vec<WedgeGeometry>,
) {
    if depth > max_depth {
        return;
    }

    let node = tree.get(node_id);
    // `node.children` is already sorted descending by size (the scanner sorts
    // it that way), so once a child's angular span drops below the visible
    // threshold every remaining child is at least as small and can be
    // folded into the aggregate without inspecting it individually.
    let children: Vec<NodeId> = node
        .children
        .iter()
        .copied()
        .filter(|&id| tree.get(id).size > 0)
        .collect();
    let total: u64 = children.iter().map(|&id| tree.get(id).size).sum();
    if total == 0 {
        return;
    }

    let inner = (depth - 1) as f32 * RING_THICKNESS_FRAC + CENTER_HOLE_FRAC;
    let outer = inner + RING_THICKNESS_FRAC;
    let full_span = end_angle - start_angle;

    let mut angle_cursor = start_angle;
    let mut included_size: u64 = 0;
    let mut included_count = 0usize;

    for (ix, &child_id) in children.iter().enumerate() {
        let child = tree.get(child_id);
        let span = full_span * (child.size as f32 / total as f32);
        if span < MIN_WEDGE_ANGLE {
            break;
        }

        let child_start = angle_cursor;
        let child_end = angle_cursor + span;
        let this_branch = branch_index.unwrap_or(ix);

        out.push(WedgeGeometry {
            target: WedgeTarget::Node(child_id),
            depth,
            branch_index: this_branch,
            start_angle: child_start,
            end_angle: child_end,
            inner_radius_frac: inner,
            outer_radius_frac: outer,
            parent_total: total,
        });

        layout_children(tree, child_id, Some(this_branch), child_start, child_end, depth + 1, max_depth, out);

        angle_cursor = child_end;
        included_size += child.size;
        included_count += 1;
    }

    let aggregate_count = children.len() - included_count;
    if aggregate_count > 0 {
        out.push(WedgeGeometry {
            target: WedgeTarget::Aggregate { size: total - included_size, count: aggregate_count },
            depth,
            branch_index: branch_index.unwrap_or(included_count),
            start_angle: angle_cursor,
            end_angle,
            inner_radius_frac: inner,
            outer_radius_frac: outer,
            parent_total: total,
        });
    }
}

pub fn polar(offset: (f32, f32), max_radius: f32) -> (f32, f32) {
    let (dx, dy) = offset;
    let radius = (dx * dx + dy * dy).sqrt();
    let mut angle = dy.atan2(dx) + PI / 2.0;
    if angle < 0.0 {
        angle += 2.0 * PI;
    }
    let radius_frac = if max_radius > 0.0 { radius / max_radius } else { 0.0 };
    (angle, radius_frac)
}

fn angle_in_range(angle: f32, start: f32, end: f32) -> bool {
    if end - start >= 2.0 * PI - 1e-4 {
        return true;
    }
    let normalized = angle.rem_euclid(2.0 * PI);
    let start = start.rem_euclid(2.0 * PI);
    let end = end.rem_euclid(2.0 * PI);
    if start <= end {
        normalized >= start && normalized < end
    } else {
        normalized >= start || normalized < end
    }
}

pub fn hit_test(wedges: &[WedgeGeometry], angle: f32, radius_frac: f32) -> Option<WedgeTarget> {
    wedges
        .iter()
        .find(|wedge| {
            radius_frac >= wedge.inner_radius_frac
                && radius_frac < wedge.outer_radius_frac
                && angle_in_range(angle, wedge.start_angle, wedge.end_angle)
        })
        .map(|wedge| wedge.target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_hash::FxHashMap;
    use std::path::PathBuf;

    use crate::filesystem::disk_usage::{DiskUsageNode, NodeKind};

    fn node(name: &str, size: u64, parent: Option<NodeId>, children: Vec<NodeId>) -> DiskUsageNode {
        DiskUsageNode {
            name: name.to_string(),
            path: PathBuf::from(name),
            kind: NodeKind::Directory,
            size,
            item_count: 0,
            modified: None,
            parent,
            children,
        }
    }

    fn flat_tree() -> DiskUsageTree {
        let nodes = vec![
            node("root", 400, None, vec![1, 2]),
            node("big", 300, Some(0), vec![]),
            node("small", 100, Some(0), vec![]),
        ];
        let path_index = nodes
            .iter()
            .enumerate()
            .map(|(ix, n)| (n.path.clone(), ix))
            .collect::<FxHashMap<_, _>>();
        DiskUsageTree { nodes, path_index }
    }

    fn nested_tree() -> DiskUsageTree {
        let nodes = vec![
            node("root", 200, None, vec![1]),
            node("mid", 200, Some(0), vec![2]),
            node("leaf", 200, Some(1), vec![]),
        ];
        let path_index = nodes
            .iter()
            .enumerate()
            .map(|(ix, n)| (n.path.clone(), ix))
            .collect::<FxHashMap<_, _>>();
        DiskUsageTree { nodes, path_index }
    }

    #[test]
    fn splits_angles_proportionally_to_size() {
        let tree = flat_tree();
        let wedges = layout_wedges(&tree, 0, 1);

        assert_eq!(wedges.len(), 2);
        let big = wedges.iter().find(|w| w.target == WedgeTarget::Node(1)).unwrap();
        let small = wedges.iter().find(|w| w.target == WedgeTarget::Node(2)).unwrap();

        assert!((big.end_angle - big.start_angle - 0.75 * 2.0 * PI).abs() < 1e-4);
        assert!((small.end_angle - small.start_angle - 0.25 * 2.0 * PI).abs() < 1e-4);
        assert_eq!(big.start_angle, 0.0);
        assert_eq!(big.end_angle, small.start_angle);
    }

    #[test]
    fn nests_deeper_rings_outside_shallower_ones() {
        let tree = nested_tree();
        let wedges = layout_wedges(&tree, 0, 2);

        let mid = wedges.iter().find(|w| w.target == WedgeTarget::Node(1)).unwrap();
        let leaf = wedges.iter().find(|w| w.target == WedgeTarget::Node(2)).unwrap();

        assert_eq!(mid.depth, 1);
        assert_eq!(leaf.depth, 2);
        assert_eq!(mid.outer_radius_frac, leaf.inner_radius_frac);
        assert_eq!(mid.start_angle, leaf.start_angle);
        assert_eq!(mid.end_angle, leaf.end_angle);
    }

    #[test]
    fn zero_size_children_produce_no_wedge() {
        let nodes = vec![
            node("root", 100, None, vec![1, 2]),
            node("real", 100, Some(0), vec![]),
            node("empty", 0, Some(0), vec![]),
        ];
        let path_index = nodes
            .iter()
            .enumerate()
            .map(|(ix, n)| (n.path.clone(), ix))
            .collect::<FxHashMap<_, _>>();
        let tree = DiskUsageTree { nodes, path_index };

        let wedges = layout_wedges(&tree, 0, 1);

        assert_eq!(wedges.len(), 1);
        assert_eq!(wedges[0].target, WedgeTarget::Node(1));
    }

    #[test]
    fn hit_test_round_trips_a_point_back_to_its_wedge() {
        let tree = flat_tree();
        let wedges = layout_wedges(&tree, 0, 1);
        let big = wedges.iter().find(|w| w.target == WedgeTarget::Node(1)).unwrap();

        let mid_angle = (big.start_angle + big.end_angle) / 2.0;
        let mid_radius = (big.inner_radius_frac + big.outer_radius_frac) / 2.0;

        assert_eq!(hit_test(&wedges, mid_angle, mid_radius), Some(WedgeTarget::Node(1)));
    }

    #[test]
    fn hit_test_returns_none_inside_the_center_hole() {
        let tree = flat_tree();
        let wedges = layout_wedges(&tree, 0, 1);

        assert_eq!(hit_test(&wedges, 0.0, CENTER_HOLE_FRAC / 2.0), None);
    }

    #[test]
    fn many_tiny_children_are_folded_into_one_aggregate_wedge() {
        let tiny_count = 10_000;
        let mut nodes = vec![node("root", 0, None, (1..=tiny_count + 1).collect())];
        nodes.push(node("big", 1_000_000, Some(0), vec![]));
        for _ in 0..tiny_count {
            nodes.push(node("tiny", 1, Some(0), vec![]));
        }
        nodes[0].size = 1_000_000 + tiny_count as u64;

        let path_index = nodes
            .iter()
            .enumerate()
            .map(|(ix, n)| (n.path.clone(), ix))
            .collect::<FxHashMap<_, _>>();
        let tree = DiskUsageTree { nodes, path_index };

        let wedges = layout_wedges(&tree, 0, 1);

        assert_eq!(wedges.len(), 2);
        let aggregate = wedges
            .iter()
            .find(|w| matches!(w.target, WedgeTarget::Aggregate { .. }))
            .expect("expected an aggregate wedge for the tiny children");
        assert_eq!(aggregate.target, WedgeTarget::Aggregate { size: tiny_count as u64, count: tiny_count });
    }
}
