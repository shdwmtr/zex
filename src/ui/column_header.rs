use gpui::{
    App, ClickEvent, Context, Div, DragMoveEvent, ElementId, Entity, InteractiveElement,
    IntoElement, MouseButton, MouseUpEvent, ParentElement, Pixels, Point, Render, Stateful,
    StatefulInteractiveElement, Styled, Transformation, Window, div, prelude::FluentBuilder as _,
    px, radians, svg,
};

use crate::explorer::columns::SortDirection;
use crate::theme;

pub const RESIZE_HIT_WIDTH: f32 = 9.0;

pub struct ResizeGhost;

impl Render for ResizeGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[allow(clippy::too_many_arguments)]
pub fn sort_cell(
    id: impl Into<ElementId>,
    group_name: impl Into<String>,
    label: &'static str,
    width: Option<f32>,
    min_width: f32,
    is_active: bool,
    direction: SortDirection,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Stateful<Div> {
    let rotation = match direction {
        SortDirection::Ascending => radians(-std::f32::consts::FRAC_PI_2),
        SortDirection::Descending => radians(std::f32::consts::FRAC_PI_2),
    };
    let group_name = group_name.into();

    div()
        .id(id)
        .group(group_name.clone())
        .h_full()
        .flex_shrink_0()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .gap_1()
        .pr_2()
        .cursor_pointer()
        .when(width.is_some(), |el| el.pl_2())
        .when_some(width, |el, w| el.w(px(w)))
        .when(width.is_none(), |el| el.flex_1().min_w(px(min_width)))
        .when(is_active, |el| el.text_color(theme::text_primary()))
        .child(div().truncate().child(label))
        .child(
            svg()
                .flex_shrink_0()
                .path("icons/chevron-right.svg")
                .size(px(10.0))
                .text_color(if is_active {
                    theme::text_primary()
                } else {
                    theme::text_muted()
                })
                .with_transformation(Transformation::rotate(rotation))
                .opacity(0.0)
                .group_hover(group_name, |style| style.opacity(1.0)),
        )
        .on_click(on_click)
}

pub fn resize_handle<T: Clone + 'static>(
    id: impl Into<ElementId>,
    value: T,
    on_drag_start: impl Fn(&T, Point<Pixels>, &mut Window, &mut App) -> Entity<ResizeGhost> + 'static,
    on_drag_move: impl Fn(&DragMoveEvent<T>, &mut Window, &mut App) + 'static,
    on_drag_end: impl Fn(&MouseUpEvent, &mut Window, &mut App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .absolute()
        .top_0()
        .bottom_0()
        .left(px(-RESIZE_HIT_WIDTH / 2.0))
        .w(px(RESIZE_HIT_WIDTH))
        .occlude()
        .cursor_col_resize()
        .on_drag(value, on_drag_start)
        .on_drag_move::<T>(on_drag_move)
        .on_mouse_up(MouseButton::Left, on_drag_end)
}

pub fn divider_overlay(
    height: Pixels,
    leading_min_width: f32,
    widths: impl IntoIterator<Item = f32>,
) -> impl IntoElement {
    fn divider(width: f32) -> Div {
        div()
            .w(px(width))
            .h_full()
            .flex_shrink_0()
            .border_l_1()
            .border_color(theme::border())
    }

    div()
        .absolute()
        .top_0()
        .left_0()
        .right_0()
        .h(height)
        .flex()
        .flex_row()
        .px_3()
        .child(div().flex_1().min_w(px(leading_min_width)))
        .children(widths.into_iter().map(divider))
}
