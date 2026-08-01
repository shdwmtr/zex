use gpui::{Context, Entity, IntoElement, ParentElement, Render, Styled, Window, div};

use crate::theme::{UI_FONT_SCALE, UiFont};
use crate::ui;
use crate::workspace::Workspace;

pub struct WindowRoot {
    pub content: Entity<Workspace>,
}

impl Render for WindowRoot {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let font = cx.global::<UiFont>();
        window.set_rem_size(font.font_size * UI_FONT_SCALE);
        let font_family = font.font_family.clone();

        ui::window_chrome::window_chrome().child(
            div()
                .size_full()
                .font_family(font_family)
                .child(self.content.clone()),
        )
    }
}
