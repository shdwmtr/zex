use std::path::PathBuf;

use gpui::Rgba;
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
    #[serde(default)]
    pub git: GitSettings,
    #[serde(default)]
    pub disk_usage: DiskUsageSettings,
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct GitSettings {
    pub enabled: bool,
    pub status: GitStatusSettings,
    pub branch: GitBranchSettings,
    pub refresh: GitRefreshSettings,
    pub cli: GitCliSettings,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GitBadgeStyle {
    TextColor,
    Icon,
    Both,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct GitStatusSettings {
    pub enabled: bool,
    pub show_untracked: bool,
    pub show_ignored: bool,
    pub dim_ignored: bool,
    pub aggregate_folders: bool,
    pub badge_style: GitBadgeStyle,
    pub colors: GitStatusColors,
}

impl Default for GitStatusSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_untracked: true,
            show_ignored: false,
            dim_ignored: true,
            aggregate_folders: true,
            badge_style: GitBadgeStyle::TextColor,
            colors: GitStatusColors::default(),
        }
    }
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct GitStatusColors {
    pub modified: Option<Rgba>,
    pub added: Option<Rgba>,
    pub deleted: Option<Rgba>,
    pub renamed: Option<Rgba>,
    pub untracked: Option<Rgba>,
    pub ignored: Option<Rgba>,
    pub conflicted: Option<Rgba>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct GitBranchSettings {
    pub enabled: bool,
    pub show_in_status_bar: bool,
    pub show_dirty_indicator: bool,
    pub show_ahead_behind: bool,
}

impl Default for GitBranchSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            show_in_status_bar: true,
            show_dirty_indicator: true,
            show_ahead_behind: true,
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct GitRefreshSettings {
    pub poll_interval_ms: u64,
}

impl Default for GitRefreshSettings {
    fn default() -> Self {
        Self {
            poll_interval_ms: 2000,
        }
    }
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct GitCliSettings {
    pub binary_path: Option<String>,
    pub timeout_ms: u64,
    pub max_repo_entries: Option<u64>,
}

impl Default for GitCliSettings {
    fn default() -> Self {
        Self {
            binary_path: None,
            timeout_ms: 3000,
            max_repo_entries: None,
        }
    }
}

#[derive(Deserialize, Default, Debug, Clone, PartialEq)]
#[serde(default)]
pub struct DiskUsageSettings {
    pub cross_filesystem_boundaries: bool,
    pub follow_symlinks: bool,
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

    #[test]
    fn git_defaults_to_disabled_when_omitted() {
        let settings: Settings = serde_json::from_str("{}").unwrap();

        assert!(!settings.git.enabled);
        assert!(settings.git.status.enabled);
        assert!(settings.git.status.show_untracked);
        assert!(!settings.git.status.show_ignored);
        assert!(settings.git.status.dim_ignored);
        assert!(settings.git.status.aggregate_folders);
        assert_eq!(settings.git.status.badge_style, GitBadgeStyle::TextColor);
        assert_eq!(settings.git.status.colors, GitStatusColors::default());
        assert!(settings.git.branch.enabled);
        assert!(settings.git.branch.show_in_status_bar);
        assert!(settings.git.branch.show_dirty_indicator);
        assert!(settings.git.branch.show_ahead_behind);
        assert_eq!(settings.git.refresh.poll_interval_ms, 2000);
        assert_eq!(settings.git.cli.binary_path, None);
        assert_eq!(settings.git.cli.timeout_ms, 3000);
        assert_eq!(settings.git.cli.max_repo_entries, None);
    }

    #[test]
    fn git_empty_object_uses_same_defaults() {
        let settings: Settings = serde_json::from_str(r#"{ "git": {} }"#).unwrap();

        assert_eq!(settings.git, GitSettings::default());
    }

    #[test]
    fn git_enabled_flag_parses_with_defaults_for_the_rest() {
        let settings: Settings =
            serde_json::from_str(r#"{ "git": { "enabled": true } }"#).unwrap();

        assert!(settings.git.enabled);
        assert!(settings.git.status.enabled);
        assert_eq!(settings.git.refresh.poll_interval_ms, 2000);
    }

    #[test]
    fn git_granular_overrides_parse() {
        let contents = r#"{
            "git": {
                "enabled": true,
                "status": {
                    "show_ignored": true,
                    "badge_style": "icon"
                },
                "branch": {
                    "show_ahead_behind": false
                },
                "refresh": {
                    "poll_interval_ms": 0
                },
                "cli": {
                    "binary_path": "/usr/bin/git",
                    "timeout_ms": 500,
                    "max_repo_entries": 20000
                }
            }
        }"#;

        let settings: Settings = serde_json::from_str(contents).unwrap();

        assert!(settings.git.status.show_ignored);
        assert_eq!(settings.git.status.badge_style, GitBadgeStyle::Icon);
        assert!(settings.git.status.dim_ignored);
        assert!(!settings.git.branch.show_ahead_behind);
        assert!(settings.git.branch.show_in_status_bar);
        assert_eq!(settings.git.refresh.poll_interval_ms, 0);
        assert_eq!(
            settings.git.cli.binary_path.as_deref(),
            Some("/usr/bin/git")
        );
        assert_eq!(settings.git.cli.timeout_ms, 500);
        assert_eq!(settings.git.cli.max_repo_entries, Some(20000));
    }

    #[test]
    fn git_custom_colors_parse_hex_strings() {
        let contents = r##"{
            "git": {
                "status": {
                    "colors": {
                        "modified": "#ffaa00",
                        "conflicted": "#ff0000ff"
                    }
                }
            }
        }"##;

        let settings: Settings = serde_json::from_str(contents).unwrap();

        assert_eq!(settings.git.status.colors.modified, Rgba::try_from("#ffaa00").ok());
        assert_eq!(
            settings.git.status.colors.conflicted,
            Rgba::try_from("#ff0000ff").ok()
        );
        assert_eq!(settings.git.status.colors.added, None);
    }

    #[test]
    fn disk_usage_defaults_to_safe_values_when_omitted() {
        let settings: Settings = serde_json::from_str("{}").unwrap();

        assert!(!settings.disk_usage.cross_filesystem_boundaries);
        assert!(!settings.disk_usage.follow_symlinks);
    }

    #[test]
    fn disk_usage_granular_overrides_parse() {
        let contents = r#"{
            "disk_usage": {
                "cross_filesystem_boundaries": true,
                "follow_symlinks": true
            }
        }"#;

        let settings: Settings = serde_json::from_str(contents).unwrap();

        assert!(settings.disk_usage.cross_filesystem_boundaries);
        assert!(settings.disk_usage.follow_symlinks);
    }
}
