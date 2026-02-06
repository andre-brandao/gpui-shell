//! Launcher button — opens the launcher overlay on click.

use gpui::{Context, Window, prelude::*};
use services::Services;
use ui::{Button, ButtonStyle, IconName, IconSize, prelude::*};

pub struct LauncherBtn {
    services: Services,
}

impl LauncherBtn {
    pub fn new(services: Services, _cx: &mut Context<Self>) -> Self {
        Self { services }
    }
}

impl Render for LauncherBtn {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let services = self.services.clone();

        Button::new("launcher", "")
            .style(ButtonStyle::Subtle)
            .icon(IconName::Menu)
            .icon_size(IconSize::Small)
            .on_click(move |_, _, cx| {
                crate::launcher::toggle(services.clone(), cx);
            })
            .tooltip(ui::Tooltip::text("Open launcher"))
    }
}
