use gpui::{App, IntoElement, RenderOnce, Window, div, prelude::*};

use crate::{ActiveTheme, Spacing};

/// A horizontal separator line for use between list items.
#[derive(IntoElement)]
pub struct ListSeparator;

impl RenderOnce for ListSeparator {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .h_px()
            .w_full()
            .my(Spacing::Medium.pixels())
            .bg(cx.theme().colors().border_variant)
    }
}
