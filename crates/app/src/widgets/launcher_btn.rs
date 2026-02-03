//! Launcher button widget for opening the application launcher.

use crate::launcher;
use gpui::{Context, MouseButton, Window, div, prelude::*, px};
use services::Services;
use ui::{font_size, icon_size, interactive, radius, spacing, text};

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
        div()
            .id("launcher-btn")
            .flex()
            .items_center()
            .justify_center()
            .px(px(spacing::SM))
            .py(px(spacing::XS))
            .rounded(px(radius::SM))
            .cursor_pointer()
            .hover(|s| s.bg(interactive::hover()))
            .active(|s| s.bg(interactive::active()))
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
                            .text_size(px(icon_size::MD))
                            .text_color(text::primary())
                            .child(""), // nf-oct-apps
                    ), // Optional label
                       // .child(
                       //     div()
                       //         .text_size(px(font_size::SM))
                       //         .text_color(text::secondary())
                       //         .child("Apps"),
                       // ),
            )
    }
}
