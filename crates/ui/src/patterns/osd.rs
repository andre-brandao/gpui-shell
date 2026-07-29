//! On-screen display: the pill that flashes up on a volume or brightness
//! change.
//!
//! Horizontal reads icon → bar → value; vertical reads value → bar → icon,
//! so the value stays at the top and the bar still fills from the bottom.

use gpui::{App, Hsla, IntoElement, RenderOnce, Window, div, prelude::*, px, relative};

use crate::{ActiveTheme, Color, Icon, IconName, IconSize, Radius, Spacing};

const TRACK: f32 = 6.0;

/// A level readout: icon, filled track, percentage.
///
/// Fills its parent, so give it one. `level` may exceed 100 (overamplified
/// volume) - the track clamps, the number does not.
#[derive(IntoElement)]
#[must_use = "OsdIndicator does nothing unless rendered"]
pub struct OsdIndicator {
    icon: IconName,
    level: u8,
    vertical: bool,
    fill: Option<Hsla>,
    icon_color: Option<Hsla>,
}

impl OsdIndicator {
    pub fn new(icon: IconName, level: u8) -> Self {
        Self {
            icon,
            level,
            vertical: false,
            fill: None,
            icon_color: None,
        }
    }

    pub fn vertical(mut self, vertical: bool) -> Self {
        self.vertical = vertical;
        self
    }

    /// Track colour. Defaults to the accent - the caller overrides it to say
    /// something about the value (muted, overamplified).
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
        let colors = cx.theme().colors();
        let fill = self.fill.unwrap_or(colors.accent);
        let icon_color = self.icon_color.unwrap_or(colors.text);
        let track_bg = colors.elevated_surface_background;
        let text = colors.text;
        let vertical = self.vertical;
        let filled = (self.level as f32 / 100.0).min(1.0);

        let icon = Icon::new(self.icon)
            .size(IconSize::Large)
            .color(Color::Custom(icon_color));

        let value = div()
            .text_size(px(12.0))
            .text_color(text)
            .child(format!("{}%", self.level));

        let track = div()
            .flex_1()
            .bg(track_bg)
            .rounded(px(TRACK / 2.0))
            .overflow_hidden()
            .map(|el| {
                let bar = div().bg(fill).rounded(px(TRACK / 2.0));
                if vertical {
                    // Grows upward, so the fill has to sit at the bottom of
                    // the track.
                    el.w(px(TRACK))
                        .flex()
                        .flex_col()
                        .justify_end()
                        .child(bar.w_full().h(relative(filled)))
                } else {
                    el.h(px(TRACK)).child(bar.h_full().w(relative(filled)))
                }
            });

        div()
            .size_full()
            .flex()
            .justify_center()
            .items_center()
            .gap(Spacing::Large.pixels())
            .bg(colors.background)
            .border_1()
            .border_color(colors.border)
            .rounded(Radius::Large.pixels())
            .map(|el| {
                if vertical {
                    el.flex_col()
                        .py(Spacing::Large.pixels())
                        .child(value)
                        .child(track)
                        .child(icon)
                } else {
                    el.px(Spacing::Large.pixels())
                        .child(icon)
                        .child(track)
                        // Fixed so the pill does not twitch between 9% and
                        // 100%.
                        .child(div().w(px(36.0)).text_right().child(value))
                }
            })
    }
}
