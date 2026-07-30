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

/// gpui throws an SVG's colors away and keeps only its alpha
/// ([`gpui::SvgRenderer`]), so "does it parse" is not enough: an icon that
/// flattens to a filled box, or to nothing at all, passes every check above
/// and still renders as a blob or as blank space. This measures what the
/// renderer actually produces.
///
/// Both bounds have caught something real. The brand marks arrived as
/// multi-variant logos where the wrong pick flattens solid, and two of them
/// had to be reduced to the single path that defines the silhouette - taking
/// the wrong path there yields either an empty render or the clip rect, i.e.
/// full coverage.
#[test]
fn every_icon_renders_a_readable_silhouette() {
    let renderer = gpui::SvgRenderer::new(std::sync::Arc::new(assets::Assets));

    let mut coverage: Vec<(String, f32)> = Vec::new();
    for name in IconName::iter() {
        let bytes = assets::Assets
            .load(&name.path())
            .ok()
            .flatten()
            .unwrap_or_else(|| panic!("{name:?} does not resolve"));

        let image = renderer
            .render_single_frame(&bytes, 1.0)
            .unwrap_or_else(|error| panic!("{name:?} failed to render: {error}"));

        let pixels = image.as_bytes(0).expect("rendered frame");
        let opaque = pixels.chunks_exact(4).filter(|px| px[3] > 8).count();
        coverage.push((
            format!("{name:?}"),
            opaque as f32 / (pixels.len() / 4) as f32,
        ));
    }

    // The floor has to clear `WifiZero`, which is legitimately a single dot
    // (~0.7%) - the lightest mark the set is ever meant to draw.
    let blank: Vec<&(String, f32)> = coverage.iter().filter(|(_, c)| *c < 0.003).collect();
    assert!(blank.is_empty(), "icon(s) render essentially nothing: {blank:?}");

    let solid: Vec<&(String, f32)> = coverage.iter().filter(|(_, c)| *c > 0.92).collect();
    assert!(solid.is_empty(), "icon(s) render as a filled box: {solid:?}");

    coverage.sort_by(|a, b| b.1.total_cmp(&a.1));
    println!("highest coverage: {:?}", &coverage[..6]);
    println!("lowest coverage: {:?}", &coverage[coverage.len() - 6..]);
}
