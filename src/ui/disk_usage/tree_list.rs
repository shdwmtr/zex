use std::cmp::Ordering;
use std::ops::Range;
use std::path::Path;
use std::sync::Arc;

use gpui::{
    Context, DragMoveEvent, Entity, InteractiveElement, IntoElement, MouseButton, MouseUpEvent,
    ParentElement, StatefulInteractiveElement, Styled, Window, div, prelude::*, px, relative,
    uniform_list,
};

use crate::explorer::Explorer;
use crate::explorer::columns::SortDirection;
use crate::explorer::disk_usage::{DiskUsageColumn, DiskUsageColumnWidths, DiskUsageSortColumn};
use crate::explorer::drag::ScrollbarId;
use crate::filesystem::disk_usage::{DiskUsageTree, NodeId, NodeKind};
use crate::filesystem::entry::{format_modified, format_size};
use crate::theme;
use crate::ui::column_header;
use crate::ui::context_menu;
use crate::ui::popup_menu::ContextMenuExt;
use crate::ui::scrollbar::Scrollbar;

use super::palette;

const RESIZE_HIT_WIDTH: f32 = column_header::RESIZE_HIT_WIDTH;
const HEADER_HEIGHT: f32 = 28.0;
const NAME_MIN_WIDTH: f32 = 100.0;
const SPACER_WIDTH: f32 = 8.0;
const BAR_WIDTH: f32 = 70.0;
const LEADING_PREFIX_WIDTH: f32 = BAR_WIDTH + SPACER_WIDTH;

pub fn render(explorer: &Explorer, tree: Arc<DiskUsageTree>, root_id: NodeId, cx: &Context<Explorer>) -> impl IntoElement {
    let state = explorer.disk_usage.as_ref().unwrap();
    let sort = (state.sort_column, state.sort_direction);
    let widths = state.column_widths;
    let entity = cx.entity();
    let scroll_handle = state.tree_scroll_handle.clone();
    let selected = state.selected_row.clone();

    let visible = visible_rows(&tree, root_id, sort);
    let row_count = visible.len();

    let row_entity = entity.clone();

    div()
        .id("disk-usage-tree-list")
        .relative()
        .w(px(state.list_width))
        .flex_shrink_0()
        .flex()
        .flex_col()
        .border_r_1()
        .border_color(theme::border())
        .child(header_row(sort, widths, entity.clone(), cx))
        .child(
            div()
                .id("disk-usage-tree-rows")
                .relative()
                .flex()
                .flex_1()
                .w_full()
                .overflow_hidden()
                .child(
                    uniform_list(
                        "disk-usage-tree-rows-list",
                        row_count,
                        cx.processor(move |_explorer: &mut Explorer, range: Range<usize>, _window: &mut Window, cx| {
                            range
                                .map(|ix| {
                                    let id = visible[ix];
                                    row(&tree, id, ix, selected.as_deref(), widths, row_entity.clone(), cx)
                                        .into_any_element()
                                })
                                .collect::<Vec<_>>()
                        }),
                    )
                    .flex_1()
                    .track_scroll(scroll_handle.clone()),
                )
                .child(Scrollbar::vertical_for_uniform_list(
                    &scroll_handle,
                    entity.clone(),
                    ScrollbarId::DiskUsageTree,
                )),
        )
        .child(column_header::divider_overlay(
            px(HEADER_HEIGHT),
            LEADING_PREFIX_WIDTH + NAME_MIN_WIDTH,
            [
                widths.get(DiskUsageColumn::Size),
                widths.get(DiskUsageColumn::Contents),
                widths.get(DiskUsageColumn::Modified),
            ],
        ))
}

fn visible_rows(tree: &DiskUsageTree, root_id: NodeId, sort: (DiskUsageSortColumn, SortDirection)) -> Vec<NodeId> {
    let mut children = tree.get(root_id).children.clone();
    children.sort_by(|&a, &b| compare_nodes(tree, a, b, sort));
    children
}

