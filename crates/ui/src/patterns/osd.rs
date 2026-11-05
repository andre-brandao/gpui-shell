//! On-screen display: the pill that flashes up on a volume or brightness change.

use gpui::{
    AnimationExt, App, ElementId, Hsla, IntoElement, RenderOnce, Window, div, prelude::*, relative,
    rems,
};

use super::surface_radius;
use crate::{
    ActiveTheme, Color, Icon, IconName, IconSize, Motion, Radius, Spacing, SpringConfigExt,
    TextSize,
};

/// Thickness of the level track.
const TRACK: Spacing = Spacing::Small;
/// Room for the widest value (`100%`), so the pill does not twitch between
/// 9% and 100%.
const VALUE_WIDTH: f32 = 2.8;
/// Settling tolerance for the level spring, in percent. Finer than a pixel
/// of track; coarser than gpui's default, which would run the frame loop
/// long after the eye has stopped seeing movement.
const LEVEL_EPSILON: f32 = 0.05;

/// A level readout: icon, filled track, percentage.
///
/// The level springs to each new value - the fill and the number move
/// together - under [`key`](Self::key), so a change of what is shown
/// (volume to brightness) starts fresh instead of sliding between the two.
#[derive(IntoElement)]
#[must_use = "OsdIndicator does nothing unless rendered"]
pub struct OsdIndicator {
    icon: IconName,
    level: u8,
    vertical: bool,
    fill: Option<Hsla>,
    icon_color: Option<Hsla>,
    background: Option<Hsla>,
    key: ElementId,
}

impl OsdIndicator {
    pub fn new(icon: IconName, level: u8) -> Self {
        Self {
            icon,
            level,
            vertical: false,
            fill: None,
            icon_color: None,
            background: None,
            key: ElementId::from("osd-level"),
        }
    }

    /// What the level is a level of; the spring's identity.
    pub fn key(mut self, key: impl Into<ElementId>) -> Self {
        self.key = key.into();
        self
    }

    /// Surface fill in place of the theme's, e.g. at a configured opacity.
    pub fn background(mut self, background: Hsla) -> Self {
        self.background = Some(background);
        self
    }

    pub fn vertical(mut self, vertical: bool) -> Self {
        self.vertical = vertical;
        self
    }

    /// Track colour.
    pub fn fill(mut self, fill: Hsla) -> Self {
        self.fill = Some(fill);
        self
    }

    pub fn icon_color(mut self, color: Hsla) -> Self {
        self.icon_color = Some(color);
        self
    }
}

impl RenderOnce for OsdIndicator {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.theme();
        let colors = theme.colors();
        let background = self.background.unwrap_or_else(|| theme.surface(None));
        let fill = self.fill.unwrap_or(colors.accent);
        let icon_color = self.icon_color.unwrap_or(colors.text);
        let track_bg = colors.elevated_surface_background;
        let text = colors.text;
        let vertical = self.vertical;

        let icon = Icon::new(self.icon)
            .size(IconSize::Large)
            .color(Color::Custom(icon_color));

        div()
            .size_full()
            .flex()
            .justify_center()
            .items_center()
            .gap(Spacing::Large.pixels())
            .bg(background)
            .border_1()
            .border_color(colors.border)
            .rounded(surface_radius())
            .map(|el| {
                if vertical {
                    el.flex_col().py(Spacing::Large.pixels())
                } else {
                    el.px(Spacing::Large.pixels())
                }
            })
            .with_spring(
                self.key,
                Motion::SNAPPY
                    .to(self.level as f32)
                    .with_epsilon(LEVEL_EPSILON),
                move |el, level: f32| {
                    // Readout and fill come off the one sprung value, so the
                    // number counts while the bar moves.
                    let shown = level.round().max(0.0) as u32;
                    let filled = (level / 100.0).clamp(0.0, 1.0);

                    let value = div()
                        .text_size(TextSize::Small.rems())
                        .text_color(text)
                        // Nowrap: a flex column lays text out at min-content, which splits
                        // "100%" over two lines.
                        .whitespace_nowrap()
                        .child(format!("{shown}%"));

                    let track = div()
                        .flex_1()
                        .bg(track_bg)
                        .rounded(Radius::Full.pixels())
                        .overflow_hidden()
                        .map(|el| {
                            let bar = div().bg(fill).rounded(Radius::Full.pixels());
                            if vertical {
                                // Grows upward, so the fill has to sit at the
                                // bottom of the track.
                                el.w(TRACK.pixels())
                                    .flex()
                                    .flex_col()
                                    .justify_end()
                                    .child(bar.w_full().h(relative(filled)))
                            } else {
                                el.h(TRACK.pixels()).child(bar.h_full().w(relative(filled)))
                            }
                        });

                    if vertical {
                        el.child(div().w_full().text_center().child(value))
                            .child(track)
                            .child(icon)
                    } else {
                        el.child(icon)
                            .child(track)
                            .child(div().w(rems(VALUE_WIDTH)).text_right().child(value))
                    }
                },
            )
    }
}
