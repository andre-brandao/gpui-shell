use crate::launcher;
use crate::services::Services;
use crate::theme::{icon_size, interactive, radius, spacing, text};
use gpui::{Context, MouseButton, Window, div, prelude::*, px};

/// A button widget that opens the application launcher.
pub struct LauncherBtn {
    services: Services,
}

impl LauncherBtn {
    pub fn with_services(services: Services, _cx: &mut Context<Self>) -> Self {
        LauncherBtn { services }
    }
}

impl Render for LauncherBtn {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let services = self.services.clone();

        div()
            .id("launcher-btn")
            .px(px(spacing::SM))
            .py(px(spacing::XS))
            .rounded(px(radius::SM))
            .cursor_pointer()
            .hover(|s| s.bg(interactive::hover()))
            .active(|s| s.bg(interactive::active()))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |_this, _, _, cx| {
                    launcher::toggle(services.clone(), cx);
                }),
            )
            .child(
                div()
                    .text_size(px(icon_size::LG))
                    .text_color(text::primary())
                    .child(""),
            )
    }
}
