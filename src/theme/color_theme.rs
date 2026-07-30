use std::path::{Path, PathBuf};

use gpui::{App, Rgba};
use serde::Deserialize;

use crate::settings::{ThemeMode, ThemeSettings};

use super::ColorTheme;

#[derive(Deserialize)]
struct ThemeFamilyManifest {
    themes: Vec<ThemeEntryContent>,
}

#[derive(Deserialize)]
struct ThemeEntryContent {
    name: String,
    #[serde(default)]
    style: ThemeStyleContent,
}

#[derive(Deserialize, Default)]
struct ThemeStyleContent {
    #[serde(default)]
    background: Option<Rgba>,
    #[serde(default, rename = "panel.background")]
    panel_background: Option<Rgba>,
    #[serde(default, rename = "status_bar.background")]
    status_bar_background: Option<Rgba>,
    #[serde(default, rename = "toolbar.background")]
    toolbar_background: Option<Rgba>,
    #[serde(default, rename = "element.hover")]
    element_hover: Option<Rgba>,
    #[serde(default, rename = "element.selected")]
    element_selected: Option<Rgba>,
    #[serde(default, rename = "ghost_element.hover")]
    ghost_element_hover: Option<Rgba>,
    #[serde(default)]
    border: Option<Rgba>,
    #[serde(default, rename = "border.focused")]
    border_focused: Option<Rgba>,
    #[serde(default)]
    text: Option<Rgba>,
    #[serde(default, rename = "text.muted")]
    text_muted: Option<Rgba>,
    #[serde(default, rename = "text.placeholder")]
    text_placeholder: Option<Rgba>,
    #[serde(default, rename = "text.disabled")]
    text_disabled: Option<Rgba>,
    #[serde(default)]
    error: Option<Rgba>,
    #[serde(default, rename = "error.background")]
    error_background: Option<Rgba>,
    #[serde(default, rename = "drop_target.background")]
    drop_target_background: Option<Rgba>,
    #[serde(default)]
    modified: Option<Rgba>,
    #[serde(default)]
    deleted: Option<Rgba>,
    #[serde(default)]
    renamed: Option<Rgba>,
    #[serde(default)]
    conflict: Option<Rgba>,
    #[serde(default)]
    ignored: Option<Rgba>,
    #[serde(default)]
    created: Option<Rgba>,
}

fn parse_manifest(contents: &str) -> Vec<(String, ThemeStyleContent)> {
    let Ok(manifest) = serde_json::from_str::<ThemeFamilyManifest>(contents) else {
        return Vec::new();
    };
    manifest
        .themes
        .into_iter()
        .map(|entry| (entry.name, entry.style))
        .collect()
}

#[derive(Deserialize, Default)]
struct ExtensionManifest {
    #[serde(default)]
    themes: Vec<String>,
}

fn zed_extensions_dir() -> PathBuf {
    let data_home = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/"));
            home.join(".local/share")
        });
    data_home.join("zed/extensions/installed")
}

fn discover_zed_extension_themes() -> Vec<(String, ThemeStyleContent)> {
    discover_zed_extension_themes_in(&zed_extensions_dir())
}

fn discover_zed_extension_themes_in(extensions_dir: &Path) -> Vec<(String, ThemeStyleContent)> {
    let Ok(entries) = std::fs::read_dir(extensions_dir) else {
        return Vec::new();
    };

    let mut themes = Vec::new();

    for entry in entries.flatten() {
        let ext_root = entry.path();
        if !ext_root.is_dir() {
            continue;
        }

        let Ok(manifest_toml) = std::fs::read_to_string(ext_root.join("extension.toml")) else {
            continue;
        };
        let Ok(manifest) = toml::from_str::<ExtensionManifest>(&manifest_toml) else {
            continue;
        };

        for theme_rel_path in &manifest.themes {
            let Ok(json_contents) = std::fs::read_to_string(ext_root.join(theme_rel_path)) else {
                continue;
            };
            themes.extend(parse_manifest(&json_contents));
        }
    }

    themes
}

fn find_zed_extension_theme(name: &str) -> Option<ThemeStyleContent> {
    discover_zed_extension_themes()
        .into_iter()
        .find(|(theme_name, _)| theme_name == name)
        .map(|(_, style)| style)
}

