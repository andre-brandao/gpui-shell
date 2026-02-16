//! Launcher button widget for opening the application launcher.

use crate::launcher;
use gpui::{Context, MouseButton, Window, div, prelude::*, px};
use ui::{ActiveTheme, radius};

use super::style;
use crate::config::ActiveConfig;

/// A button widget that opens the launcher when clicked.
pub struct LauncherBtn;

const LAUNCHER_ICON: &str = "󰀻";

impl LauncherBtn {
    /// Create a new launcher button.
    pub fn new(_cx: &mut Context<Self>) -> Self {
        LauncherBtn
    }
}

impl Render for LauncherBtn {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let is_vertical = cx.config().bar.is_vertical();

        // Pre-compute colors for closures
        let interactive_default = theme.interactive.default;
        let interactive_hover = theme.interactive.hover;
        let interactive_active = theme.interactive.active;
        let text_primary = theme.text.primary;

        div()
            .id("launcher-btn")
            .flex()
            .items_center()
            .justify_center()
            .px(px(style::chip_padding_x(is_vertical)))
            .py(px(style::CHIP_PADDING_Y))
            .rounded(px(radius::SM))
            .cursor_pointer()
            .bg(interactive_default)
            .hover(move |s| s.bg(interactive_hover))
            .active(move |s| s.bg(interactive_active))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |_, _, _, cx| {
                    launcher::toggle(None, cx);
                }),
            )
            .child(
                div().flex().items_center().justify_center().child(
                    div()
                        .text_size(px(style::icon(is_vertical)))
                        .text_color(text_primary)
                        .child(LAUNCHER_ICON),
                ),
            )
    }
}
