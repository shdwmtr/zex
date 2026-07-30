use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize, Default, Debug, PartialEq)]
pub struct Settings {
    pub icon_theme: Option<String>,
    pub ui_font_family: Option<String>,
    pub ui_font_size: Option<f32>,
    pub ui_font_weight: Option<f32>,
    pub show_hidden_files: Option<bool>,
    #[serde(default)]
    pub sidebar: Vec<SidebarItem>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct SidebarEntry {
    pub name: String,
    pub path: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct SidebarSection {
    pub section: String,
    #[serde(default)]
    pub entries: Vec<SidebarEntry>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
pub enum SidebarItem {
    Section(SidebarSection),
    Entry(SidebarEntry),
}

fn config_dir() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("/"));
            home.join(".config")
        })
}

pub fn load() -> Settings {
    let path = config_dir().join("zex").join("config.json");

    std::fs::read_to_string(&path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_icon_theme_field() {
        let contents = r#"{ "icon_theme": "Colored Zed Icons Theme Dark" }"#;

        let settings: Settings = serde_json::from_str(contents).unwrap();

        assert_eq!(
            settings.icon_theme.as_deref(),
            Some("Colored Zed Icons Theme Dark")
        );
    }

    #[test]
    fn empty_object_defaults_icon_theme_to_none() {
        let settings: Settings = serde_json::from_str("{}").unwrap();

        assert_eq!(settings.icon_theme, None);
        assert_eq!(settings.ui_font_family, None);
        assert_eq!(settings.ui_font_size, None);
        assert_eq!(settings.ui_font_weight, None);
        assert_eq!(settings.show_hidden_files, None);
        assert_eq!(settings.sidebar, Vec::<SidebarItem>::new());
    }

    #[test]
    fn parses_ui_font_family_field() {
        let contents = r#"{ "ui_font_family": "JetBrainsMono Nerd Font Mono" }"#;

        let settings: Settings = serde_json::from_str(contents).unwrap();

        assert_eq!(
            settings.ui_font_family.as_deref(),
            Some("JetBrainsMono Nerd Font Mono")
        );
    }

    #[test]
    fn parses_ui_font_size_and_weight_fields() {
        let contents = r#"{ "ui_font_size": 15.0, "ui_font_weight": 500.0 }"#;

        let settings: Settings = serde_json::from_str(contents).unwrap();

        assert_eq!(settings.ui_font_size, Some(15.0));
        assert_eq!(settings.ui_font_weight, Some(500.0));
    }

    #[test]
    fn parses_show_hidden_files_field() {
        let contents = r#"{ "show_hidden_files": true }"#;

        let settings: Settings = serde_json::from_str(contents).unwrap();

        assert_eq!(settings.show_hidden_files, Some(true));
    }

    #[test]
    fn parses_sidebar_entries() {
        let contents = r#"{
            "sidebar": [
                { "name": "Projects", "path": "/home/user/Projects" },
                { "name": "Root", "path": "/" }
            ]
        }"#;

        let settings: Settings = serde_json::from_str(contents).unwrap();

        assert_eq!(
            settings.sidebar,
            vec![
                SidebarItem::Entry(SidebarEntry {
                    name: "Projects".into(),
                    path: "/home/user/Projects".into(),
                }),
                SidebarItem::Entry(SidebarEntry {
                    name: "Root".into(),
                    path: "/".into(),
                }),
            ]
        );
    }

    #[test]
    fn parses_sidebar_sections_with_nested_entries() {
        let contents = r#"{
            "sidebar": [
                { "name": "Root", "path": "/" },
                { "section": "Work", "entries": [
                    { "name": "Projects", "path": "/home/user/Projects" },
                    { "name": "Downloads", "path": "/home/user/Downloads" }
                ] }
            ]
        }"#;

        let settings: Settings = serde_json::from_str(contents).unwrap();

        assert_eq!(
            settings.sidebar,
            vec![
                SidebarItem::Entry(SidebarEntry {
                    name: "Root".into(),
                    path: "/".into(),
                }),
                SidebarItem::Section(SidebarSection {
                    section: "Work".into(),
                    entries: vec![
                        SidebarEntry {
                            name: "Projects".into(),
                            path: "/home/user/Projects".into(),
                        },
                        SidebarEntry {
                            name: "Downloads".into(),
                            path: "/home/user/Downloads".into(),
                        },
                    ],
                }),
            ]
        );
    }

    #[test]
    fn ignores_unknown_fields() {
        let contents = r#"{ "icon_theme": "Foo", "some_future_setting": 42 }"#;

        let settings: Settings = serde_json::from_str(contents).unwrap();

        assert_eq!(settings.icon_theme.as_deref(), Some("Foo"));
    }
}
