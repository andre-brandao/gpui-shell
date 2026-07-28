//! Tooltip - a small elevated card shown when hovering an interactive element.
//!
//! The underlying machinery is provided by `gpui`: any stateful interactive
//! element (e.g. anything with an `id`) has a [`.tooltip(builder)`](gpui::StatefulInteractiveElement::tooltip)
//! method that takes a closure returning an [`AnyView`]. This module supplies:
//!
//! - a minimal [`Tooltip`] view that lays out a title plus optional metadata
//!   on the theme's elevated surface,
//! - a [`Tooltip::text`] helper that produces a ready-to-pass builder closure,
//!   so call sites can write `.tooltip(Tooltip::text("Save"))` without
//!   wiring up a `cx.new(...)` every time.

use crate::theme::{ActiveTheme, Color, Radius, Spacing};
use gpui::{
    AnyView, App, Context, IntoElement, ParentElement, Render, SharedString, Styled, Window, div,
    prelude::*,
};

use crate::styles::ElevationIndex;

use crate::components::label::{Label, LabelCommon};
use crate::components::stack::v_flex;
use crate::theme::TextSize;

/// An elevated hover card with a title and optional metadata line.
pub struct Tooltip {
    title: SharedString,
    meta: Option<SharedString>,
}

impl Tooltip {
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            meta: None,
        }
    }

    pub fn meta(mut self, meta: impl Into<SharedString>) -> Self {
        self.meta = Some(meta.into());
        self
    }

    /// Build a tooltip-builder closure for a plain text title. Pass the result
    /// directly to gpui's `.tooltip(...)` method on a stateful element.
    pub fn text(
        title: impl Into<SharedString>,
    ) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
        let title = title.into();
        move |_window, cx| cx.new(|_| Self::new(title.clone())).into()
    }

    /// Build a tooltip-builder closure for a title plus a secondary metadata
    /// line (useful for shortcut hints, descriptions, etc.).
    pub fn with_meta(
        title: impl Into<SharedString>,
        meta: impl Into<SharedString>,
    ) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
        let title = title.into();
        let meta = meta.into();
        move |_window, cx| {
            cx.new(|_| Self::new(title.clone()).meta(meta.clone()))
                .into()
        }
    }
}

impl Render for Tooltip {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors();

        // Outer offset keeps the card from appearing directly under the cursor.
        div()
            .pl(Spacing::XSmall.pixels())
            .pt(Spacing::Small.pixels())
            .child(
                v_flex()
                    .gap(Spacing::XXSmall.pixels())
                    .px(Spacing::Small.pixels())
                    .py(Spacing::XSmall.pixels())
                    .rounded(Radius::XSmall.pixels())
                    .bg(colors.elevated_surface_background)
                    .border_1()
                    .border_color(colors.border)
                    .shadow(ElevationIndex::ElevatedSurface.shadow(cx))
                    .child(Label::new(self.title.clone()).size(TextSize::Small))
                    .when_some(self.meta.clone(), |this, meta| {
                        this.child(Label::new(meta).size(TextSize::XSmall).color(Color::Muted))
                    }),
            )
    }
}
