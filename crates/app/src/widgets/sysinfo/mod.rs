//! Placeholder SysInfo widget.

use gpui::{Context, Window, prelude::*};
use services::Services;
use ui::{Color, Label, LabelSize, prelude::*};

pub struct SysInfo {
    _services: Services,
}

impl SysInfo {
    pub fn new(services: Services, _cx: &mut Context<Self>) -> Self {
        Self {
            _services: services,
        }
    }
}

impl Render for SysInfo {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        Label::new("Sys").size(LabelSize::Small).color(Color::Muted)
    }
}
