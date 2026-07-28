//! Every [`IconName`] must resolve through the [`AssetSource`] the app
//! actually registers.
//!
//! This deliberately goes through `assets::Assets` rather than looking at the
//! filesystem. A file can be present on disk and still be unreachable at
//! runtime - if the embed root and [`IconName::path`]'s prefix disagree, every
//! lookup returns `None` and gpui silently paints nothing where the icon
//! should be. An earlier version of this test checked the directory instead,
//! and missed exactly that.

use gpui::AssetSource;
use strum::IntoEnumIterator;
use ui::IconName;

#[test]
fn every_icon_name_resolves_through_the_asset_source() {
    let missing: Vec<String> = IconName::iter()
        .filter(|name| {
            !matches!(assets::Assets.load(&name.path()), Ok(Some(bytes)) if !bytes.is_empty())
        })
        .map(|name| format!("{name:?} -> {}", name.path()))
        .collect();

    assert!(
        missing.is_empty(),
        "{} IconName variant(s) do not resolve:\n  {}",
        missing.len(),
        missing.join("\n  ")
    );
}

/// The reverse direction: an SVG nobody can name is dead weight in the
/// binary. Not fatal, but it should be a deliberate choice.
#[test]
fn every_embedded_svg_is_reachable_from_an_icon_name() {
    let named: Vec<String> = IconName::iter()
        .map(|name| name.path().to_string())
        .collect();

    let orphans: Vec<String> = assets::Assets
        .list("icons/")
        .expect("icon bundle lists")
        .into_iter()
        .map(|path| path.to_string())
        .filter(|path| path.ends_with(".svg") && !named.contains(path))
        .collect();

    assert!(
        orphans.is_empty(),
        "{} embedded SVG(s) are not reachable from any IconName:\n  {}",
        orphans.len(),
        orphans.join("\n  ")
    );
}

/// Icons are tinted from the theme, which only works if they carry
/// `stroke="currentColor"` rather than a baked-in color.
#[test]
fn every_icon_inherits_its_color() {
    let hardcoded: Vec<String> = IconName::iter()
        .filter_map(|name| {
            let bytes = assets::Assets.load(&name.path()).ok()??;
            let svg = String::from_utf8_lossy(&bytes);
            let tintable = svg.contains("currentColor");
            (!tintable).then(|| format!("{name:?}"))
        })
        .collect();

    assert!(
        hardcoded.is_empty(),
        "{} icon(s) have no `currentColor`, so the theme can't tint them:\n  {}",
        hardcoded.len(),
        hardcoded.join("\n  ")
    );
}
