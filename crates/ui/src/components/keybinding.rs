//! KeyBinding - display-only chip strip for keyboard shortcuts.
//!
//! A pared-down take on Zed's `KeyBinding`. Zed integrates with `Action`,
//! `FocusHandle`, and the live keymap so a binding can be looked up by
//! action - that needs an `App` and a focus handle. Engram has no keymap
//! infrastructure today, so this version is purely *visual*: callers hand
//! it the labels they want rendered (e.g. `["Cmd", "S"]`) and we draw a
//! row of small chips with a thin border.
//!
//! When engram grows action / keymap support we can add a parallel
//! `KeyBinding::for_action(...)` constructor without breaking call sites.

use crate::theme::{ActiveTheme, Color, Radius, Spacing};
use gpui::{IntoElement, RenderOnce, SharedString, div, prelude::*, px};

use crate::components::label::{Label, LabelCommon};
use crate::components::stack::h_flex;
use crate::theme::TextSize;

#[derive(IntoElement)]
#[must_use = "KeyBinding does nothing unless rendered"]
pub struct KeyBinding {
    keys: Vec<SharedString>,
}

impl KeyBinding {
    /// Build a binding from any iterable of key names. The display order
    /// is the iteration order.
    pub fn new<I, S>(keys: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<SharedString>,
    {
        Self {
            keys: keys.into_iter().map(Into::into).collect(),
        }
    }
}

impl RenderOnce for KeyBinding {
    fn render(self, _window: &mut gpui::Window, cx: &mut gpui::App) -> impl IntoElement {
        let colors = cx.theme().colors();
        h_flex()
            .gap(Spacing::XXSmall.pixels())
            .children(self.keys.into_iter().map(|key| {
                div()
                    .px(Spacing::XSmall.pixels())
                    .py(px(1.0))
                    .rounded(Radius::XSmall.pixels())
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.element_background)
                    .child(Label::new(key).size(TextSize::XSmall).color(Color::Muted))
            }))
    }
}
