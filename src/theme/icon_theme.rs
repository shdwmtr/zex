use std::collections::HashMap;
use std::path::{Path, PathBuf};

use gpui::{
    AnyElement, App, DevicePixels, Global, IntoElement, Pixels, SharedString, Size, SvgSize, img,
    prelude::*, px, svg,
};
use serde::Deserialize;

use crate::filesystem::entry::FsEntry;

#[derive(Deserialize)]
struct IconThemeManifest {
    themes: Vec<ThemeEntry>,
}

#[derive(Deserialize)]
struct ThemeEntry {
    name: String,
    #[serde(default)]
    directory_icons: Option<DirectoryIconsDef>,
    #[serde(default)]
    file_stems: HashMap<String, String>,
    #[serde(default)]
    file_suffixes: HashMap<String, String>,
    #[serde(default)]
    file_icons: HashMap<String, IconDef>,
}

#[derive(Deserialize)]
struct DirectoryIconsDef {
    collapsed: Option<String>,
    expanded: Option<String>,
}

#[derive(Deserialize)]
struct IconDef {
    path: String,
}

#[derive(Clone, Debug)]
pub struct LoadedIconTheme {
    pub name: String,
    pub directory_icon: Option<PathBuf>,
    pub file_stems: HashMap<String, String>,
    pub file_suffixes: HashMap<String, String>,
    pub file_icons: HashMap<String, PathBuf>,
}

pub struct IconThemeState {
    pub primary: Option<LoadedIconTheme>,
    pub fallback: LoadedIconTheme,
}

impl Global for IconThemeState {}

fn resolve_path(base_dir: &Path, relative: &str) -> PathBuf {
    let relative = relative.strip_prefix("./").unwrap_or(relative);
    base_dir.join(relative)
}

fn load_theme_entry(entry: ThemeEntry, base_dir: &Path) -> LoadedIconTheme {
    let directory_icon = entry
        .directory_icons
        .as_ref()
        .and_then(|dirs| dirs.collapsed.as_ref().or(dirs.expanded.as_ref()))
        .map(|path| resolve_path(base_dir, path));

    let file_icons = entry
        .file_icons
        .into_iter()
        .map(|(key, def)| (key, resolve_path(base_dir, &def.path)))
        .collect();

    let mut file_stems = super::zed_default_icons::file_stems();
    file_stems.extend(entry.file_stems);

    let mut file_suffixes = super::zed_default_icons::file_suffixes();
    file_suffixes.extend(entry.file_suffixes);

    LoadedIconTheme {
        name: entry.name,
        directory_icon,
        file_stems,
        file_suffixes,
        file_icons,
    }
}

pub fn parse_manifest(contents: &str, base_dir: &Path) -> Vec<LoadedIconTheme> {
    let Ok(manifest) = serde_json::from_str::<IconThemeManifest>(contents) else {
        return Vec::new();
    };
    manifest
        .themes
        .into_iter()
        .map(|entry| load_theme_entry(entry, base_dir))
        .collect()
}

#[derive(Deserialize, Default)]
struct ExtensionManifest {
    #[serde(default)]
    icon_themes: Vec<String>,
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

pub fn discover_zed_extension_themes() -> Vec<LoadedIconTheme> {
    discover_zed_extension_themes_in(&zed_extensions_dir())
}

fn discover_zed_extension_themes_in(extensions_dir: &Path) -> Vec<LoadedIconTheme> {
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

        for icon_theme_rel_path in &manifest.icon_themes {
            let Ok(json_contents) = std::fs::read_to_string(ext_root.join(icon_theme_rel_path))
            else {
                continue;
            };
            themes.extend(parse_manifest(&json_contents, &ext_root));
        }
    }

    themes
}

pub fn find_zed_extension_theme(name: &str) -> Option<LoadedIconTheme> {
    discover_zed_extension_themes()
        .into_iter()
        .find(|theme| theme.name == name)
}

