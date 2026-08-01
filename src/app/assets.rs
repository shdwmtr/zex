use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets"]
#[include = "icons/*"]
#[include = "icon_themes/*"]
#[include = "themes/**/*"]
struct EmbeddedAssets;

pub struct Assets;

impl Assets {
    pub fn new() -> Self {
        Self
    }

    pub fn get(path: &str) -> Option<Cow<'static, [u8]>> {
        EmbeddedAssets::get(path).map(|file| file.data)
    }

    pub fn read_to_string(path: &str) -> String {
        let data = Self::get(path).unwrap_or_else(|| panic!("zex asset {path:?} must be present"));
        String::from_utf8(data.into_owned())
            .unwrap_or_else(|_| panic!("zex asset {path:?} must be valid UTF-8"))
    }

    pub fn list_dir(prefix: &str) -> Vec<SharedString> {
        let prefix = prefix.strip_suffix('/').unwrap_or(prefix);
        let mut names: Vec<SharedString> = EmbeddedAssets::iter()
            .filter_map(|file| {
                let rest = file.strip_prefix(prefix)?.strip_prefix('/')?;
                let name = rest.split('/').next()?;
                Some(SharedString::from(name.to_string()))
            })
            .collect();
        names.sort();
        names.dedup();
        names
    }
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(Self::get(path))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Self::list_dir(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn root_level_icon_png_is_not_embedded() {
        assert!(EmbeddedAssets::get("icon.png").is_none());
    }

    #[test]
    fn known_content_dirs_are_still_embedded() {
        assert!(EmbeddedAssets::get("icons/folder.svg").is_some());
        assert!(EmbeddedAssets::get("icon_themes/zex-default.json").is_some());
        assert!(EmbeddedAssets::get("themes/zex-default.json").is_some());
        assert!(EmbeddedAssets::get("themes/zed_builtin/ayu.json").is_some());
    }
}
