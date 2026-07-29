//! Panel - the two surface levels every floating shell popup paints with.
//!
//! A `Styled` extension, not an element, so a panel keeps its own id, focus
//! handle and scroll handle: `div().id("x").panel_surface(cx)`.

use gpui::{AnyElement, div};
use ui::patterns::PanelSurface;

use crate::prelude::*;

fn control_center(cx: &App) -> AnyElement {
    div()
        .w(px(300.))
        .p(Spacing::Large.pixels())
        .panel_surface(cx)
        .flex()
        .flex_col()
        .gap(Spacing::Large.pixels())
        .child(
            h_flex()
                .justify_between()
                .items_center()
                .p(Spacing::Medium.pixels())
                .panel_card(cx)
                .child(
                    v_flex().child(Label::new("84%")).child(
                        Label::new("2h14 remaining")
                            .size(TextSize::XSmall)
                            .color(Color::Muted),
                    ),
                )
                .child(Icon::new(IconName::Battery)),
        )
        .child(
            h_flex()
                .gap(Spacing::Medium.pixels())
                .items_center()
                .p(Spacing::Medium.pixels())
                .panel_card(cx)
                .child(Icon::new(IconName::Volume))
                .child(div().flex_1().child(ProgressBar::new(62., 100.))),
        )
        .child(
            h_flex()
                .gap(Spacing::Medium.pixels())
                .items_center()
                .p(Spacing::Medium.pixels())
                .panel_card(cx)
                .child(Icon::new(IconName::Wifi))
                .child(Label::new("Home").size(TextSize::Small)),
        )
        .into_any_element()
}

fn bare_panel(cx: &App) -> AnyElement {
    div()
        .w(px(220.))
        .p(Spacing::Large.pixels())
        .panel_surface(cx)
        .child(Label::new("A panel with nothing in it.").color(Color::Muted))
        .into_any_element()
}

pub struct PanelStory;

impl Render for PanelStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap(Spacing::XLarge.pixels()).child(example_group(
            "Panel",
            vec![
                example("panel_surface", bare_panel(cx))
                    .description("Opaque background, hairline border, large radius."),
                example("panel_surface + panel_card", control_center(cx))
                    .description("Cards lift off the panel: quieter border, tighter radius."),
            ],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| PanelStory).into()
}
