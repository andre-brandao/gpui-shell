use anyhow::anyhow;
use gpui::{AssetSource, Result, SharedString};
use rust_embed::RustEmbed;
use std::borrow::Cow;

/// The shell's embedded asset bundle.
///
/// Ships the Lucide icon set under `icons/`, which is what `ui::IconName`
/// resolves against. The embed root is the crate directory rather than
/// `icons/` so the keys keep their `icons/` prefix - `IconName::path()`
/// asks for `icons/<name>.svg`, and a mismatch here makes every icon
/// silently resolve to nothing.
///
/// Register it at startup:
///
/// ```no_run
/// use gpui_platform::application;
/// application().with_assets(assets::Assets).run(|_cx| {});
/// ```
#[derive(RustEmbed)]
#[folder = "."]
#[include = "icons/**/*.svg"]
#[exclude = "icons/LICENSE"]
#[exclude = "icons/README.md"]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        Self::get(path)
            .map(|f| Some(f.data))
            .ok_or_else(|| anyhow!("could not find asset at path \"{path}\""))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| p.starts_with(path).then(|| p.into()))
            .collect())
    }
}
