//! Launcher button widget for opening the application launcher.

use crate::launcher;
use gpui::{Context, MouseButton, Window, div, prelude::*, px};
use services::Services;
use ui::{ActiveTheme, icon_size, radius, spacing};

/// A button widget that opens the launcher when clicked.
pub struct LauncherBtn {
    services: Services,
}

impl LauncherBtn {
    /// Create a new launcher button with the given services.
    pub fn new(services: Services, _cx: &mut Context<Self>) -> Self {
        LauncherBtn { services }
    }
}

impl Render for LauncherBtn {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();

        // Pre-compute colors for closures
        let interactive_hover = theme.interactive.hover;
        let interactive_active = theme.interactive.active;
        let text_primary = theme.text.primary;

        div()
            .id("launcher-btn")
            .flex()
            .items_center()
            .justify_center()
            .px(px(spacing::SM))
            .py(px(spacing::XS))
            .rounded(px(radius::SM))
            .cursor_pointer()
            .hover(move |s| s.bg(interactive_hover))
            .active(move |s| s.bg(interactive_active))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| {
                    launcher::toggle(this.services.clone(), cx);
                }),
            )
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(px(spacing::XS))
                    // Grid/Apps icon
                    .child(
                        div()
                            .text_size(px(icon_size::LG))
                            .text_color(text_primary)
                            .child(""), // nf-oct-apps
                    ),
            )
    }
}
