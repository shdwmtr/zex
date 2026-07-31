use std::rc::Rc;

use gpui::{
    AnyElement, App, AppContext, ClickEvent, Context, Corner, DismissEvent, Entity, EventEmitter,
    FocusHandle, InteractiveElement, IntoElement, KeyBinding, MouseButton, ParentElement, Pixels,
    Point, Render, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Subscription,
    Window, actions, anchored, deferred, div, prelude::FluentBuilder as _, px, svg,
};

use crate::theme;

actions!(zex_popup_menu, [Dismiss]);

pub fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("escape", Dismiss, Some("PopupMenu"))]);
}

type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
type MenuBuilder = Rc<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu>;

pub enum PopupMenuItem {
    Item {
        label: SharedString,
        icon: Option<SharedString>,
        checked: Option<bool>,
        disabled: bool,
        on_click: Option<ClickHandler>,
    },
    Separator,
}

impl PopupMenuItem {
    pub fn new(label: impl Into<SharedString>) -> Self {
        PopupMenuItem::Item {
            label: label.into(),
            icon: None,
            checked: None,
            disabled: false,
            on_click: None,
        }
    }

    pub fn icon(mut self, path: impl Into<SharedString>) -> Self {
        if let PopupMenuItem::Item { icon, .. } = &mut self {
            *icon = Some(path.into());
        }
        self
    }

    pub fn checked(mut self, checked: bool) -> Self {
        if let PopupMenuItem::Item { checked: slot, .. } = &mut self {
            *slot = Some(checked);
        }
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        if let PopupMenuItem::Item { disabled: slot, .. } = &mut self {
            *slot = disabled;
        }
        self
    }

    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        if let PopupMenuItem::Item { on_click, .. } = &mut self {
            *on_click = Some(Rc::new(handler));
        }
        self
    }
}

pub struct PopupMenu {
    items: Vec<PopupMenuItem>,
    focus_handle: FocusHandle,
}

impl PopupMenu {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        window.focus(&focus_handle);
        Self {
            items: Vec::new(),
            focus_handle,
        }
    }

    pub fn item(mut self, item: impl Into<PopupMenuItem>) -> Self {
        self.items.push(item.into());
        self
    }

    pub fn separator(mut self) -> Self {
        self.items.push(PopupMenuItem::Separator);
        self
    }
}

impl EventEmitter<DismissEvent> for PopupMenu {}

fn render_item(item: &PopupMenuItem, ix: usize, cx: &mut Context<PopupMenu>) -> AnyElement {
    match item {
        PopupMenuItem::Separator => div()
            .h(px(1.0))
            .my_1()
            .mx_2()
            .bg(theme::border())
            .into_any_element(),
        PopupMenuItem::Item {
            label,
            icon,
            checked,
            disabled,
            on_click,
        } => {
            let disabled = *disabled;
            let handler = on_click.clone();

            div()
                .id(ix)
                .flex()
                .flex_row()
                .items_center()
                .gap_2()
                .px_3()
                .py_1()
                .min_w(px(160.0))
                .when(disabled, |el| el.opacity(0.4))
                .when(!disabled, |el| {
                    el.cursor_pointer()
                        .hover(|style| style.bg(theme::bg_hover()))
                        .on_click(cx.listener(move |_menu, event: &ClickEvent, window, cx| {
                            if let Some(handler) = &handler {
                                handler(event, window, cx);
                            }
                            cx.emit(DismissEvent);
                        }))
                })
                .child(
                    div()
                        .flex()
                        .items_center()
                        .justify_center()
                        .w_4()
                        .h_4()
                        .children(icon.clone().map(|path| {
                            svg()
                                .path(path)
                                .size_4()
                                .flex_none()
                                .text_color(theme::text_muted())
                        })),
                )
                .child(div().flex_1().child(label.clone()))
                .when_some(*checked, |el, checked| {
                    el.child(
                        div()
                            .w_4()
                            .h_4()
                            .flex()
                            .items_center()
                            .justify_center()
                            .when(checked, |el| el.child("✓")),
                    )
                })
                .into_any_element()
        }
    }
}

