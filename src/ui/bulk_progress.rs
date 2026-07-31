use gpui::{
    AnyElement, BoxShadow, Context, FontWeight, Hsla, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Point, StatefulInteractiveElement, Styled, div, px, relative,
};

use crate::explorer::Explorer;
use crate::explorer::bulk_op::BulkDecision;
use crate::theme;

pub fn render(explorer: &Explorer, cx: &Context<Explorer>) -> Option<impl IntoElement> {
    let state = explorer.active_bulk_op.as_ref()?;
    let title = state.title.clone();
    let current = state.current.clone();
    let completed = state.completed;
    let total = state.total.max(1);
    let counter = format!("{completed} of {total}");
    let progress = (completed as f32 / total as f32).clamp(0.0, 1.0);
    let pending = state
        .pending_error
        .as_ref()
        .map(|pending| (pending.label.clone(), pending.message.clone()));

    let bottom_row: AnyElement = match pending {
        Some((label, message)) => bulk_error_row(label.to_string(), message, cx).into_any_element(),
        None => cancel_row(cx).into_any_element(),
    };

    Some(
        div()
            .id("bulk-progress-backdrop")
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
                    .id("bulk-progress-panel")
                    .on_mouse_down(MouseButton::Left, |_, _window, cx| cx.stop_propagation())
                    .flex()
                    .flex_col()
                    .gap_3()
                    .w(px(380.0))
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
                            .flex()
                            .flex_row()
                            .items_center()
                            .justify_between()
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .text_color(theme::text_primary())
                                    .child(title),
                            )
                            .child(div().text_color(theme::text_muted()).child(counter)),
                    )
                    .child(
                        div()
                            .text_color(theme::text_muted())
                            .truncate()
                            .child(current),
                    )
                    .child(
                        div()
                            .h(px(6.0))
                            .w_full()
                            .rounded_full()
                            .bg(theme::bg_hover())
                            .child(
                                div()
                                    .h_full()
                                    .rounded_full()
                                    .bg(theme::bg_selected())
                                    .w(relative(progress)),
                            ),
                    )
                    .child(bottom_row),
            ),
    )
}

fn cancel_row(cx: &Context<Explorer>) -> impl IntoElement {
    div().flex().flex_row().justify_end().child(
        div()
            .id("bulk-progress-cancel")
            .cursor_pointer()
            .px_3()
            .py_1()
            .border_1()
            .border_color(theme::border())
            .text_color(theme::text_primary())
            .hover(|style| style.bg(theme::bg_hover()))
            .on_click(cx.listener(|explorer, _, _window, cx| {
                explorer.request_bulk_cancel(cx);
            }))
            .child("Cancel"),
    )
}

fn bulk_error_row(label: String, message: String, cx: &Context<Explorer>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_2()
        .child(
            div()
                .text_color(theme::text_error())
                .child(format!("Couldn't process \"{label}\": {message}")),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .justify_end()
                .gap_2()
                .child(decision_button(
                    "bulk-error-skip",
                    "Skip",
                    BulkDecision::Skip,
                    cx,
                ))
                .child(decision_button(
                    "bulk-error-skip-all",
                    "Skip All",
                    BulkDecision::SkipAll,
                    cx,
                ))
                .child(decision_button(
                    "bulk-error-retry",
                    "Retry",
                    BulkDecision::Retry,
                    cx,
                ))
                .child(decision_button(
                    "bulk-error-cancel",
                    "Cancel",
                    BulkDecision::Cancel,
                    cx,
                )),
        )
}

fn decision_button(
    id: &'static str,
    label: &'static str,
    decision: BulkDecision,
    cx: &Context<Explorer>,
) -> impl IntoElement {
    div()
        .id(id)
        .cursor_pointer()
        .px_3()
        .py_1()
        .border_1()
        .border_color(theme::border())
        .text_color(theme::text_primary())
        .hover(|style| style.bg(theme::bg_hover()))
        .on_click(cx.listener(move |explorer, _, _window, cx| {
            explorer.resolve_bulk_error(decision, cx);
        }))
        .child(label)
}
