//! Base16 palettes offered by the gallery and the showcase.
//!
//! Shared by both binaries so the two views always exercise the same set.

use ui::{Base16Palette, ThemeScheme, builtin_schemes};

/// Palettes offered by the switcher.
///
/// A deliberately varied set: the whole point of deriving ~50 tokens from
/// 16 colors is that the derivation has to hold up on palettes it was not
/// tuned against. A light scheme and a low-contrast one catch far more than
/// another dark grey would.
pub fn schemes() -> Vec<ThemeScheme> {
    let mut schemes = builtin_schemes();
    for (name, description, colors) in EXTRA_SCHEMES {
        match Base16Palette::from_hex(colors) {
            Ok(palette) => schemes.push(ThemeScheme::new(*name, *description, palette)),
            // A malformed palette here is a typo in this file, not user
            // input - surface it rather than silently showing fewer schemes.
            Err(err) => eprintln!("story: scheme `{name}` is malformed: {err}"),
        }
    }
    schemes
}

type SchemeSpec = (&'static str, &'static str, &'static [&'static str; 16]);

static EXTRA_SCHEMES: &[SchemeSpec] = &[
    (
        "Gruvbox Dark",
        "Warm, medium contrast",
        &[
            "#282828", "#3c3836", "#504945", "#665c54", "#bdae93", "#d5c4a1", "#ebdbb2", "#fbf1c7",
            "#fb4934", "#fe8019", "#fabd2f", "#b8bb26", "#8ec07c", "#83a598", "#d3869b", "#d65d0e",
        ],
    ),
    (
        "Solarized Light",
        "Light scheme - checks the derived tints invert correctly",
        &[
            "#fdf6e3", "#eee8d5", "#93a1a1", "#839496", "#657b83", "#586e75", "#073642", "#002b36",
            "#dc322f", "#cb4b16", "#b58900", "#859900", "#2aa198", "#268bd2", "#6c71c4", "#d33682",
        ],
    ),
    (
        "Nord",
        "Low contrast - stresses the subtle border steps",
        &[
            "#2e3440", "#3b4252", "#434c5e", "#4c566a", "#d8dee9", "#e5e9f0", "#eceff4", "#8fbcbb",
            "#bf616a", "#d08770", "#ebcb8b", "#a3be8c", "#88c0d0", "#81a1c1", "#b48ead", "#5e81ac",
        ],
    ),
];