fn compare_nodes(
    tree: &DiskUsageTree,
    a: NodeId,
    b: NodeId,
    (column, direction): (DiskUsageSortColumn, SortDirection),
) -> Ordering {
    let (na, nb) = (tree.get(a), tree.get(b));
    let ordering = match column {
        DiskUsageSortColumn::Name => na.name.to_lowercase().cmp(&nb.name.to_lowercase()),
        DiskUsageSortColumn::Size => na.size.cmp(&nb.size),
        DiskUsageSortColumn::Contents => na.item_count.cmp(&nb.item_count),
        DiskUsageSortColumn::Modified => na.modified.cmp(&nb.modified),
    };
    match direction {
        SortDirection::Ascending => ordering,
        SortDirection::Descending => ordering.reverse(),
    }
}

fn row(
    tree: &DiskUsageTree,
    id: NodeId,
    ix: usize,
    selected: Option<&Path>,
    widths: DiskUsageColumnWidths,
    entity: Entity<Explorer>,
    cx: &Context<Explorer>,
) -> impl IntoElement {
    let node = tree.get(id);
    let path = node.path.clone();
    let is_selected = selected == Some(path.as_path());
    let total = node
        .parent
        .map(|parent_id| tree.get(parent_id).size.max(1))
        .unwrap_or(node.size.max(1));
    let frac = (node.size as f32 / total as f32).clamp(0.0, 1.0);
    let bar_color = match node.kind {
        NodeKind::Inaccessible | NodeKind::MountBoundary => palette::inaccessible_color(),
        NodeKind::File | NodeKind::Directory => palette::wedge_color(ix, 1),
    };
    let badge = match node.kind {
        NodeKind::Inaccessible => Some("no access"),
        NodeKind::MountBoundary => Some("other filesystem"),
        NodeKind::File | NodeKind::Directory => None,
    };
    let click_path = path.clone();
    let menu_path = path.clone();
    let is_dir = node.kind == NodeKind::Directory;

    div()
        .id(("disk-usage-tree-row", ix))
        .flex()
        .flex_row()
        .items_center()
        .w_full()
        .px_3()
        .py_1()
        .cursor_pointer()
        .context_menu(move |menu, window, cx| {
            context_menu::disk_usage_row_menu(entity.clone(), menu_path.clone(), menu, window, cx)
        })
        .when(is_selected, |el| el.bg(theme::bg_selected()))
        .hover(|style| style.bg(theme::bg_hover()))
        .on_click(cx.listener(move |explorer, _, _window, cx| {
            explorer.select_disk_usage_row(click_path.clone(), cx);
            if is_dir {
                explorer.drill_into(click_path.clone(), cx);
            }
        }))
        .child(
            div()
                .w(px(BAR_WIDTH))
                .flex_shrink_0()
                .h(px(6.0))
                .rounded_full()
                .bg(theme::bg_hover())
                .child(div().h_full().rounded_full().bg(bar_color).w(relative(frac))),
        )
        .child(div().w(px(SPACER_WIDTH)).flex_shrink_0())
        .child(
            div()
                .flex_1()
                .min_w(px(NAME_MIN_WIDTH))
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .overflow_hidden()
                .child(div().truncate().child(node.name.clone()))
                .children(badge.map(|label| {
                    div()
                        .flex_shrink_0()
                        .text_color(theme::text_faint())
                        .child(format!("({label})"))
                })),
        )
        .child(column_cell(widths.get(DiskUsageColumn::Size), format_size(node.size)))
        .child(column_cell(widths.get(DiskUsageColumn::Contents), node.item_count.to_string()))
        .child(column_cell(widths.get(DiskUsageColumn::Modified), format_modified(node.modified)))
}

fn column_cell(width: f32, text: impl IntoElement) -> impl IntoElement {
    div()
        .w(px(width))
        .flex_shrink_0()
        .pl_2()
        .text_color(theme::text_muted())
        .child(text)
}

