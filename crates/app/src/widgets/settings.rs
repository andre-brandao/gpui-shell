//! Settings / Control Center launcher placeholder.

use gpui::{Context, Window, prelude::*};
use services::Services;
use ui::{IconButton, IconName, IconSize, prelude::*};

pub struct Settings {
    _services: Services,
}

impl Settings {
    pub fn new(services: Services, _cx: &mut Context<Self>) -> Self {
        Self {
            _services: services,
        }
    }
}

impl Render for Settings {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        IconButton::new("settings", IconName::Cog)
            .icon_size(IconSize::Small)
            .tooltip(ui::Tooltip::text("Open control center (placeholder)"))
            .on_click(|_, _, _| {
                tracing::info!("Settings button clicked (placeholder)");
            })
    }
}
