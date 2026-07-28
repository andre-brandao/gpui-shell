//! Story-local asset bundle + fallback composition with the shell's [`Assets`].
//!
//! The story gallery doubles as a live demo of [`Icon::from_path`]. To make
//! that demo honest, the story needs at least one SVG that is *not* part of
//! the shell's embedded set - otherwise `Icon::from_path` would trivially
//! resolve against the shell's bundle and the consumer-side wiring would stay
//! invisible.
//!
//! [`StoryAssets`] embeds story-local SVGs under `demo/`. [`ComposedAssets`]
//! is the [`AssetSource`] actually handed to `application().with_assets(...)`;
//! it tries the story bundle first and falls back to the shell's [`Assets`],
//! mirroring the pattern a downstream consumer would use to layer their own
//! assets on top of the library defaults.

use std::borrow::Cow;

use assets::Assets as ShellAssets;
use gpui::{AssetSource, Result, SharedString};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "assets"]
#[include = "demo/*.svg"]
pub struct StoryAssets;

impl AssetSource for StoryAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        Ok(Self::get(path).map(|f| f.data))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| p.starts_with(path).then(|| p.into()))
            .collect())
    }
}

/// AssetSource handed to gpui at startup. Checks story-local assets first,
/// falls back to the shell's bundle.
pub struct ComposedAssets;

impl AssetSource for ComposedAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if let Some(bytes) = StoryAssets.load(path)? {
            return Ok(Some(bytes));
        }
        ShellAssets.load(path)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut combined = StoryAssets.list(path)?;
        combined.extend(ShellAssets.list(path)?);
        Ok(combined)
    }
}
