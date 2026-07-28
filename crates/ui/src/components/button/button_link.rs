//! [`ButtonLink`] - an underlined text link that opens a URL.
//!
//! Renders as an underlined label with an optional trailing arrow icon to
//! communicate that clicking navigates outside the app. Has no inner
//! padding - the link sits flush with surrounding text.

use gpui::{App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, prelude::*};

use crate::components::button::button_like::ButtonLike;
use crate::components::icon::{Icon, IconName, IconSize};
use crate::components::label::{Label, LabelCommon};
use crate::components::stack::h_flex;
use crate::theme::Color;
use crate::theme::TextSize;
use crate::traits::Clickable;

/// A button that looks like a web link and opens a URL on click.
#[derive(IntoElement)]
#[must_use = "ButtonLink does nothing unless rendered"]
pub struct ButtonLink {
    label: SharedString,
    label_size: TextSize,
    label_color: Color,
    link: String,
    show_icon: bool,
}

impl ButtonLink {
    pub fn new(label: impl Into<SharedString>, link: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            link: link.into(),
            label_size: TextSize::Small,
            label_color: Color::Default,
            show_icon: true,
        }
    }

    pub fn no_icon(mut self) -> Self {
        self.show_icon = false;
        self
    }

    pub fn label_size(mut self, label_size: TextSize) -> Self {
        self.label_size = label_size;
        self
    }

    pub fn label_color(mut self, label_color: Color) -> Self {
        self.label_color = label_color;
        self
    }
}

impl RenderOnce for ButtonLink {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        let id = format!("{}-{}", self.label, self.link);
        let link = self.link;

        // Zero-padding link - matches Zed's ButtonSize::None behavior.
        let mut btn = ButtonLike::new(id);
        btn.base = btn.base.self_start();
        btn.padding(gpui::px(0.0), gpui::px(0.0))
            .child(
                h_flex()
                    .gap(gpui::px(2.0))
                    .child(
                        Label::new(self.label)
                            .size(self.label_size)
                            .color(self.label_color)
                            .underline(),
                    )
                    .when(self.show_icon, |this: gpui::Div| {
                        this.child(
                            Icon::new(IconName::ArrowUpRight)
                                .size(IconSize::Small)
                                .color(Color::Muted),
                        )
                    }),
            )
            .on_click(move |_, _, cx: &mut App| cx.open_url(&link))
    }
}
