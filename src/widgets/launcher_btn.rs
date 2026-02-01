use crate::launcher;
use crate::services::Services;
use gpui::{Context, MouseButton, Window, div, prelude::*, px, rgba};

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
            .px(px(8.))
            .py(px(4.))
            .rounded(px(4.))
            .cursor_pointer()
            .hover(|s| s.bg(rgba(0x333333ff)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |_this, _, _, cx| {
                    launcher::toggle(services.clone(), cx);
                }),
            )
            .child(div().text_size(px(14.)).child(""))
    }
}
