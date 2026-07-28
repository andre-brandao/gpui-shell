//! Every [`IconName`] must resolve to a real SVG in the asset bundle.
//!
//! Icon lookup failures are invisible at compile time and nearly invisible at
//! runtime - gpui just paints nothing where the icon should be - so a missing
//! or misnamed file otherwise ships as a silently blank button.
//!
//! The icons live in the sibling `assets` crate (it owns the `AssetSource`
//! the app registers), so this walks the workspace layout directly rather
//! than going through gpui.

use std::path::PathBuf;

use strum::IntoEnumIterator;
use ui::IconName;

fn icons_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("crates/ui has a parent")
        .join("assets/icons")
}

#[test]
fn every_icon_name_has_an_svg() {
    let dir = icons_dir();
    assert!(dir.is_dir(), "icon bundle not found at {}", dir.display());

    let missing: Vec<_> = IconName::iter()
        .filter(|name| {
            let stem: &'static str = (*name).into();
            !dir.join(format!("{stem}.svg")).is_file()
        })
        .map(|name| {
            let stem: &'static str = name.into();
            format!("{name:?} -> icons/{stem}.svg")
        })
        .collect();

    assert!(
        missing.is_empty(),
        "{} IconName variant(s) have no SVG:\n  {}",
        missing.len(),
        missing.join("\n  ")
    );
}

/// The reverse direction: an SVG nobody can name is dead weight in the
/// binary. Not fatal, but it should be a deliberate choice.
#[test]
fn every_svg_is_reachable_from_an_icon_name() {
    let named: Vec<String> = IconName::iter()
        .map(|name| {
            let stem: &'static str = name.into();
            stem.to_string()
        })
        .collect();

    let orphans: Vec<String> = std::fs::read_dir(icons_dir())
        .expect("icon bundle is readable")
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension()? != "svg" {
                return None;
            }
            let stem = path.file_stem()?.to_str()?.to_string();
            (!named.contains(&stem)).then_some(stem)
        })
        .collect();

    assert!(
        orphans.is_empty(),
        "{} SVG(s) are not reachable from any IconName:\n  {}",
        orphans.len(),
        orphans.join("\n  ")
    );
}
