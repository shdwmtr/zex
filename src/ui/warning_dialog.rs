use gpui::{
    BoxShadow, Context, FontWeight, Hsla, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Point, StatefulInteractiveElement, Styled, div, px,
};

use crate::explorer::Explorer;
use crate::theme;

pub fn render(explorer: &Explorer, cx: &Context<Explorer>) -> Option<impl IntoElement> {
    let warning = explorer.warning.as_ref()?;
    let title = warning.title.clone();
    let message = warning.message.clone();
    let confirm_label = warning.confirm_label.clone();

    Some(
        div()
            .id("warning-backdrop")
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
                    .id("warning-panel")
                    .on_mouse_down_out(cx.listener(|explorer, _, _window, cx| {
                        explorer.dismiss_warning(cx);
                    }))
                    .on_mouse_down(MouseButton::Left, |_, _window, cx| cx.stop_propagation())
                    .flex()
                    .flex_col()
                    .gap_3()
                    .w(px(340.0))
                    .p_4()
                    .bg(theme::bg_elevated())
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
                            .font_weight(FontWeight::BOLD)
                            .text_color(theme::text_primary())
                            .child(title),
                    )
                    .child(div().text_color(theme::text_muted()).child(message))
                    .child(
                        div()
                            .flex()
                            .flex_row()
                            .justify_end()
                            .gap_2()
                            .child(
                                div()
                                    .id("warning-cancel")
                                    .cursor_pointer()
                                    .px_3()
                                    .py_1()
                                    .border_1()
                                    .border_color(theme::border())
                                    .text_color(theme::text_primary())
                                    .hover(|style| style.bg(theme::bg_hover()))
                                    .on_click(cx.listener(|explorer, _, _window, cx| {
                                        explorer.dismiss_warning(cx);
                                    }))
                                    .child("Cancel"),
                            )
                            .child(
                                div()
                                    .id("warning-confirm")
                                    .cursor_pointer()
                                    .px_3()
                                    .py_1()
                                    .bg(theme::bg_error())
                                    .text_color(theme::text_error())
                                    .hover(|style| style.opacity(0.85))
                                    .on_click(cx.listener(|explorer, _, _window, cx| {
                                        explorer.confirm_warning(cx);
                                    }))
                                    .child(confirm_label),
                            ),
                    ),
            ),
    )
}
