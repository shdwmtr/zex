use gpui::{Context, IntoElement, div, prelude::*};

use crate::explorer::Explorer;
use crate::filesystem::entry::format_size;
use crate::theme;

pub fn render(explorer: &Explorer, _cx: &Context<Explorer>) -> impl IntoElement {
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
        .child(explorer.free_space_label.clone())
}
