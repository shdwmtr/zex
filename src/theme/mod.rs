use std::sync::OnceLock;

use gpui::{FontWeight, Global, Hsla, Pixels, Rgba, SharedString, px, rgb};

pub mod color_theme;
pub mod icon_theme;
pub mod zed_default_icons;

#[derive(Clone, Copy, Debug)]
pub struct ColorTheme {
    pub bg_root: Rgba,
    pub bg_panel: Rgba,
    pub bg_bar: Rgba,
    pub bg_header: Rgba,
    pub bg_hover: Rgba,
    pub bg_selected: Rgba,
    pub bg_sidebar_hover: Rgba,
    pub bg_sidebar_selected: Rgba,
    pub bg_breadcrumb_hover: Rgba,
    pub border: Rgba,
    pub text_primary: Rgba,
    pub text_muted: Rgba,
    pub text_faint: Rgba,
    pub text_error: Rgba,
    pub bg_error: Rgba,
    pub drop_target_fill: Hsla,
    pub drop_target_border: Rgba,
    pub git_color_modified: Rgba,
    pub git_color_added: Rgba,
    pub git_color_deleted: Rgba,
    pub git_color_renamed: Rgba,
    pub git_color_untracked: Rgba,
    pub git_color_ignored: Rgba,
    pub git_color_conflicted: Rgba,
}

impl ColorTheme {
    pub fn zex_default() -> Self {
        Self {
            bg_root: rgb(0x1e1e1e),
            bg_panel: rgb(0x252525),
            bg_bar: rgb(0x2a2a2a),
            bg_header: rgb(0x202020),
            bg_hover: rgb(0x2d2d2d),
            bg_selected: rgb(0x3a5a8a),
            bg_sidebar_hover: rgb(0x33415a),
            bg_sidebar_selected: rgb(0x404040),
            bg_breadcrumb_hover: rgb(0x3a3a3a),
            border: rgb(0x3a3a3a),
            text_primary: rgb(0xe0e0e0),
            text_muted: rgb(0x999999),
            text_faint: rgb(0x777777),
            text_error: rgb(0xdd6666),
            bg_error: rgb(0x442222),
            drop_target_fill: Hsla {
                h: 0.333,
                s: 0.6,
                l: 0.4,
                a: 0.18,
            },
            drop_target_border: rgb(0x81c995),
            git_color_modified: rgb(0xe2c08d),
            git_color_added: rgb(0x73c991),
            git_color_deleted: rgb(0xf14c4c),
            git_color_renamed: rgb(0x73c991),
            git_color_untracked: rgb(0x73c991),
            git_color_ignored: rgb(0x777777),
            git_color_conflicted: rgb(0xf14c4c),
        }
    }
}

static ACTIVE_THEME: OnceLock<ColorTheme> = OnceLock::new();

pub fn init(theme: ColorTheme) {
    let _ = ACTIVE_THEME.set(theme);
}

fn active() -> ColorTheme {
    ACTIVE_THEME.get().copied().unwrap_or_else(ColorTheme::zex_default)
}

pub const UI_FONT_SCALE: f32 = 0.875;

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
    active().bg_root
}
pub fn bg_panel() -> Rgba {
    active().bg_panel
}
pub fn bg_bar() -> Rgba {
    active().bg_bar
}
pub fn bg_header() -> Rgba {
    active().bg_header
}
pub fn bg_hover() -> Rgba {
    active().bg_hover
}
pub fn bg_selected() -> Rgba {
    active().bg_selected
}
pub fn bg_sidebar_hover() -> Rgba {
    active().bg_sidebar_hover
}
pub fn bg_sidebar_selected() -> Rgba {
    active().bg_sidebar_selected
}
pub fn bg_breadcrumb_hover() -> Rgba {
    active().bg_breadcrumb_hover
}
pub fn border() -> Rgba {
    active().border
}

pub fn text_primary() -> Rgba {
    active().text_primary
}
pub fn text_muted() -> Rgba {
    active().text_muted
}
pub fn text_faint() -> Rgba {
    active().text_faint
}
pub fn text_error() -> Rgba {
    active().text_error
}
pub fn bg_error() -> Rgba {
    active().bg_error
}
pub fn text_on_error() -> Rgba {
    rgb(0xffffff)
}
pub fn bg_on_error() -> Rgba {
    rgb(0x4a1717)
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
        h: 0.6,
        s: 0.75,
        l: 0.65,
        a: 0.15,
    }
}

pub fn box_select_border() -> Rgba {
    rgb(0x8ab4f8)
}

pub fn drop_target_fill() -> Hsla {
    active().drop_target_fill
}

pub fn drop_target_border() -> Rgba {
    active().drop_target_border
}

pub fn git_color_modified() -> Rgba {
    active().git_color_modified
}
pub fn git_color_added() -> Rgba {
    active().git_color_added
}
pub fn git_color_deleted() -> Rgba {
    active().git_color_deleted
}
pub fn git_color_renamed() -> Rgba {
    active().git_color_renamed
}
pub fn git_color_untracked() -> Rgba {
    active().git_color_untracked
}
pub fn git_color_ignored() -> Rgba {
    active().git_color_ignored
}
pub fn git_color_conflicted() -> Rgba {
    active().git_color_conflicted
}
