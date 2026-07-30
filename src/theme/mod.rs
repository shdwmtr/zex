use gpui::{FontWeight, Global, Hsla, Pixels, Rgba, SharedString, px, rgb};

pub mod icon_theme;
pub mod zed_default_icons;

#[derive(Clone)]
pub struct UiFont {
    pub weight: FontWeight,
    pub font_family: SharedString,
    pub font_size: Pixels,
}

impl Default for UiFont {
    fn default() -> Self {
        Self {
            weight: FontWeight::NORMAL,
            font_family: ".SystemUIFont".into(),
            font_size: px(16.0),
        }
    }
}

impl Global for UiFont {}

pub fn bg_root() -> Rgba {
    rgb(0x1e1e1e)
}
pub fn bg_panel() -> Rgba {
    rgb(0x252525)
}
pub fn bg_bar() -> Rgba {
    rgb(0x2a2a2a)
}
pub fn bg_header() -> Rgba {
    rgb(0x202020)
}
pub fn bg_hover() -> Rgba {
    rgb(0x2d2d2d)
}
pub fn bg_selected() -> Rgba {
    rgb(0x3a5a8a)
}
pub fn bg_sidebar_hover() -> Rgba {
    rgb(0x33415a)
}
pub fn bg_sidebar_selected() -> Rgba {
    rgb(0x404040)
}
pub fn bg_breadcrumb_hover() -> Rgba {
    rgb(0x3a3a3a)
}
pub fn border() -> Rgba {
    rgb(0x3a3a3a)
}

pub fn text_primary() -> Rgba {
    rgb(0xe0e0e0)
}
pub fn text_muted() -> Rgba {
    rgb(0x999999)
}
pub fn text_faint() -> Rgba {
    rgb(0x777777)
}
pub fn text_error() -> Rgba {
    rgb(0xdd6666)
}
pub fn bg_error() -> Rgba {
    rgb(0x442222)
}

pub fn text_selection_fill() -> Hsla {
    Hsla {
        h: 0.58,
        s: 0.55,
        l: 0.55,
        a: 0.35,
    }
}

pub fn box_select_fill() -> Hsla {
    Hsla {
        h: 0.0,
        s: 0.0,
        l: 0.0,
        a: 0.12,
    }
}

pub fn box_select_border() -> Rgba {
    rgb(0x8ab4f8)
}

pub fn drop_target_fill() -> Hsla {
    Hsla {
        h: 0.333,
        s: 0.6,
        l: 0.4,
        a: 0.18,
    }
}

pub fn drop_target_border() -> Rgba {
    rgb(0x81c995)
}
