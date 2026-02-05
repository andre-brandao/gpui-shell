//! Launcher button placeholder using Zed's `Button`.
//!
//! In later phases this will open the launcher and integrate with IPC.

use gpui::{Context, Window, prelude::*};
use services::Services;
use ui::{Button, ButtonStyle, IconName, IconSize, prelude::*};

pub struct LauncherBtn {
    _services: Services,
}

impl LauncherBtn {
    pub fn new(services: Services, _cx: &mut Context<Self>) -> Self {
        Self {
            _services: services,
        }
    }
}

impl Render for LauncherBtn {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Button::new("launcher", "Launcher")
            .style(ButtonStyle::Subtle)
            .icon(IconName::Menu)
            .icon_size(IconSize::Small)
            .on_click(|_, _, _| {
                tracing::info!("Launcher button clicked (placeholder)");
            })
            .tooltip(ui::Tooltip::text("Open launcher"))
    }
}
