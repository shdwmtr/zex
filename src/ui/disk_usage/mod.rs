pub mod geometry;
pub mod palette;
pub mod sunburst;
pub mod tree_list;

use std::sync::Arc;

use gpui::{
    AnyElement, Context, FontWeight, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div, px,
};

use crate::explorer::Explorer;
use crate::explorer::disk_usage::ScanState;
use crate::filesystem::disk_usage::DiskUsageTree;
use crate::filesystem::entry::format_size;
use crate::keys;
use crate::theme;
use crate::ui::path_bar;
use crate::ui::warning_dialog;

pub fn render_panel(explorer: &Explorer, cx: &Context<Explorer>) -> impl IntoElement {
    let state = explorer.disk_usage.as_ref().unwrap();

    let content: AnyElement = match &state.scan {
        ScanState::Scanning { files_scanned, bytes_scanned } => {
            render_scanning_screen(*files_scanned, *bytes_scanned, cx).into_any_element()
        }
        ScanState::Failed { error } => render_error_screen(error, cx).into_any_element(),
        ScanState::Ready { tree } => render_ready_screen(explorer, tree.clone(), cx).into_any_element(),
    };

    div()
        .id("disk-usage-root")
        .key_context("DiskUsage")
        .track_focus(&state.focus_handle)
        .on_action(cx.listener(|explorer, _: &keys::CloseDiskUsage, window, cx| {
            explorer.close_disk_usage(window, cx);
        }))
        .size_full()
        .flex()
        .flex_col()
        .bg(theme::bg_root())
        .text_color(theme::text_primary())
        .child(path_bar::render(explorer, cx))
        .children(explorer.op_error.as_ref().map(|message| {
            div()
                .w_full()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .gap_2()
                .px_3()
                .py_1()
                .bg(theme::bg_error())
                .text_color(theme::text_error())
                .child(message.clone())
                .child(
                    div()
                        .id("disk-usage-dismiss-op-error")
                        .cursor_pointer()
                        .px_2()
                        .on_click(cx.listener(|explorer, _, _, cx| explorer.dismiss_op_error(cx)))
                        .child("×"),
                )
        }))
        .child(content)
        .children(warning_dialog::render(explorer, cx))
}

fn centered_panel(content: impl IntoElement) -> impl IntoElement {
    div().flex_1().flex().items_center().justify_center().child(
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_3()
            .w(px(360.0))
            .p_4()
            .bg(theme::bg_panel())
            .border_1()
            .border_color(theme::border())
            .child(content),
    )
}

fn render_scanning_screen(
    files_scanned: u64,
    bytes_scanned: u64,
    cx: &Context<Explorer>,
) -> impl IntoElement {
    centered_panel(
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_3()
            .child(div().font_weight(FontWeight::BOLD).child("Scanning disk…"))
            .child(div().text_color(theme::text_muted()).child(format!(
                "{files_scanned} files • {} found",
                format_size(bytes_scanned)
            )))
            .child(
                div()
                    .id("disk-usage-cancel")
                    .cursor_pointer()
                    .px_3()
                    .py_1()
                    .border_1()
                    .border_color(theme::border())
                    .hover(|style| style.bg(theme::bg_hover()))
                    .on_click(cx.listener(|explorer, _, window, cx| {
                        explorer.close_disk_usage(window, cx);
                    }))
                    .child("Cancel"),
            ),
    )
}

fn render_error_screen(error: &str, cx: &Context<Explorer>) -> impl IntoElement {
    centered_panel(
        div()
            .flex()
            .flex_col()
            .items_center()
            .gap_3()
            .child(div().text_color(theme::text_error()).child(error.to_string()))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .gap_2()
                    .child(
                        div()
                            .id("disk-usage-retry")
                            .cursor_pointer()
                            .px_3()
                            .py_1()
                            .border_1()
                            .border_color(theme::border())
                            .hover(|style| style.bg(theme::bg_hover()))
                            .on_click(cx.listener(|explorer, _, _window, cx| {
                                explorer.refresh_scan(cx);
                            }))
                            .child("Retry"),
                    )
                    .child(
                        div()
                            .id("disk-usage-error-close")
                            .cursor_pointer()
                            .px_3()
                            .py_1()
                            .border_1()
                            .border_color(theme::border())
                            .hover(|style| style.bg(theme::bg_hover()))
                            .on_click(cx.listener(|explorer, _, window, cx| {
                                explorer.close_disk_usage(window, cx);
                            }))
                            .child("Close"),
                    ),
            ),
    )
}

fn render_ready_screen(
    explorer: &Explorer,
    tree: Arc<DiskUsageTree>,
    cx: &Context<Explorer>,
) -> impl IntoElement {
    let state = explorer.disk_usage.as_ref().unwrap();
    let root_id = tree.find(&state.current_root).unwrap_or(tree.root());
    let list_width = state.list_width;
    let list_resize_active = state.list_resize_drag.is_some();
    let entity = cx.entity();

    div()
        .relative()
        .flex()
        .flex_row()
        .flex_1()
        .overflow_hidden()
        .child(tree_list::render(explorer, tree.clone(), root_id, cx))
        .child(sunburst::render(explorer, tree, root_id, cx))
        .child(tree_list::list_resize_handle(
            list_width,
            list_resize_active,
            entity,
            cx,
        ))
}