fn load_bundled_default() -> ColorTheme {
    let manifest_path = crate::app::assets::assets_dir().join("themes/zex-default.json");
    let contents = std::fs::read_to_string(&manifest_path)
        .expect("zex's bundled color theme manifest must be present");

    let style = parse_manifest(&contents)
        .into_iter()
        .next()
        .expect("zex's bundled color theme manifest must define at least one theme")
        .1;

    ColorTheme {
        bg_root: style.background.expect("bundled theme must set background"),
        bg_panel: style
            .panel_background
            .expect("bundled theme must set panel.background"),
        bg_bar: style
            .status_bar_background
            .expect("bundled theme must set status_bar.background"),
        bg_header: style
            .toolbar_background
            .expect("bundled theme must set toolbar.background"),
        bg_hover: style
            .element_hover
            .expect("bundled theme must set element.hover"),
        bg_selected: style
            .element_selected
            .expect("bundled theme must set element.selected"),
        bg_sidebar_hover: style
            .element_hover
            .expect("bundled theme must set element.hover"),
        bg_sidebar_selected: style
            .element_selected
            .expect("bundled theme must set element.selected"),
        bg_breadcrumb_hover: style
            .ghost_element_hover
            .expect("bundled theme must set ghost_element.hover"),
        border: style.border.expect("bundled theme must set border"),
        text_primary: style.text.expect("bundled theme must set text"),
        text_muted: style
            .text_muted
            .expect("bundled theme must set text.muted"),
        text_faint: style
            .text_placeholder
            .expect("bundled theme must set text.placeholder"),
        text_error: style.error.expect("bundled theme must set error"),
        bg_error: style
            .error_background
            .expect("bundled theme must set error.background"),
        drop_target_fill: style
            .drop_target_background
            .expect("bundled theme must set drop_target.background")
            .into(),
        drop_target_border: style
            .border_focused
            .expect("bundled theme must set border.focused"),
        git_color_modified: style.modified.expect("bundled theme must set modified"),
        git_color_added: style.created.expect("bundled theme must set created"),
        git_color_deleted: style.deleted.expect("bundled theme must set deleted"),
        git_color_renamed: style.renamed.expect("bundled theme must set renamed"),
        git_color_untracked: style.created.expect("bundled theme must set created"),
        git_color_ignored: style.ignored.expect("bundled theme must set ignored"),
        git_color_conflicted: style.conflict.expect("bundled theme must set conflict"),
    }
}

fn merge(style: &ThemeStyleContent, fallback: &ColorTheme) -> ColorTheme {
    ColorTheme {
        bg_root: style.background.unwrap_or(fallback.bg_root),
        bg_panel: style.panel_background.unwrap_or(fallback.bg_panel),
        bg_bar: style.status_bar_background.unwrap_or(fallback.bg_bar),
        bg_header: style.toolbar_background.unwrap_or(fallback.bg_header),
        bg_hover: style.element_hover.unwrap_or(fallback.bg_hover),
        bg_selected: style.element_selected.unwrap_or(fallback.bg_selected),
        bg_sidebar_hover: style.element_hover.unwrap_or(fallback.bg_sidebar_hover),
        bg_sidebar_selected: style
            .element_selected
            .unwrap_or(fallback.bg_sidebar_selected),
        bg_breadcrumb_hover: style
            .ghost_element_hover
            .unwrap_or(fallback.bg_breadcrumb_hover),
        border: style.border.unwrap_or(fallback.border),
        text_primary: style.text.unwrap_or(fallback.text_primary),
        text_muted: style.text_muted.unwrap_or(fallback.text_muted),
        text_faint: style
            .text_placeholder
            .or(style.text_disabled)
            .unwrap_or(fallback.text_faint),
        text_error: style.error.unwrap_or(fallback.text_error),
        bg_error: style.error_background.unwrap_or(fallback.bg_error),
        drop_target_fill: style
            .drop_target_background
            .map(Into::into)
            .unwrap_or(fallback.drop_target_fill),
        drop_target_border: style.border_focused.unwrap_or(fallback.drop_target_border),
        git_color_modified: style.modified.unwrap_or(fallback.git_color_modified),
        git_color_added: style.created.unwrap_or(fallback.git_color_added),
        git_color_deleted: style.deleted.unwrap_or(fallback.git_color_deleted),
        git_color_renamed: style.renamed.unwrap_or(fallback.git_color_renamed),
        git_color_untracked: style.created.unwrap_or(fallback.git_color_untracked),
        git_color_ignored: style.ignored.unwrap_or(fallback.git_color_ignored),
        git_color_conflicted: style.conflict.unwrap_or(fallback.git_color_conflicted),
    }
}

