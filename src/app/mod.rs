pub mod assets;
mod window_root;

use gpui::{App, AppContext, Application, Bounds, WindowBounds, WindowOptions, px, size};

use crate::explorer::Explorer;
use crate::keys;
use crate::settings::Settings;
use crate::theme::{self, UiFont, color_theme, icon_theme};
use crate::ui;

use window_root::WindowRoot;

pub fn run(settings: Settings) {
    let icon_theme_state = icon_theme::resolve(&settings);
    let show_hidden_files = settings.show_hidden_files.unwrap_or(false);
    let sidebar_visible = settings.sidebar_visible.unwrap_or(true);
    let sidebar_entries = settings.sidebar.clone();
    let git_settings = settings.git.clone();
    let disk_usage_settings = settings.disk_usage.clone();

    Application::new()
        .with_assets(assets::Assets::new())
        .run(move |cx: &mut App| {
            keys::init(cx);
            ui::text_input::init(cx);
            ui::popup_menu::init(cx);
            ui::path_bar::init(cx);
            cx.set_global(icon_theme_state);
            theme::init(color_theme::resolve(&settings.theme, cx));

            let default_font = UiFont::default();
            cx.set_global(UiFont {
                weight: settings
                    .ui_font_weight
                    .map(Into::into)
                    .unwrap_or(default_font.weight),
                font_family: settings
                    .ui_font_family
                    .clone()
                    .map(Into::into)
                    .unwrap_or(default_font.font_family),
                font_size: settings
                    .ui_font_size
                    .map(px)
                    .unwrap_or(default_font.font_size),
            });

            let bounds = Bounds::centered(None, size(px(1000.0), px(650.0)), cx);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, cx| {
                    let explorer = cx.new(|cx| {
                        Explorer::new(
                            window,
                            cx,
                            show_hidden_files,
                            sidebar_visible,
                            sidebar_entries,
                            git_settings,
                            disk_usage_settings,
                        )
                    });
                    cx.new(|_| WindowRoot { content: explorer })
                },
            )
            .unwrap();
            cx.activate(true);
        });
}
