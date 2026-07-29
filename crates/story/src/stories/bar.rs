//! Bar - the edge-anchored status surface and the chips inside it.
//!
//! Shown at both orientations, because that is where bar work goes wrong:
//! the same widget has to read as a pill on a horizontal bar and as a stack
//! on a vertical one.

use gpui::{AnyElement, Hsla, div};
use ui::patterns::{BarChip, BarEdge, BarSurface};

use crate::prelude::*;

/// Stand-in widgets. The real ones are entities wired to services; what the
/// pattern sees is an element.
fn chip(icon: IconName, label: &'static str, vertical: bool, cx: &App) -> AnyElement {
    let colors = cx.theme().colors();
    let content = if vertical {
        v_flex()
            .items_center()
            .gap(px(2.))
            .child(Icon::new(icon).size(IconSize::Small))
            .child(Label::new(label).size(TextSize::XSmall))
    } else {
        h_flex()
            .items_center()
            .gap(px(4.))
            .child(Icon::new(icon).size(IconSize::Small))
            .child(Label::new(label).size(TextSize::Small))
    };

    BarChip::new(content)
        .vertical(vertical)
        .background(colors.surface_background)
        .border(Some(colors.border))
        .hover(Some(colors.element_hover))
        .into_any_element()
}

fn group(icons: &[IconName], vertical: bool, cx: &App) -> AnyElement {
    let colors = cx.theme().colors();
    let dots = icons
        .iter()
        .map(|icon| Icon::new(*icon).size(IconSize::Small).into_any_element());

    BarChip::new(
        div()
            .flex()
            .when(vertical, |el| el.flex_col())
            .items_center()
            .gap(px(3.))
            .children(dots),
    )
    .vertical(vertical)
    .grouped(true)
    .background(colors.surface_background)
    .border(Some(colors.border))
    .into_any_element()
}

fn horizontal_bar(cx: &App) -> AnyElement {
    div()
        .w_full()
        .h(px(32.))
        .child(
            BarSurface::new(BarEdge::Top)
                .border(true)
                .padding(px(14.))
                .start([
                    chip(IconName::Layout, "1", false, cx),
                    group(&[IconName::Cpu, IconName::MemoryStick], false, cx),
                ])
                .center([chip(IconName::Layers, "zed - bar.rs", false, cx)])
                .end([
                    chip(IconName::Volume, "62%", false, cx),
                    chip(IconName::Battery, "84%", false, cx),
                    chip(IconName::Clock, "09:41", false, cx),
                ]),
        )
        .into_any_element()
}

fn vertical_bar(cx: &App) -> AnyElement {
    div()
        .w(px(44.))
        .h(px(320.))
        .child(
            BarSurface::new(BarEdge::Left)
                .border(true)
                .start([
                    chip(IconName::Layout, "1", true, cx),
                    group(&[IconName::Cpu, IconName::MemoryStick], true, cx),
                ])
                .end([
                    chip(IconName::Volume, "62", true, cx),
                    chip(IconName::Clock, "9:41", true, cx),
                ]),
        )
        .into_any_element()
}

/// A chip with no background or border - what `widget_background = false`
/// and `widget_border = false` in the bar config look like.
fn bare_chips(cx: &App) -> AnyElement {
    let transparent: Option<Hsla> = None;
    div()
        .flex()
        .items_center()
        .child(
            BarChip::new(
                h_flex()
                    .items_center()
                    .gap(px(4.))
                    .child(Icon::new(IconName::Wifi).size(IconSize::Small))
                    .child(Label::new("Home").size(TextSize::Small)),
            )
            .border(transparent),
        )
        .child(
            BarChip::new(Label::new("09:41").size(TextSize::Small))
                .hover(Some(cx.theme().colors().element_hover)),
        )
        .into_any_element()
}

pub struct BarStory;

impl Render for BarStory {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap(Spacing::XLarge.pixels()).child(example_group(
            "Bar",
            vec![
                example("Horizontal", horizontal_bar(cx)).description(
                    "Start, centre and end sections; only the screen edge is bordered.",
                ),
                example("Vertical", vertical_bar(cx))
                    .description("Chips stack, radii tighten, labels drop a size."),
                example("Chip without chrome", bare_chips(cx))
                    .description("Background and border are the app's call, so both are optional."),
            ],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| BarStory).into()
}
