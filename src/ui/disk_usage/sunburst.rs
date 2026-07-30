use std::f32::consts::PI;
use std::sync::Arc;

use gpui::{
    AnyElement, BoxShadow, Bounds, ClickEvent, Context, DispatchPhase, FontWeight,
    InteractiveElement, IntoElement, MouseMoveEvent, ParentElement, Path, PathBuilder, Pixels,
    Point, Styled, Window, canvas, div, point, prelude::*, px,
};

use crate::explorer::Explorer;
use crate::filesystem::disk_usage::{DiskUsageTree, NodeId, NodeKind};
use crate::filesystem::entry::format_size;
use crate::theme;

use super::geometry::{CENTER_HOLE_FRAC, WedgeGeometry, WedgeTarget};
use super::palette;

pub fn render(
    explorer: &Explorer,
    tree: Arc<DiskUsageTree>,
    root_id: NodeId,
    cx: &Context<Explorer>,
) -> impl IntoElement {
    let state = explorer.disk_usage.as_ref().unwrap();
    let root_node = tree.get(root_id);
    let center_name = root_node.name.clone();
    let center_size = format_size(root_node.size);

    let wedges = state.wedges.clone();
    let hovered = state.hovered_wedge;
    let selected = state
        .selected_row
        .as_deref()
        .and_then(|path| tree.find(path))
        .map(WedgeTarget::Node);
    let entity = cx.entity();
    let paint_tree = tree.clone();

    div()
        .id("disk-usage-sunburst")
        .relative()
        .flex_1()
        .size_full()
        .on_click(cx.listener(|explorer, _: &ClickEvent, _window, cx| {
            explorer.click_sunburst(cx);
        }))
        .child(
            canvas(
                move |bounds, _window, _cx| bounds,
                move |bounds, _prepaint, window, _cx| {
                    paint_sunburst(bounds, &wedges, &paint_tree, hovered, selected, window);

                    let entity = entity.clone();
                    window.on_mouse_event(move |event: &MouseMoveEvent, phase, _window, cx| {
                        if phase != DispatchPhase::Bubble {
                            return;
                        }
                        entity.update(cx, |explorer, cx| {
                            explorer.update_sunburst_hover(bounds, event.position, cx);
                        });
                    });
                },
            )
            .size_full(),
        )
        .child(
            div()
                .absolute()
                .inset_0()
                .flex()
                .flex_col()
                .items_center()
                .justify_center()
                .gap_1()
                .child(div().font_weight(FontWeight::BOLD).child(center_name))
                .child(div().text_color(theme::text_muted()).child(center_size)),
        )
        .children(tooltip(&tree, state))
}

fn tooltip(tree: &DiskUsageTree, state: &crate::explorer::disk_usage::DiskUsageState) -> Option<AnyElement> {
    let target = state.hovered_wedge?;
    let pos = state.hover_local_pos?;
    let wedge = state.wedges.iter().find(|w| w.target == target)?;

    let (name, size, item_count): (String, u64, u64) = match target {
        WedgeTarget::Node(id) => {
            let node = tree.get(id);
            (node.name.clone(), node.size, node.item_count)
        }
        WedgeTarget::Aggregate { size, count } => ("Other".to_string(), size, count as u64),
    };
    let percent = size as f32 / wedge.parent_total.max(1) as f32 * 100.0;

    Some(
        div()
            .absolute()
            .left(pos.x + px(12.0))
            .top(pos.y + px(12.0))
            .flex()
            .flex_col()
            .gap_1()
            .p_2()
            .max_w(px(280.0))
            .bg(theme::bg_panel())
            .border_1()
            .border_color(theme::border())
            .shadow(vec![BoxShadow {
                color: gpui::Hsla { h: 0., s: 0., l: 0., a: 0.4 },
                blur_radius: px(12.0),
                spread_radius: px(0.),
                offset: Point::new(px(0.0), px(2.0)),
            }])
            .child(div().font_weight(FontWeight::BOLD).truncate().child(name))
            .child(
                div()
                    .text_color(theme::text_muted())
                    .child(format!("{} • {:.1}%", format_size(size), percent)),
            )
            .child(
                div()
                    .text_color(theme::text_muted())
                    .child(format!("{item_count} items")),
            )
            .into_any_element(),
    )
}

fn paint_sunburst(
    bounds: Bounds<Pixels>,
    wedges: &[WedgeGeometry],
    tree: &DiskUsageTree,
    hovered: Option<WedgeTarget>,
    selected: Option<WedgeTarget>,
    window: &mut Window,
) {
    let center = bounds.center();
    let max_radius = f32::from(bounds.size.width).min(f32::from(bounds.size.height)) / 2.0 * 0.95;

    for wedge in wedges {
        let mut color = match wedge.target {
            WedgeTarget::Node(id) => match tree.get(id).kind {
                NodeKind::Inaccessible | NodeKind::MountBoundary => palette::inaccessible_color(),
                NodeKind::File | NodeKind::Directory => palette::wedge_color(wedge.branch_index, wedge.depth),
            },
            WedgeTarget::Aggregate { .. } => palette::aggregate_color(),
        };
        if Some(wedge.target) == hovered || Some(wedge.target) == selected {
            color = palette::highlight(color);
        }

        if let Some(path) = wedge_path(center, max_radius, wedge) {
            window.paint_path(path, color);
        }
    }

    if let Some(hole) = circle_path(center, max_radius * CENTER_HOLE_FRAC) {
        window.paint_path(hole, theme::bg_panel());
    }
}

fn wedge_point(center: Point<Pixels>, radius: f32, angle: f32) -> Point<Pixels> {
    point(center.x + px(radius * angle.sin()), center.y - px(radius * angle.cos()))
}

fn wedge_path(center: Point<Pixels>, max_radius: f32, wedge: &WedgeGeometry) -> Option<Path<Pixels>> {
    let inner = wedge.inner_radius_frac * max_radius;
    let outer = wedge.outer_radius_frac * max_radius;
    let large_arc = wedge.end_angle - wedge.start_angle > PI;

    let mut builder = PathBuilder::fill();
    builder.move_to(wedge_point(center, inner, wedge.start_angle));
    builder.line_to(wedge_point(center, outer, wedge.start_angle));
    builder.arc_to(
        point(px(outer), px(outer)),
        px(0.0),
        large_arc,
        true,
        wedge_point(center, outer, wedge.end_angle),
    );
    builder.line_to(wedge_point(center, inner, wedge.end_angle));
    builder.arc_to(
        point(px(inner), px(inner)),
        px(0.0),
        large_arc,
        false,
        wedge_point(center, inner, wedge.start_angle),
    );
    builder.close();
    builder.build().ok()
}

fn circle_path(center: Point<Pixels>, radius: f32) -> Option<Path<Pixels>> {
    if radius <= 0.0 {
        return None;
    }
    let r = px(radius);
    let mut builder = PathBuilder::fill();
    builder.move_to(point(center.x + r, center.y));
    builder.arc_to(point(r, r), px(0.0), false, false, point(center.x - r, center.y));
    builder.arc_to(point(r, r), px(0.0), false, false, point(center.x + r, center.y));
    builder.close();
    builder.build().ok()
}