pub fn resolve(settings: &crate::settings::Settings) -> IconThemeState {
    let fallback = load_bundled_default();
    let primary = settings
        .icon_theme
        .as_deref()
        .and_then(find_zed_extension_theme);
    IconThemeState { primary, fallback }
}

pub fn load_bundled_default() -> LoadedIconTheme {
    let assets_dir = crate::app::assets::assets_dir();
    let manifest_path = assets_dir.join("icon_themes/zex-default.json");
    let contents = std::fs::read_to_string(&manifest_path)
        .expect("zex's bundled icon theme manifest must be present");

    parse_manifest(&contents, &assets_dir)
        .into_iter()
        .next()
        .expect("zex's bundled icon theme manifest must define at least one theme")
}

fn resolve_key(theme: &LoadedIconTheme, key: &str) -> Option<PathBuf> {
    theme
        .file_stems
        .get(key)
        .or_else(|| theme.file_suffixes.get(key))
        .and_then(|type_key| theme.file_icons.get(type_key))
        .cloned()
}

fn multiple_extensions(path: &Path) -> Option<String> {
    let file_name = path.file_name()?.to_str()?;
    let parts: Vec<&str> = file_name.split('.').skip(1).collect();
    if parts.len() < 2 {
        return None;
    }
    Some(parts.join("."))
}

fn extension_or_hidden_file_name(path: &Path) -> Option<&str> {
    let file_name = path.file_name()?.to_str()?;
    if let Some(stripped) = file_name.strip_prefix('.') {
        return Some(stripped);
    }
    path.extension()
        .and_then(|ext| ext.to_str())
        .or_else(|| path.file_stem()?.to_str())
}

