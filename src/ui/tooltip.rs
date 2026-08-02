use gpui::{
    AnyView, App, AppContext, Context, IntoElement, ParentElement, Render, SharedString, Styled,
    Window, div,
};

use crate::theme;
use crate::theme::UiFont;

pub struct Tooltip {
    text: SharedString,
}

impl Tooltip {
    pub fn text(text: impl Into<SharedString>) -> Self {
        Self { text: text.into() }
    }

    pub fn build(
        text: impl Into<SharedString> + Clone + 'static,
    ) -> impl Fn(&mut Window, &mut App) -> AnyView + Clone {
        move |_window, cx| cx.new(|_| Tooltip::text(text.clone())).into()
    }
}

impl Render for Tooltip {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let font_family = cx.global::<UiFont>().font_family.clone();

        div()
            .px_2()
            .py_1()
            .font_family(font_family)
            .bg(theme::bg_elevated())
            .border_1()
            .border_color(theme::border_variant())
            .rounded_lg()
            .shadow_lg()
            .text_base()
            .text_color(theme::text_primary())
            .child(self.text.clone())
    }
}
