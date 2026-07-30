use std::path::PathBuf;

use gpui::{
    Context, DragMoveEvent, Entity, IntoElement, MouseButton, MouseUpEvent, Render, Window, div,
    prelude::*, px,
};

use crate::explorer::Explorer;
use crate::explorer::drag::ScrollbarId;
use crate::settings::SidebarItem;
use crate::theme;
use crate::theme::icon_theme;
use crate::ui::scrollbar::Scrollbar;

const RESIZE_HIT_WIDTH: f32 = 17.0;

struct SidebarResizeGhost;

impl Render for SidebarResizeGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

pub fn resize_handle(
    explorer: Entity<Explorer>,
    sidebar_width: f32,
    active: bool,
    cx: &Context<Explorer>,
) -> impl IntoElement {
    let drag_explorer = explorer.clone();

    div()
        .id("sidebar-resize-handle")
        .absolute()
        .top_0()
        .bottom_0()
        .left(px(sidebar_width - RESIZE_HIT_WIDTH / 2.0))
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
            drag_explorer.update(cx, |explorer, cx| {
                explorer.begin_sidebar_resize(anchor_x, cx);
            });
            cx.new(|_| SidebarResizeGhost)
        })
        .on_drag_move::<()>(
            cx.listener(move |explorer, event: &DragMoveEvent<()>, _window, cx| {
                explorer.update_sidebar_resize(f32::from(event.event.position.x), cx);
            }),
        )
        .on_mouse_up(
            MouseButton::Left,
            cx.listener(|explorer, _event: &MouseUpEvent, _window, cx| {
                explorer.end_sidebar_resize(cx);
            }),
        )
}

struct Place {
    label: String,
    path: PathBuf,
}

enum Row {
    Header(String),
    Place(Place),
}

fn rows(sidebar_entries: &[SidebarItem]) -> Vec<Row> {
    let mut rows = Vec::new();

    for item in sidebar_entries {
        match item {
            SidebarItem::Entry(entry) => rows.push(Row::Place(Place {
                label: entry.name.clone(),
                path: PathBuf::from(&entry.path),
            })),
            SidebarItem::Section(section) => {
                rows.push(Row::Header(section.section.clone()));
                rows.extend(section.entries.iter().map(|entry| {
                    Row::Place(Place {
                        label: entry.name.clone(),
                        path: PathBuf::from(&entry.path),
                    })
                }));
            }
        }
    }

    rows
}

pub fn render(explorer: &Explorer, cx: &Context<Explorer>) -> impl IntoElement {
    let entity = cx.entity();
    let scroll_handle = explorer.sidebar_scroll_handle.clone();

    div()
        .id("sidebar")
        .relative()
        .w(px(explorer.sidebar_width))
        .flex_shrink_0()
        .h_full()
        .flex()
        .flex_col()
        .bg(theme::bg_panel())
        .border_r_1()
        .border_color(theme::border())
        .child(
            div()
                .id("sidebar-rows")
                .relative()
                .flex()
                .flex_1()
                .w_full()
                .overflow_hidden()
                .child(
                    div()
                        .id("sidebar-rows-scroll")
                        .flex()
                        .flex_1()
                        .flex_col()
                        .w_full()
                        .py_2()
                        .overflow_y_scroll()
                        .track_scroll(&scroll_handle)
                        .children(
                            rows(&explorer.sidebar_entries)
                                .into_iter()
                                .enumerate()
                                .map(|(ix, row)| match row {
                                    Row::Header(title) => div()
                                        .id(ix)
                                        .w_full()
                                        .px_3()
                                        .pt_2()
                                        .pb_1()
                                        .text_xs()
                                        .text_color(theme::text_faint())
                                        .child(title)
                                        .into_any_element(),
                                    Row::Place(place) => {
                                        let is_active = place.path == explorer.current_dir();
                                        let path = place.path.clone();
                                        let drop_path = place.path.clone();

                                        div()
                                            .id(ix)
                                            .w_full()
                                            .flex()
                                            .flex_row()
                                            .items_center()
                                            .gap_2()
                                            .px_3()
                                            .py_1()
                                            .cursor_pointer()
                                            .when(is_active, |this| this.bg(theme::bg_sidebar_selected()))
                                            .hover(|style| style.bg(theme::bg_sidebar_hover()))
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
                                                    explorer.move_paths_into(
                                                        paths.paths().to_vec(),
                                                        drop_path.clone(),
                                                        cx,
                                                    );
                                                },
                                            ))
                                            .child(icon_theme::directory_svg_icon(cx))
                                            .child(place.label)
                                            .into_any_element()
                                    }
                                }),
                        ),
                )
                .child(Scrollbar::vertical(&scroll_handle, entity.clone(), ScrollbarId::Sidebar)),
        )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::{SidebarEntry, SidebarSection};

    fn entry(name: &str, path: &str) -> SidebarEntry {
        SidebarEntry {
            name: name.into(),
            path: path.into(),
        }
    }

    fn place_labels_and_paths(rows: &[Row]) -> Vec<(&str, PathBuf)> {
        rows.iter()
            .filter_map(|row| match row {
                Row::Place(place) => Some((place.label.as_str(), place.path.clone())),
                Row::Header(_) => None,
            })
            .collect()
    }

    #[test]
    fn builds_places_from_configured_sidebar_entries_in_order() {
        let entries = vec![
            SidebarItem::Entry(entry("Projects", "/home/user/Projects")),
            SidebarItem::Entry(entry("Root", "/")),
        ];

        let rows = rows(&entries);

        assert_eq!(
            place_labels_and_paths(&rows),
            vec![
                ("Projects", PathBuf::from("/home/user/Projects")),
                ("Root", PathBuf::from("/")),
            ]
        );
    }

    #[test]
    fn empty_config_yields_no_rows() {
        assert!(rows(&[]).is_empty());
    }

    #[test]
    fn sections_render_a_header_followed_by_their_entries() {
        let entries = vec![
            SidebarItem::Entry(entry("Root", "/")),
            SidebarItem::Section(SidebarSection {
                section: "Work".into(),
                entries: vec![
                    entry("Projects", "/home/user/Projects"),
                    entry("Downloads", "/home/user/Downloads"),
                ],
            }),
        ];

        let rows = rows(&entries);

        assert!(matches!(&rows[0], Row::Place(place) if place.label == "Root"));
        assert!(matches!(&rows[1], Row::Header(title) if title == "Work"));
        assert!(matches!(&rows[2], Row::Place(place) if place.label == "Projects"));
        assert!(matches!(&rows[3], Row::Place(place) if place.label == "Downloads"));
        assert_eq!(rows.len(), 4);
    }
}
