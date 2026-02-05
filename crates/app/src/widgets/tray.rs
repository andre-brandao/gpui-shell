//! Placeholder system tray widget.

use gpui::{Context, Window, prelude::*};
use services::Services;
use ui::{Color, Label, LabelSize, prelude::*};

pub struct Tray {
    _services: Services,
}

impl Tray {
    pub fn new(services: Services, _cx: &mut Context<Self>) -> Self {
        Self {
            _services: services,
        }
    }
}

impl Render for Tray {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Label::new("Tray")
            .size(LabelSize::Small)
            .color(Color::Muted)
    }
}