fn resolve_in(theme: &LoadedIconTheme, path: &Path) -> Option<PathBuf> {
    if let Some(mut rest) = path.file_name().and_then(|name| name.to_str()) {
        if let Some(icon) = resolve_key(theme, rest) {
            return Some(icon);
        }
        while let Some((_, suffix)) = rest.split_once('.') {
            if let Some(icon) = resolve_key(theme, suffix) {
                return Some(icon);
            }
            rest = suffix;
        }
    }

    if let Some(suffix) = multiple_extensions(path)
        && let Some(icon) = resolve_key(theme, &suffix)
    {
        return Some(icon);
    }

    if let Some(suffix) = extension_or_hidden_file_name(path)
        && let Some(icon) = resolve_key(theme, suffix)
    {
        return Some(icon);
    }

    if let Some(ext) = path.extension().and_then(|ext| ext.to_str())
        && let Some(icon) = resolve_key(theme, ext)
    {
        return Some(icon);
    }

    None
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IconRenderMode {
    Tinted,
    FullColor,
}

#[derive(Clone, Debug)]
pub struct ResolvedIcon {
    pub path: PathBuf,
    pub mode: IconRenderMode,
}

impl IconThemeState {
    pub fn directory_icon(&self) -> ResolvedIcon {
        if let Some(theme) = &self.primary
            && let Some(path) = theme.directory_icon.clone()
        {
            return ResolvedIcon {
                path,
                mode: IconRenderMode::FullColor,
            };
        }
        ResolvedIcon {
            path: self
                .fallback
                .directory_icon
                .clone()
                .expect("zex's bundled theme always defines a directory icon"),
            mode: IconRenderMode::Tinted,
        }
    }

    pub fn generic_file_icon(&self) -> ResolvedIcon {
        if let Some(theme) = &self.primary
            && let Some(path) = theme.file_icons.get("file").cloned()
        {
            return ResolvedIcon {
                path,
                mode: IconRenderMode::FullColor,
            };
        }
        ResolvedIcon {
            path: self
                .fallback
                .file_icons
                .get("file")
                .cloned()
                .expect("zex's bundled theme must define a \"file\" fallback icon"),
            mode: IconRenderMode::Tinted,
        }
    }

    pub fn icon_for(&self, entry: &FsEntry) -> ResolvedIcon {
        if entry.is_dir {
            return self.directory_icon();
        }

        if let Some(theme) = &self.primary
            && let Some(path) = resolve_in(theme, &entry.path)
        {
            return ResolvedIcon {
                path,
                mode: IconRenderMode::FullColor,
            };
        }

        if let Some(path) = resolve_in(&self.fallback, &entry.path) {
            return ResolvedIcon {
                path,
                mode: IconRenderMode::Tinted,
            };
        }

        self.generic_file_icon()
    }
}

fn render_icon(icon: ResolvedIcon, size: Pixels) -> AnyElement {
    match icon.mode {
        IconRenderMode::Tinted => svg()
            .path(icon.path.to_string_lossy().into_owned())
            .size(size)
            .flex_shrink_0()
            .text_color(super::text_primary())
            .into_any_element(),
        IconRenderMode::FullColor => img(icon.path).size(size).flex_shrink_0().into_any_element(),
    }
}

pub fn rasterize_drag_icon(
    icon: &ResolvedIcon,
    target_size: DevicePixels,
    cx: &App,
) -> Option<(Size<DevicePixels>, Vec<u8>)> {
    eprintln!(
        "[drag-icon] mode={:?} path={:?} target_size={:?}",
        icon.mode, icon.path, target_size
    );
    match icon.mode {
        IconRenderMode::Tinted => {
            let bytes = match cx.asset_source().load(&icon.path.to_string_lossy()) {
                Ok(Some(bytes)) => bytes,
                Ok(None) => {
                    eprintln!(
                        "[drag-icon] asset_source().load returned None for {:?}",
                        icon.path
                    );
                    return None;
                }
                Err(err) => {
                    eprintln!(
                        "[drag-icon] asset_source().load failed for {:?}: {err:?}",
                        icon.path
                    );
                    return None;
                }
            };
            let pixmap = match cx
                .svg_renderer()
                .render_pixmap(&bytes, SvgSize::Size(Size::new(target_size, target_size)))
            {
                Ok(pixmap) => pixmap,
                Err(err) => {
                    eprintln!(
                        "[drag-icon] render_pixmap failed for {:?}: {err:?}",
                        icon.path
                    );
                    return None;
                }
            };

            let tint = super::text_primary();
            let mut out = Vec::with_capacity(pixmap.data().len());
            for chunk in pixmap.data().chunks_exact(4) {
                let coverage = chunk[3] as f32 / 255.0;
                let alpha = coverage * tint.a;
                out.push((tint.r * alpha * 255.0).round() as u8);
                out.push((tint.g * alpha * 255.0).round() as u8);
                out.push((tint.b * alpha * 255.0).round() as u8);
                out.push((alpha * 255.0).round() as u8);
            }

            Some((
                Size::new(
                    DevicePixels(pixmap.width() as i32),
                    DevicePixels(pixmap.height() as i32),
                ),
                out,
            ))
        }
        IconRenderMode::FullColor => {
            let bytes = match std::fs::read(&icon.path) {
                Ok(bytes) => bytes,
                Err(err) => {
                    eprintln!("[drag-icon] fs::read failed for {:?}: {err:?}", icon.path);
                    return None;
                }
            };

            match image::load_from_memory(&bytes) {
                Ok(image) => {
                    let image = image
                        .resize_exact(
                            target_size.0 as u32,
                            target_size.0 as u32,
                            image::imageops::FilterType::Triangle,
                        )
                        .to_rgba8();
                    let (width, height) = image.dimensions();
                    let mut pixels = image.into_raw();
                    for chunk in pixels.chunks_exact_mut(4) {
                        let a = chunk[3] as u32;
                        chunk[0] = ((chunk[0] as u32 * a) / 255) as u8;
                        chunk[1] = ((chunk[1] as u32 * a) / 255) as u8;
                        chunk[2] = ((chunk[2] as u32 * a) / 255) as u8;
                    }

                    Some((
                        Size::new(DevicePixels(width as i32), DevicePixels(height as i32)),
                        pixels,
                    ))
                }
                Err(_) => match cx
                    .svg_renderer()
                    .render_pixmap(&bytes, SvgSize::Size(Size::new(target_size, target_size)))
                {
                    Ok(pixmap) => Some((
                        Size::new(
                            DevicePixels(pixmap.width() as i32),
                            DevicePixels(pixmap.height() as i32),
                        ),
                        pixmap.data().to_vec(),
                    )),
                    Err(err) => {
                        eprintln!(
                            "[drag-icon] full-color decode failed for {:?}: {err:?}",
                            icon.path
                        );
                        None
                    }
                },
            }
        }
    }
}

pub fn svg_icon_for(entry: &FsEntry, cx: &App) -> AnyElement {
    render_icon(cx.global::<IconThemeState>().icon_for(entry), px(16.0))
}

pub fn directory_svg_icon(cx: &App) -> AnyElement {
    render_icon(cx.global::<IconThemeState>().directory_icon(), px(16.0))
}

pub fn generic_file_svg_icon(cx: &App) -> AnyElement {
    render_icon(cx.global::<IconThemeState>().generic_file_icon(), px(16.0))
}

pub fn svg_icon_for_size(entry: &FsEntry, size: Pixels, cx: &App) -> AnyElement {
    render_icon(cx.global::<IconThemeState>().icon_for(entry), size)
}

pub fn directory_svg_icon_size(size: Pixels, cx: &App) -> AnyElement {
    render_icon(cx.global::<IconThemeState>().directory_icon(), size)
}

pub fn generic_file_svg_icon_size(size: Pixels, cx: &App) -> AnyElement {
    render_icon(cx.global::<IconThemeState>().generic_file_icon(), size)
}

pub fn directory_menu_icon(cx: &App) -> SharedString {
    cx.global::<IconThemeState>()
        .directory_icon()
        .path
        .to_string_lossy()
        .into_owned()
        .into()
}

pub fn generic_file_menu_icon(cx: &App) -> SharedString {
    cx.global::<IconThemeState>()
        .generic_file_icon()
        .path
        .to_string_lossy()
        .into_owned()
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_manifest_and_resolves_paths() {
        let json = r#"{
            "themes": [{
                "name": "Test Theme",
                "directory_icons": { "collapsed": "./icons/folder.svg" },
                "file_suffixes": { "rs": "rust" },
                "file_icons": { "rust": { "path": "./icons/rust.svg" } }
            }]
        }"#;
        let base = PathBuf::from("/theme/root");

        let themes = parse_manifest(json, &base);

        assert_eq!(themes.len(), 1);
        let theme = &themes[0];
        assert_eq!(theme.name, "Test Theme");
        assert_eq!(theme.directory_icon, Some(base.join("icons/folder.svg")));
        assert_eq!(
            theme.file_icons.get("rust"),
            Some(&base.join("icons/rust.svg"))
        );
    }

    #[test]
    fn missing_optional_fields_default_to_vendored_zed_defaults() {
        let json = r#"{ "themes": [{ "name": "Bare" }] }"#;

        let themes = parse_manifest(json, Path::new("/root"));

        let theme = &themes[0];
        assert!(theme.directory_icon.is_none());
        assert!(theme.file_icons.is_empty());
        assert_eq!(
            theme.file_stems,
            super::super::zed_default_icons::file_stems()
        );
        assert_eq!(
            theme.file_suffixes,
            super::super::zed_default_icons::file_suffixes()
        );
    }

    #[test]
    fn file_stems_take_priority_over_file_suffixes() {
        let mut file_stems = HashMap::new();
        file_stems.insert("Dockerfile".to_string(), "docker".to_string());
        let mut file_suffixes = HashMap::new();
        file_suffixes.insert("Dockerfile".to_string(), "generic".to_string());
        let mut file_icons = HashMap::new();
        file_icons.insert("docker".to_string(), PathBuf::from("/root/docker.svg"));
        file_icons.insert("generic".to_string(), PathBuf::from("/root/generic.svg"));

        let theme = LoadedIconTheme {
            name: "T".into(),
            directory_icon: None,
            file_stems,
            file_suffixes,
            file_icons,
        };

        let resolved = resolve_in(&theme, Path::new("/project/Dockerfile"));

        assert_eq!(resolved, Some(PathBuf::from("/root/docker.svg")));
    }

    #[test]
    fn primary_theme_resolves_common_suffixes_via_vendored_zed_defaults() {
        let json = r#"{
            "themes": [{
                "name": "Sparse Theme",
                "file_suffixes": { "as": "actionscript" },
                "file_icons": {
                    "rust": { "path": "./icons/rust.svg" },
                    "actionscript": { "path": "./icons/as.svg" }
                }
            }]
        }"#;
        let primary = parse_manifest(json, Path::new("/theme"))
            .into_iter()
            .next()
            .unwrap();

        let mut fallback_icons = HashMap::new();
        fallback_icons.insert("file".to_string(), PathBuf::from("/fallback/file.svg"));
        fallback_icons.insert("code".to_string(), PathBuf::from("/fallback/code.svg"));
        let mut fallback_suffixes = HashMap::new();
        fallback_suffixes.insert("rs".to_string(), "code".to_string());
        let fallback = LoadedIconTheme {
            name: "Fallback".into(),
            directory_icon: Some(PathBuf::from("/fallback/folder.svg")),
            file_stems: HashMap::new(),
            file_suffixes: fallback_suffixes,
            file_icons: fallback_icons,
        };
        let state = IconThemeState {
            primary: Some(primary),
            fallback,
        };

        let entry = FsEntry {
            name: "main.rs".into(),
            path: PathBuf::from("/home/user/main.rs"),
            is_dir: false,
            size: 0,
            modified: None,
        };

        let icon = state.icon_for(&entry);
        assert_eq!(icon.path, PathBuf::from("/theme/icons/rust.svg"));
        assert_eq!(icon.mode, IconRenderMode::FullColor);
    }

    #[test]
    fn resolves_stem_only_filenames_via_vendored_zed_defaults() {
        let mut file_icons = HashMap::new();
        file_icons.insert("docker".to_string(), PathBuf::from("/theme/docker.svg"));
        let theme = LoadedIconTheme {
            name: "T".into(),
            directory_icon: None,
            file_stems: super::super::zed_default_icons::file_stems(),
            file_suffixes: super::super::zed_default_icons::file_suffixes(),
            file_icons,
        };

        let resolved = resolve_in(&theme, Path::new("/project/Dockerfile"));

        assert_eq!(resolved, Some(PathBuf::from("/theme/docker.svg")));
    }

    #[test]
    fn resolves_multi_part_filenames_via_vendored_zed_defaults() {
        let mut file_icons = HashMap::new();
        file_icons.insert("eslint".to_string(), PathBuf::from("/theme/eslint.svg"));
        let theme = LoadedIconTheme {
            name: "T".into(),
            directory_icon: None,
            file_stems: super::super::zed_default_icons::file_stems(),
            file_suffixes: super::super::zed_default_icons::file_suffixes(),
            file_icons,
        };

        let resolved = resolve_in(&theme, Path::new("/project/eslint.config.js"));

        assert_eq!(resolved, Some(PathBuf::from("/theme/eslint.svg")));
    }

    #[test]
    fn falls_back_to_fallback_theme_file_key_when_nothing_matches() {
        let mut fallback_icons = HashMap::new();
        fallback_icons.insert("file".to_string(), PathBuf::from("/fallback/file.svg"));
        let fallback = LoadedIconTheme {
            name: "Fallback".into(),
            directory_icon: Some(PathBuf::from("/fallback/folder.svg")),
            file_stems: HashMap::new(),
            file_suffixes: HashMap::new(),
            file_icons: fallback_icons,
        };
        let state = IconThemeState {
            primary: None,
            fallback,
        };

        let entry = FsEntry {
            name: "mystery.xyz".into(),
            path: PathBuf::from("/home/user/mystery.xyz"),
            is_dir: false,
            size: 0,
            modified: None,
        };

        let icon = state.icon_for(&entry);
        assert_eq!(icon.path, PathBuf::from("/fallback/file.svg"));
        assert_eq!(icon.mode, IconRenderMode::Tinted);
    }

    #[test]
    fn directory_uses_primary_theme_when_available() {
        let fallback = LoadedIconTheme {
            name: "Fallback".into(),
            directory_icon: Some(PathBuf::from("/fallback/folder.svg")),
            file_stems: HashMap::new(),
            file_suffixes: HashMap::new(),
            file_icons: HashMap::new(),
        };
        let primary = LoadedIconTheme {
            name: "Primary".into(),
            directory_icon: Some(PathBuf::from("/primary/folder.svg")),
            file_stems: HashMap::new(),
            file_suffixes: HashMap::new(),
            file_icons: HashMap::new(),
        };
        let state = IconThemeState {
            primary: Some(primary),
            fallback,
        };

        let entry = FsEntry {
            name: "Documents".into(),
            path: PathBuf::from("/home/user/Documents"),
            is_dir: true,
            size: 0,
            modified: None,
        };

        let icon = state.icon_for(&entry);
        assert_eq!(icon.path, PathBuf::from("/primary/folder.svg"));
        assert_eq!(icon.mode, IconRenderMode::FullColor);
    }

    #[test]
    fn discovers_theme_from_synthetic_extension_directory() {
        let root = std::env::temp_dir().join(format!("zex_icon_theme_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let ext_dir = root.join("fake-extension");
        std::fs::create_dir_all(ext_dir.join("icon_themes")).unwrap();
        std::fs::create_dir_all(ext_dir.join("icons")).unwrap();

        std::fs::write(
            ext_dir.join("extension.toml"),
            "id = \"fake-extension\"\nicon_themes = [\"icon_themes/fake.json\"]\n",
        )
        .unwrap();
        std::fs::write(
            ext_dir.join("icon_themes/fake.json"),
            r#"{
                "themes": [{
                    "name": "Fake Theme",
                    "file_suffixes": { "rs": "rust" },
                    "file_icons": { "rust": { "path": "./icons/rust.svg" } }
                }]
            }"#,
        )
        .unwrap();
        std::fs::write(ext_dir.join("icons/rust.svg"), b"<svg></svg>").unwrap();

        let themes = discover_zed_extension_themes_in(&root);

        assert_eq!(themes.len(), 1);
        assert_eq!(themes[0].name, "Fake Theme");
        assert_eq!(
            themes[0].file_icons.get("rust"),
            Some(&ext_dir.join("icons/rust.svg"))
        );

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn skips_extensions_with_no_icon_theme_or_malformed_manifest() {
        let root =
            std::env::temp_dir().join(format!("zex_icon_theme_test_skip_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);

        let plain_ext = root.join("plain-extension");
        std::fs::create_dir_all(&plain_ext).unwrap();
        std::fs::write(plain_ext.join("extension.toml"), "id = \"plain\"\n").unwrap();

        let broken_ext = root.join("broken-extension");
        std::fs::create_dir_all(&broken_ext).unwrap();
        std::fs::write(
            broken_ext.join("extension.toml"),
            "id = \"broken\"\nicon_themes = [\"icon_themes/missing.json\"]\n",
        )
        .unwrap();

        let themes = discover_zed_extension_themes_in(&root);

        assert!(themes.is_empty());

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn loads_bundled_default_theme_from_real_assets() {
        let theme = load_bundled_default();

        assert_eq!(theme.name, "Zex Default");
        assert!(theme.directory_icon.as_ref().is_some_and(|p| p.exists()));
        assert!(theme.file_icons.get("file").is_some_and(|p| p.exists()));
        assert_eq!(theme.file_suffixes.get("rs"), Some(&"code".to_string()));
    }
}
