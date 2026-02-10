use gpui::{App, IntoElement, RenderOnce, Window, div, prelude::*, px};

use crate::{ActiveTheme, spacing};

/// A horizontal separator line for use between list items.
#[derive(IntoElement)]
pub struct ListSeparator;

impl RenderOnce for ListSeparator {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .h_px()
            .w_full()
            .my(px(spacing::SM))
            .bg(cx.theme().border.subtle)
    }
}
