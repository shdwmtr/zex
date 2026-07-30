use gpui::{Context, ElementId, IntoElement, Stateful, div, prelude::*, px, svg};

use crate::app::assets;
use crate::explorer::Explorer;
use crate::filesystem::entry::format_size;
use crate::theme;

fn status_button(id: impl Into<ElementId>) -> Stateful<gpui::Div> {
    div()
        .id(id)
        .flex()
        .flex_row()
        .items_center()
        .gap_1()
        .px_2()
        .py_1()
        .rounded_md()
        .cursor_pointer()
        .text_color(theme::text_muted())
        .hover(|style| {
            style
                .bg(theme::bg_breadcrumb_hover())
                .text_color(theme::text_primary())
        })
}

fn git_indicator(explorer: &Explorer, cx: &Context<Explorer>) -> Option<impl IntoElement> {
    if !explorer.git_settings.enabled || !explorer.git_settings.branch.show_in_status_bar {
        return None;
    }
    let branch = explorer.git_branch_label()?;

    let dirty_dot = (explorer.git_settings.branch.show_dirty_indicator && explorer.git_is_dirty())
        .then(|| {
            div()
                .flex_shrink_0()
                .w(gpui::px(6.0))
                .h(gpui::px(6.0))
                .rounded_full()
                .bg(theme::git_color_modified())
        });

    let ahead_behind = explorer
        .git_settings
        .branch
        .show_ahead_behind
        .then(|| explorer.git_ahead_behind())
        .flatten()
        .filter(|(ahead, behind)| *ahead > 0 || *behind > 0)
        .map(|(ahead, behind)| {
            let mut label = String::new();
            if ahead > 0 {
                label.push_str(&format!("↑{ahead}"));
            }
            if behind > 0 {
                if !label.is_empty() {
                    label.push(' ');
                }
                label.push_str(&format!("↓{behind}"));
            }
            label
        });

    let icon_path = assets::assets_dir().join("icons/git-branch.svg");

    Some(
        status_button("git-branch-button")
            .on_click(cx.listener(|explorer, _event: &gpui::ClickEvent, _window, cx| {
                explorer.refresh_git(cx);
            }))
            .child(
                svg()
                    .flex_shrink_0()
                    .path(icon_path.to_string_lossy().into_owned())
                    .size(px(12.0))
                    .text_color(theme::text_muted()),
            )
            .children(dirty_dot)
            .child(branch.to_string())
            .children(ahead_behind),
    )
}

pub fn render(explorer: &Explorer, cx: &Context<Explorer>) -> impl IntoElement {
    let git_indicator = git_indicator(explorer, cx);
    let item_count = if explorer.is_trash() {
        explorer.trash_entries.len()
    } else {
        explorer.entries.len()
    };
    let item_label = if item_count == 1 {
        "1 item".to_string()
    } else {
        format!("{item_count} items")
    };

    let selection_label = if explorer.selected.is_empty() {
        None
    } else {
        let total: u64 = if explorer.is_trash() {
            explorer
                .trash_entries
                .iter()
                .filter(|entry| explorer.selected.contains(&entry.id_path))
                .map(|entry| entry.size)
                .sum()
        } else {
            explorer
                .entries
                .iter()
                .filter(|entry| explorer.selected.contains(&entry.path))
                .map(|entry| entry.size)
                .sum()
        };
        Some(format!(
            "{} selected ({})",
            explorer.selected.len(),
            format_size(total)
        ))
    };

    div()
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .px_3()
        .py_1()
        .bg(theme::bg_bar())
        .text_color(theme::text_muted())
        .child(
            div()
                .flex()
                .flex_row()
                .gap_2()
                .child(item_label)
                .children(selection_label),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .children(git_indicator)
                .child(
                    status_button("free-space-button")
                        .on_click(cx.listener(|explorer, _event: &gpui::ClickEvent, window, cx| {
                            explorer.open_disk_usage(window, cx);
                        }))
                        .child(explorer.free_space_label.clone()),
                ),
        )
}
