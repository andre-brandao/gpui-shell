use gpui::*;
use prelude::FluentBuilder;

#[derive(IntoElement)]
pub struct Icon {
    icon_path: SharedString,
    size: f32,
    color: Rgba,
    hover: Option<Rgba>,
}

impl Icon {
    pub fn new(icon_path: SharedString) -> Self {
        Icon {
            icon_path,
            size: 22.0,
            color: rgb(0x000000),
            hover: None,
        }
    }

    pub fn size(mut self, size: f32) -> Self {
        self.size = size;
        self
    }

    pub fn color(mut self, color: Rgba) -> Self {
        self.color = color;
        self
    }

    pub fn hover(mut self, color: Rgba) -> Self {
        self.hover = Some(color);
        self
    }
}

impl RenderOnce for Icon {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        svg()
            .flex()
            .items_center()
            .justify_center()
            .size(px(self.size))
            .text_color(self.color)
            .when_some(self.hover, |this, color| {
                this.hover(|this| this.text_color(color))
            })
            .path(self.icon_path)
    }
}
