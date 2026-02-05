//! Placeholder Workspaces widget.

use gpui::{Context, Window, prelude::*};
use services::Services;
use ui::{Color, Label, LabelSize, prelude::*};

pub struct Workspaces {
    _services: Services,
}

impl Workspaces {
    pub fn new(services: Services, _cx: &mut Context<Self>) -> Self {
        Self {
            _services: services,
        }
    }
}

impl Render for Workspaces {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Label::new("WS")
            .size(LabelSize::Small)
            .color(Color::Default)
    }
}