fn selected_theme_name<'a>(settings: &'a ThemeSettings, cx: &App) -> Option<&'a str> {
    match settings.mode {
        ThemeMode::Dark => settings.dark.as_deref(),
        ThemeMode::Light => settings.light.as_deref(),
        ThemeMode::System => {
            let is_light = matches!(
                cx.window_appearance(),
                gpui::WindowAppearance::Light | gpui::WindowAppearance::VibrantLight
            );
            if is_light {
                settings.light.as_deref()
            } else {
                settings.dark.as_deref()
            }
        }
    }
}

pub fn resolve(settings: &ThemeSettings, cx: &App) -> ColorTheme {
    let fallback = load_bundled_default();

    let Some(name) = selected_theme_name(settings, cx) else {
        return fallback;
    };

    match find_zed_extension_theme(name) {
        Some(style) => merge(&style, &fallback),
        None => fallback,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_manifest_and_resolves_hex_colors() {
        let json = r##"{
            "themes": [{
                "name": "Test Theme",
                "appearance": "dark",
                "style": {
                    "background": "#101010",
                    "text": "#e0e0e0",
                    "drop_target.background": "#12345678"
                }
            }]
        }"##;

        let themes = parse_manifest(json);

        assert_eq!(themes.len(), 1);
        assert_eq!(themes[0].0, "Test Theme");
        let style = &themes[0].1;
        assert_eq!(style.background, Rgba::try_from("#101010").ok());
        assert_eq!(style.text, Rgba::try_from("#e0e0e0").ok());
        assert_eq!(
            style.drop_target_background,
            Rgba::try_from("#12345678").ok()
        );
    }

    #[test]
    fn missing_and_null_keys_stay_none() {
        let json = r##"{
            "themes": [{
                "name": "Sparse",
                "appearance": "dark",
                "style": { "background": "#101010", "border": null }
            }]
        }"##;

        let themes = parse_manifest(json);
        let style = &themes[0].1;

        assert!(style.border.is_none());
        assert!(style.text.is_none());
    }

    #[test]
    fn merge_falls_back_to_bundled_default_for_missing_keys() {
        let fallback = load_bundled_default();
        let json = r##"{
            "themes": [{
                "name": "Partial",
                "appearance": "dark",
                "style": { "text": "#ffffff" }
            }]
        }"##;
        let style = &parse_manifest(json)[0].1;

        let merged = merge(style, &fallback);

        assert_eq!(merged.text_primary, Rgba::try_from("#ffffff").unwrap());
        assert_eq!(merged.bg_root, fallback.bg_root);
        assert_eq!(merged.git_color_modified, fallback.git_color_modified);
    }

    #[test]
    fn discovers_theme_from_synthetic_extension_directory() {
        let root =
            std::env::temp_dir().join(format!("zex_color_theme_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let ext_dir = root.join("fake-theme-extension");
        std::fs::create_dir_all(ext_dir.join("themes")).unwrap();

        std::fs::write(
            ext_dir.join("extension.toml"),
            "id = \"fake-theme-extension\"\nthemes = [\"themes/fake.json\"]\n",
        )
        .unwrap();
        std::fs::write(
            ext_dir.join("themes/fake.json"),
            r##"{
                "themes": [{
                    "name": "Fake Theme",
                    "appearance": "dark",
                    "style": { "background": "#abcdef" }
                }]
            }"##,
        )
        .unwrap();

        let themes = discover_zed_extension_themes_in(&root);

        assert_eq!(themes.len(), 1);
        assert_eq!(themes[0].0, "Fake Theme");
        assert_eq!(themes[0].1.background, Rgba::try_from("#abcdef").ok());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn skips_extensions_with_no_theme_or_malformed_manifest() {
        let root = std::env::temp_dir()
            .join(format!("zex_color_theme_test_skip_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);

        let plain_ext = root.join("plain-extension");
        std::fs::create_dir_all(&plain_ext).unwrap();
        std::fs::write(plain_ext.join("extension.toml"), "id = \"plain\"\n").unwrap();

        let broken_ext = root.join("broken-extension");
        std::fs::create_dir_all(&broken_ext).unwrap();
        std::fs::write(
            broken_ext.join("extension.toml"),
            "id = \"broken\"\nthemes = [\"themes/missing.json\"]\n",
        )
        .unwrap();

        let themes = discover_zed_extension_themes_in(&root);

        assert!(themes.is_empty());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn loads_bundled_default_theme_from_real_assets() {
        let theme = load_bundled_default();

        assert_eq!(theme.bg_root, Rgba::try_from("#1e1e1e").unwrap());
        assert_eq!(theme.text_primary, Rgba::try_from("#e0e0e0").unwrap());
    }
}