fn header_row(
    sort: (DiskUsageSortColumn, SortDirection),
    widths: DiskUsageColumnWidths,
    entity: Entity<Explorer>,
    cx: &Context<Explorer>,
) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .items_center()
        .h(px(HEADER_HEIGHT))
        .px_3()
        .bg(theme::bg_header())
        .text_color(theme::text_muted())
        .border_b_1()
        .border_color(theme::border())
        .child(div().w(px(BAR_WIDTH)).flex_shrink_0())
        .child(div().w(px(SPACER_WIDTH)).flex_shrink_0())
        .child(sort_header_cell("Name", DiskUsageSortColumn::Name, sort, None, cx))
        .child(resizable_header_cell(
            "Size",
            DiskUsageSortColumn::Size,
            DiskUsageColumn::Size,
            widths.get(DiskUsageColumn::Size),
            sort,
            entity.clone(),
            cx,
        ))
        .child(resizable_header_cell(
            "Contents",
            DiskUsageSortColumn::Contents,
            DiskUsageColumn::Contents,
            widths.get(DiskUsageColumn::Contents),
            sort,
            entity.clone(),
            cx,
        ))
        .child(resizable_header_cell(
            "Modified",
            DiskUsageSortColumn::Modified,
            DiskUsageColumn::Modified,
            widths.get(DiskUsageColumn::Modified),
            sort,
            entity,
            cx,
        ))
}

fn sort_header_cell(
    label: &'static str,
    column: DiskUsageSortColumn,
    (active_column, direction): (DiskUsageSortColumn, SortDirection),
    width: Option<f32>,
    cx: &Context<Explorer>,
) -> gpui::Stateful<gpui::Div> {
    let is_active = active_column == column;
    let direction = if is_active { direction } else { SortDirection::Ascending };

    column_header::sort_cell(
        ("disk-usage-sort-header", column as u8 as u64),
        format!("disk-usage-sort-header-{column:?}"),
        label,
        width,
        NAME_MIN_WIDTH,
        is_active,
        direction,
        cx.listener(move |explorer, _event: &gpui::ClickEvent, _window, cx| {
            explorer.set_disk_usage_sort(column, cx);
        }),
    )
}

#[allow(clippy::too_many_arguments)]
fn resizable_header_cell(
    label: &'static str,
    sort_column: DiskUsageSortColumn,
    resize_column: DiskUsageColumn,
    width: f32,
    sort: (DiskUsageSortColumn, SortDirection),
    entity: Entity<Explorer>,
    cx: &Context<Explorer>,
) -> impl IntoElement {
    let drag_entity = entity;

    sort_header_cell(label, sort_column, sort, Some(width), cx)
        .relative()
        .child(column_header::resize_handle(
            ("disk-usage-column-resize", resize_column as u8 as u64),
            resize_column,
            move |_column, _point, window, cx| {
                let anchor_x = f32::from(window.mouse_position().x);
                drag_entity.update(cx, |explorer, cx| {
                    explorer.begin_disk_usage_column_resize(resize_column, anchor_x, cx);
                });
                cx.new(|_| column_header::ResizeGhost)
            },
            cx.listener(move |explorer, event: &DragMoveEvent<DiskUsageColumn>, _window, cx| {
                explorer.update_disk_usage_column_resize(f32::from(event.event.position.x), cx);
            }),
            cx.listener(|explorer, _event: &MouseUpEvent, _window, cx| {
                explorer.end_disk_usage_column_resize(cx);
            }),
        ))
}

pub fn list_resize_handle(
    list_width: f32,
    active: bool,
    entity: Entity<Explorer>,
    cx: &Context<Explorer>,
) -> impl IntoElement {
    let drag_entity = entity;

    div()
        .id("disk-usage-list-resize-handle")
        .absolute()
        .top_0()
        .bottom_0()
        .left(px(list_width - RESIZE_HIT_WIDTH / 2.0))
        .w(px(RESIZE_HIT_WIDTH))
        .occlude()
        .flex()
        .justify_center()
        .cursor_col_resize()
        .child(
            div()
                .w(px(1.0))
                .h_full()
                .when(active, |bar| bar.bg(theme::bg_selected())),
        )
        .on_drag((), move |_, _point, window, cx| {
            let anchor_x = f32::from(window.mouse_position().x);
            drag_entity.update(cx, |explorer, cx| {
                explorer.begin_disk_usage_list_resize(anchor_x, cx);
            });
            cx.new(|_| column_header::ResizeGhost)
        })
        .on_drag_move::<()>(cx.listener(move |explorer, event: &DragMoveEvent<()>, _window, cx| {
            explorer.update_disk_usage_list_resize(f32::from(event.event.position.x), cx);
        }))
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|explorer, _event: &MouseUpEvent, _window, cx| {
                explorer.end_disk_usage_list_resize(cx);
            }),
        )
}
