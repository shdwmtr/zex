use std::path::PathBuf;

use gpui::{
    AnyElement, App, Context, IntoElement, KeyBinding, actions, deferred, div, prelude::*, px, svg,
};

use crate::app::assets;
use crate::explorer::Explorer;
use crate::theme;
use crate::ui::text_input::{Escape, TextInput as Input};

actions!(zex_path_bar, [CompletePath, SuggestionUp, SuggestionDown]);

pub fn init(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("tab", CompletePath, Some("Input")),
        KeyBinding::new("up", SuggestionUp, Some("Input")),
        KeyBinding::new("down", SuggestionDown, Some("Input")),
    ]);
}

#[derive(Clone, Copy)]
enum NavDirection {
    Back,
    Forward,
}

fn nav_button(
    id: &'static str,
    icon_asset: &str,
    enabled: bool,
    direction: NavDirection,
    cx: &Context<Explorer>,
) -> AnyElement {
    let color = if enabled {
        theme::text_primary()
    } else {
        theme::text_faint()
    };
    let icon_path = assets::assets_dir().join(icon_asset);

    let button = div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .size(px(22.0))
        .rounded_md()
        .child(
            svg()
                .path(icon_path.to_string_lossy().into_owned())
                .size(px(14.0))
                .text_color(color),
        );

    if enabled {
        button
            .cursor_pointer()
            .hover(|style| style.bg(theme::bg_breadcrumb_hover()))
            .on_click(
                cx.listener(move |explorer, _event, _window, cx| match direction {
                    NavDirection::Back => explorer.go_back(cx),
                    NavDirection::Forward => explorer.go_forward(cx),
                }),
            )
            .into_any_element()
    } else {
        button.into_any_element()
    }
}

fn path_bar_empty_space(cx: &Context<Explorer>) -> AnyElement {
    div()
        .id("path-bar-empty-space")
        .flex_1()
        .min_w(px(8.0))
        .h_full()
        .cursor_text()
        .on_click(cx.listener(|explorer, _event, window, cx| {
            explorer.begin_edit_path(window, cx);
        }))
        .into_any_element()
}

fn breadcrumbs(explorer: &Explorer, cx: &Context<Explorer>) -> Vec<AnyElement> {
    if explorer.is_trash() {
        return vec![
            div()
                .id("breadcrumb-trash")
                .px_1()
                .rounded_md()
                .text_color(theme::text_primary())
                .child("Trash")
                .into_any_element(),
            path_bar_empty_space(cx),
        ];
    }

    let mut segments: Vec<(String, PathBuf)> = Vec::new();
    let mut accumulated = PathBuf::new();

    for component in explorer.current_dir().components() {
        accumulated.push(component.as_os_str());
        let label = component.as_os_str().to_string_lossy().into_owned();
        let label = if label.is_empty() {
            "/".to_string()
        } else {
            label
        };
        segments.push((label, accumulated.clone()));
    }

    let mut children: Vec<AnyElement> = Vec::new();
    let mut prev_was_root = false;

    for (ix, (label, path)) in segments.into_iter().enumerate() {
        let is_root = label == "/";
        if ix > 0 && !prev_was_root {
            children.push(
                div()
                    .text_color(theme::text_faint())
                    .child("/")
                    .into_any_element(),
            );
        }
        let drop_path = path.clone();
        children.push(
            div()
                .id(ix)
                .px_1()
                .rounded_md()
                .cursor_pointer()
                .text_color(theme::text_muted())
                .hover(|style| {
                    style
                        .bg(theme::bg_breadcrumb_hover())
                        .text_color(theme::text_primary())
                })
                .on_click(cx.listener(move |explorer, _event, _window, cx| {
                    explorer.navigate_to(path.clone(), cx);
                }))
                .drag_over::<gpui::ExternalPaths>(|style, _paths, _window, _cx| {
                    style
                        .bg(theme::drop_target_fill())
                        .border_1()
                        .border_color(theme::drop_target_border())
                })
                .on_drop::<gpui::ExternalPaths>(cx.listener(
                    move |explorer, paths: &gpui::ExternalPaths, _window, cx| {
                        explorer.move_paths_into(paths.paths().to_vec(), drop_path.clone(), cx);
                    },
                ))
                .child(label)
                .into_any_element(),
        );
        prev_was_root = is_root;
    }

    children.push(path_bar_empty_space(cx));

    children
}

fn suggestions_popup(
    suggestions: &[PathBuf],
    selected: Option<usize>,
    cx: &Context<Explorer>,
) -> AnyElement {
    let panel = div()
        .absolute()
        .top(px(32.0))
        .left_0()
        .right_0()
        .flex()
        .flex_col()
        .bg(theme::bg_panel())
        .border_1()
        .border_color(theme::border())
        .rounded_md()
        .py_1()
        .children(suggestions.iter().enumerate().map(|(ix, path)| {
            let label = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string());
            let click_path = path.clone();
            let is_selected = selected == Some(ix);

            div()
                .id(ix)
                .px_2()
                .py_1()
                .cursor_pointer()
                .text_color(theme::text_primary())
                .when(is_selected, |el| el.bg(theme::bg_selected()))
                .hover(|style| style.bg(theme::bg_hover()))
                .on_click(cx.listener(move |explorer, _event, _window, cx| {
                    explorer.accept_path_suggestion(click_path.clone(), cx);
                }))
                .child(label)
        }));

    deferred(panel).with_priority(1).into_any_element()
}

pub fn render(explorer: &Explorer, cx: &Context<Explorer>) -> impl IntoElement {
    let body: AnyElement = match &explorer.editing_path {
        Some(editing) => div()
            .relative()
            .flex_1()
            .on_action(
                cx.listener(|explorer, _: &Escape, _window, cx| explorer.cancel_edit_path(cx)),
            )
            .on_action(cx.listener(|explorer, _: &CompletePath, _window, cx| {
                explorer.complete_path_suggestion(cx)
            }))
            .on_action(cx.listener(|explorer, _: &SuggestionDown, _window, cx| {
                explorer.select_next_suggestion(cx)
            }))
            .on_action(cx.listener(|explorer, _: &SuggestionUp, _window, cx| {
                explorer.select_prev_suggestion(cx)
            }))
            .child(Input::new(&editing.input))
            .when(!editing.suggestions.is_empty(), |el| {
                el.child(suggestions_popup(
                    &editing.suggestions,
                    editing.selected_suggestion,
                    cx,
                ))
            })
            .into_any_element(),
        None => div()
            .flex_1()
            .flex()
            .flex_row()
            .items_center()
            .gap_1()
            .h_full()
            .children(breadcrumbs(explorer, cx))
            .into_any_element(),
    };

    div()
        .id("path-bar")
        .w_full()
        .flex()
        .flex_row()
        .items_center()
        .gap_1()
        .px_3()
        .py_2()
        .bg(theme::bg_bar())
        .child(nav_button(
            "go-back",
            "icons/chevron-left.svg",
            explorer.can_go_back(),
            NavDirection::Back,
            cx,
        ))
        .child(nav_button(
            "go-forward",
            "icons/chevron-right.svg",
            explorer.can_go_forward(),
            NavDirection::Forward,
            cx,
        ))
        .child(body)
}
