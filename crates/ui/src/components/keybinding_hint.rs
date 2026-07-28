//! [`KeybindingHint`] - inline keyboard-shortcut glyph for hover cards and
//! descriptions.
//!
//! A compact, italic rendering of a [`KeyBinding`] with optional prefix and
//! suffix text. Where [`KeyBinding`] is a standalone chip strip (toolbar or
//! menu end-slot), `KeybindingHint` is designed to sit *inside* running text
//! - think "Press **Enter** to confirm" in a tooltip or hover card.
//!
//! Adapted from zed's `ui::KeybindingHint`, simplified to match engram's
//! visual-only [`KeyBinding`] (no action / keymap lookup).

use crate::theme::{ActiveTheme, Appearance, Color, Radius, Spacing, TextSize};
use gpui::{
    BoxShadow, FontStyle, IntoElement, Pixels, RenderOnce, SharedString, Styled, Window, point,
    prelude::*, px,
};

use crate::components::keybinding::KeyBinding;
use crate::components::label::{Label, LabelCommon};
use crate::components::stack::h_flex;

/// An inline keybinding hint with optional prefix and suffix text.
#[derive(IntoElement)]
#[must_use = "KeybindingHint does nothing unless rendered"]
pub struct KeybindingHint {
    prefix: Option<SharedString>,
    suffix: Option<SharedString>,
    keybinding: KeyBinding,
    size: Option<Pixels>,
}

impl KeybindingHint {
    pub fn new(keybinding: KeyBinding) -> Self {
        Self {
            prefix: None,
            suffix: None,
            keybinding,
            size: None,
        }
    }

    pub fn with_prefix(prefix: impl Into<SharedString>, keybinding: KeyBinding) -> Self {
        Self {
            prefix: Some(prefix.into()),
            suffix: None,
            keybinding,
            size: None,
        }
    }

    pub fn with_suffix(keybinding: KeyBinding, suffix: impl Into<SharedString>) -> Self {
        Self {
            prefix: None,
            suffix: Some(suffix.into()),
            keybinding,
            size: None,
        }
    }

    pub fn prefix(mut self, prefix: impl Into<SharedString>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    pub fn suffix(mut self, suffix: impl Into<SharedString>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    pub fn size(mut self, size: impl Into<Pixels>) -> Self {
        self.size = Some(size.into());
        self
    }
}

impl RenderOnce for KeybindingHint {
    fn render(self, _window: &mut Window, cx: &mut gpui::App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let is_light = cx.theme().appearance() == Appearance::Light;

        let border_color = colors.border.opacity(if is_light { 0.6 } else { 0.4 });
        let bg_color = colors.elevated_surface_background;
        let shadow_color = colors.text.opacity(if is_light { 0.04 } else { 0.08 });

        let size = self
            .size
            .unwrap_or_else(|| TextSize::Small.pixels(cx.theme().font_size));

        let mut base = h_flex();
        base.text_style().font_style = Some(FontStyle::Italic);

        base.gap(Spacing::XXSmall.pixels())
            .text_size(size)
            .text_color(colors.text_disabled)
            .when_some(self.prefix, |this, prefix| {
                this.child(
                    Label::new(prefix)
                        .size(TextSize::XSmall)
                        .color(Color::Disabled),
                )
            })
            .child(
                h_flex()
                    .rounded(Radius::XSmall.pixels())
                    .px(px(3.0))
                    .border_1()
                    .border_color(border_color)
                    .bg(bg_color)
                    .shadow(vec![BoxShadow {
                        color: shadow_color,
                        offset: point(px(0.), px(1.)),
                        blur_radius: px(0.),
                        spread_radius: px(0.),
                        inset: false,
                    }])
                    .child(self.keybinding),
            )
            .when_some(self.suffix, |this, suffix| {
                this.child(
                    Label::new(suffix)
                        .size(TextSize::XSmall)
                        .color(Color::Disabled),
                )
            })
    }
}
