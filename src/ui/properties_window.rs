use gpui::{
    AnyElement, BoxShadow, Context, FontWeight, Hsla, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Point, SharedString, StatefulInteractiveElement, Styled, div,
    prelude::FluentBuilder as _, px,
};

use crate::explorer::Explorer;
use crate::explorer::properties::{PropertiesTab, StatsState};
use crate::filesystem::entry::{self, FsEntry, format_modified, format_size};
use crate::theme;
use crate::theme::icon_theme;

fn row(label: &'static str, value: SharedString) -> impl IntoElement {
    div()
        .flex()
        .flex_row()
        .gap_3()
        .child(
            div()
                .w(px(180.0))
                .flex_shrink_0()
                .whitespace_nowrap()
                .text_color(theme::text_muted())
                .child(label),
        )
        .child(
            div()
                .flex_1()
                .text_color(theme::text_primary())
                .child(value),
        )
}

pub fn render(explorer: &Explorer, cx: &Context<Explorer>) -> Option<impl IntoElement> {
    let props = explorer.properties.as_ref()?;

    let icon: AnyElement = if let [path] = props.paths.as_slice() {
        let is_dir = if props.is_symlink {
            props.link_target_is_dir
        } else {
            props.is_dir
        };
        let fake = FsEntry {
            name: String::new(),
            path: path.clone(),
            is_dir,
            size: 0,
            modified: None,
            is_symlink: props.is_symlink,
            ..Default::default()
        };
        icon_theme::svg_icon_for_size(&fake, px(40.0), cx)
    } else if props.is_dir {
        icon_theme::directory_svg_icon_size(px(40.0), cx)
    } else {
        icon_theme::generic_file_svg_icon_size(px(40.0), cx)
    };

    let general_tab = props.tab == PropertiesTab::General;

    let (count_label, size_label, disk_label): (SharedString, SharedString, SharedString) =
        match &props.stats {
            StatsState::Loading => (
                "Calculating…".into(),
                "Calculating…".into(),
                "Calculating…".into(),
            ),
            StatsState::Ready(stats) => (
                stats.file_count.to_string().into(),
                format_size(stats.total_size).into(),
                format_size(stats.size_on_disk).into(),
            ),
        };

    let stats_rows = div()
        .flex()
        .flex_col()
        .gap_2()
        .when(props.is_dir, |col| {
            col.child(row("Total count of files:", count_label))
        })
        .child(row(
            if props.is_dir {
                "Total size of files:"
            } else {
                "Size:"
            },
            size_label,
        ))
        .child(row("Size on disk:", disk_label));

    let dates_rows = props.single.as_ref().map(|single| {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(row(
                "Last modification:",
                format_modified(single.modified).into(),
            ))
            .child(row("Last access:", format_modified(single.accessed).into()))
            .child(row(
                "Last permissions change:",
                format_modified(single.permissions_changed).into(),
            ))
    });

    let general_content = div()
        .flex()
        .flex_col()
        .gap_2()
        .child(row("Location:", props.location.clone()))
        .child(row("Type:", props.type_label.clone()))
        .when(props.is_symlink, |col| {
            col.child(row(
                "Target:",
                props
                    .link_target
                    .clone()
                    .unwrap_or_else(|| "(unresolved)".into()),
            ))
        })
        .child(stats_rows)
        .children(dates_rows);

    let permissions_content: AnyElement = match &props.single {
        Some(single) => div()
            .flex()
            .flex_col()
            .gap_2()
            .child(row("Owner:", single.uid.to_string().into()))
            .child(row("Group:", single.gid.to_string().into()))
            .child(row(
                "Permissions:",
                format!(
                    "{} ({})",
                    entry::permission_string(single.mode),
                    entry::octal_permissions(single.mode)
                )
                .into(),
            ))
            .into_any_element(),
        None => div()
            .text_color(theme::text_muted())
            .child("Select a single item to view its permissions.")
            .into_any_element(),
    };

    Some(
        div()
            .id("properties-backdrop")
            .occlude()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(Hsla {
                h: 0.,
                s: 0.,
                l: 0.,
                a: 0.5,
            })
            .child(
                div()
                    .id("properties-panel")
                    .track_focus(&props.focus_handle)
                    .on_mouse_down_out(cx.listener(|explorer, _, window, cx| {
                        explorer.close_properties(window, cx);
                    }))
                    .on_mouse_down(MouseButton::Left, |_, _window, cx| cx.stop_propagation())
                    .flex()
                    .flex_col()
                    .w(px(420.0))
                    .bg(theme::bg_panel())
                    .border_1()
                    .border_color(theme::border())
                    .shadow(vec![BoxShadow {
                        color: Hsla {
                            h: 0.,
                            s: 0.,
                            l: 0.,
                            a: 0.4,
                        },
                        blur_radius: px(16.0),
                        spread_radius: px(0.),
                        offset: Point::new(px(0.0), px(4.0)),
                    }])
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap_3()
                            .p_4()
                            .child(icon)
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(theme::text_primary())
                                    .child(props.name.clone()),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .border_b_1()
                            .border_color(theme::border())
                            .px_4()
                            .child(
                                div()
                                    .id("properties-tab-general")
                                    .cursor_pointer()
                                    .px_3()
                                    .py_2()
                                    .text_color(if general_tab {
                                        theme::text_primary()
                                    } else {
                                        theme::text_muted()
                                    })
                                    .when(general_tab, |el| {
                                        el.border_b_2().border_color(theme::box_select_border())
                                    })
                                    .on_click(cx.listener(|explorer, _, _window, cx| {
                                        explorer.set_properties_tab(PropertiesTab::General, cx);
                                    }))
                                    .child("General"),
                            )
                            .child(
                                div()
                                    .id("properties-tab-permissions")
                                    .cursor_pointer()
                                    .px_3()
                                    .py_2()
                                    .text_color(if !general_tab {
                                        theme::text_primary()
                                    } else {
                                        theme::text_muted()
                                    })
                                    .when(!general_tab, |el| {
                                        el.border_b_2().border_color(theme::box_select_border())
                                    })
                                    .on_click(cx.listener(|explorer, _, _window, cx| {
                                        explorer.set_properties_tab(PropertiesTab::Permissions, cx);
                                    }))
                                    .child("Permissions"),
                            ),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_3()
                            .p_4()
                            .h(px(260.0))
                            .overflow_hidden()
                            .when(general_tab, |el| el.child(general_content))
                            .when(!general_tab, |el| el.child(permissions_content)),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_end()
                            .gap_2()
                            .p_3()
                            .border_t_1()
                            .border_color(theme::border())
                            .child(
                                div()
                                    .id("properties-close")
                                    .cursor_pointer()
                                    .px_3()
                                    .py_1()
                                    .border_1()
                                    .border_color(theme::border())
                                    .text_color(theme::text_primary())
                                    .hover(|style| style.bg(theme::bg_hover()))
                                    .on_click(cx.listener(|explorer, _, window, cx| {
                                        explorer.close_properties(window, cx);
                                    }))
                                    .child("Close"),
                            ),
                    ),
            ),
    )
}