impl Render for PopupMenu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("popup-menu")
            .key_context("PopupMenu")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|_, _: &Dismiss, _window, cx| cx.emit(DismissEvent)))
            .on_mouse_down_out(cx.listener(|_, _, _window, cx| cx.emit(DismissEvent)))
            .occlude()
            .flex()
            .flex_col()
            .py_1()
            .bg(theme::bg_elevated())
            .border_1()
            .border_color(theme::border())
            .shadow(vec![gpui::BoxShadow {
                color: gpui::Hsla {
                    h: 0.,
                    s: 0.,
                    l: 0.,
                    a: 0.3,
                },
                blur_radius: px(8.0),
                spread_radius: px(0.),
                offset: Point::new(px(0.0), px(2.0)),
            }])
            .children(
                self.items
                    .iter()
                    .enumerate()
                    .map(|(ix, item)| render_item(item, ix, cx))
                    .collect::<Vec<_>>(),
            )
    }
}

pub trait ContextMenuExt: ParentElement + Styled + InteractiveElement + Sized {
    fn context_menu(
        self,
        f: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
    ) -> ContextMenu<Self> {
        ContextMenu {
            child: self,
            builder: Rc::new(f),
        }
    }
}

impl<E: ParentElement + Styled + InteractiveElement> ContextMenuExt for E {}

#[derive(Default)]
struct ContextMenuState {
    open: bool,
    position: Point<Pixels>,
    menu: Option<Entity<PopupMenu>>,
    _subscription: Option<Subscription>,
}

pub struct ContextMenu<E> {
    child: E,
    builder: MenuBuilder,
}

impl<E> IntoElement for ContextMenu<E>
where
    E: ParentElement + Styled + InteractiveElement + IntoElement + 'static,
{
    type Element = gpui::Component<Self>;

    fn into_element(self) -> Self::Element {
        gpui::Component::new(self)
    }
}

impl<E: ParentElement> ParentElement for ContextMenu<E> {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.child.extend(elements);
    }
}

impl<E: Styled> Styled for ContextMenu<E> {
    fn style(&mut self) -> &mut gpui::StyleRefinement {
        self.child.style()
    }
}

impl<E: InteractiveElement> InteractiveElement for ContextMenu<E> {
    fn interactivity(&mut self) -> &mut gpui::Interactivity {
        self.child.interactivity()
    }
}

impl<E: InteractiveElement> StatefulInteractiveElement for ContextMenu<E> {}

impl<E> RenderOnce for ContextMenu<E>
where
    E: ParentElement + Styled + InteractiveElement + IntoElement + 'static,
{
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let state = window.use_state(cx, |_, _| ContextMenuState::default());
        let builder = self.builder;

        let overlay = {
            let open = state.read(cx).open;
            let menu = state.read(cx).menu.clone();
            let position = state.read(cx).position;
            open.then(|| {
                menu.map(|menu| {
                    deferred(
                        anchored()
                            .position(position)
                            .anchor(Corner::TopLeft)
                            .snap_to_window_with_margin(px(8.0))
                            .child(menu),
                    )
                    .with_priority(1)
                })
            })
            .flatten()
        };

        self.child
            .on_mouse_down(MouseButton::Right, move |event, window, cx| {
                cx.stop_propagation();
                let position = event.position;
                let menu_entity = cx.new(|cx| (builder)(PopupMenu::new(window, cx), window, cx));

                state.update(cx, |menu_state, cx| {
                    let subscription = cx.subscribe(
                        &menu_entity,
                        |menu_state: &mut ContextMenuState, _menu, _event: &DismissEvent, cx| {
                            menu_state.open = false;
                            cx.notify();
                        },
                    );
                    menu_state.open = true;
                    menu_state.position = position;
                    menu_state.menu = Some(menu_entity);
                    menu_state._subscription = Some(subscription);
                    cx.notify();
                });
            })
            .children(overlay)
    }
}
