//! HoverCard - a richer tooltip-like surface for preview content.
//!
//! Where [`Tooltip`](super::tooltip) shows a title and optional metadata
//! line, `HoverCard` is a full [`Popover`]-backed card that accepts
//! arbitrary children - user profiles, link previews, definition
//! summaries, etc.
//!
//! Like `Tooltip`, `HoverCard` is a view (`impl Render`) so it can be
//! handed to GPUI's `.tooltip(builder)` method on any stateful element.
//! The [`HoverCard::build`] helper produces the closure.
//!
//! The body is supplied as a builder closure (`Fn(&mut Window, &mut App)
//! -> AnyElement`) rather than a pre-built element list. The closure is
//! invoked on every `Render::render` pass, so the card's contents survive
//! repeated renders (e.g. when the parent view re-notifies). Pre-built
//! children would be consumed on first render and vanish on the next.

use std::rc::Rc;

use crate::theme::{ActiveTheme, Radius, Spacing};
use gpui::{
    AnyElement, AnyView, App, Context, IntoElement, Pixels, Render, SharedString, Styled, Window,
    div, prelude::*,
};

use crate::components::label::{Label, LabelCommon};
use crate::components::stack::v_flex;
use crate::styles::ElevationIndex;
use crate::theme::TextSize;

type HoverCardBody = Rc<dyn Fn(&mut Window, &mut App) -> AnyElement + 'static>;

/// A rich hover card for preview content.
pub struct HoverCard {
    title: Option<SharedString>,
    min_width: Option<Pixels>,
    body: HoverCardBody,
}

impl HoverCard {
    /// Build a hover card whose body is produced by `body` on every render
    /// pass. The closure is intentionally re-invoked rather than consumed
    /// so the card survives repeated renders.
    pub fn new(body: impl Fn(&mut Window, &mut App) -> AnyElement + 'static) -> Self {
        Self {
            title: None,
            min_width: None,
            body: Rc::new(body),
        }
    }

    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn min_width(mut self, width: Pixels) -> Self {
        self.min_width = Some(width);
        self
    }

    /// Build a tooltip-builder closure that produces this hover card.
    /// Pass the result directly to gpui's `.tooltip(...)` method.
    pub fn build(
        make: impl Fn() -> HoverCard + 'static,
    ) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
        move |_window, cx| cx.new(|_| make()).into()
    }
}

impl Render for HoverCard {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = *cx.theme().colors();
        let shadow = ElevationIndex::ElevatedSurface.shadow(cx);
        let body = (self.body)(window, cx);

        div()
            .pl(Spacing::XSmall.pixels())
            .pt(Spacing::Small.pixels())
            .child(
                v_flex()
                    .when_some(self.min_width, |this, w| this.min_w(w))
                    .gap(Spacing::Small.pixels())
                    .px(Spacing::Medium.pixels())
                    .py(Spacing::Small.pixels())
                    .rounded(Radius::Small.pixels())
                    .bg(colors.elevated_surface_background)
                    .border_1()
                    .border_color(colors.border)
                    .shadow(shadow)
                    .when_some(self.title.clone(), |this, title| {
                        this.child(
                            Label::new(title)
                                .size(TextSize::Small)
                                .weight(gpui::FontWeight::SEMIBOLD),
                        )
                    })
                    .child(body),
            )
    }
}
