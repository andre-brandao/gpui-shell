//! Layout helpers for rendering story examples in a consistent format.
//!
//! Inspired by Zed's `ComponentExample` / `ComponentExampleGroup` from the
//! `component` crate, adapted to engram's theme tokens.

use gpui::{
    AnyElement, App, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div,
    prelude::FluentBuilder, px,
};
use ui::*;

/// A single variant example: name label above a bordered card containing the element.
#[derive(IntoElement)]
pub struct StoryExample {
    name: SharedString,
    description: Option<SharedString>,
    element: AnyElement,
}

impl StoryExample {
    pub fn new(name: impl Into<SharedString>, element: AnyElement) -> Self {
        Self {
            name: name.into(),
            description: None,
            element,
        }
    }

    pub fn description(mut self, description: impl Into<SharedString>) -> Self {
        self.description = Some(description.into());
        self
    }
}

impl RenderOnce for StoryExample {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        v_flex()
            .gap(Spacing::Small.pixels())
            .child(
                v_flex()
                    .child(Label::new(self.name).size(TextSize::Small))
                    .when_some(self.description, |this, desc| {
                        this.child(Label::new(desc).size(TextSize::XSmall).color(Color::Muted))
                    }),
            )
            .child(
                div()
                    .w_full()
                    .p(Spacing::Large.pixels())
                    .flex()
                    .items_center()
                    .gap(Spacing::Medium.pixels())
                    .rounded(Radius::Small.pixels())
                    .border_1()
                    .border_color(colors.border_variant)
                    .child(self.element),
            )
    }
}

/// A titled group of examples: uppercase section label with a horizontal rule,
/// followed by a list of [`StoryExample`]s.
#[derive(IntoElement)]
pub struct StoryExampleGroup {
    title: SharedString,
    examples: Vec<StoryExample>,
}

impl StoryExampleGroup {
    pub fn new(title: impl Into<SharedString>, examples: Vec<StoryExample>) -> Self {
        Self {
            title: title.into(),
            examples,
        }
    }
}

impl RenderOnce for StoryExampleGroup {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        v_flex()
            .gap(Spacing::Medium.pixels())
            .child(
                h_flex()
                    .items_center()
                    .gap(Spacing::Small.pixels())
                    .child(
                        Label::new(SharedString::from(self.title.to_uppercase()))
                            .size(TextSize::XSmall)
                            .color(Color::Muted),
                    )
                    .child(div().h(px(1.0)).flex_1().bg(colors.border_variant)),
            )
            .children(self.examples)
    }
}

/// Convenience: create a single example.
pub fn example(name: impl Into<SharedString>, element: AnyElement) -> StoryExample {
    StoryExample::new(name, element)
}

/// Convenience: create a titled group of examples.
pub fn example_group(
    title: impl Into<SharedString>,
    examples: Vec<StoryExample>,
) -> StoryExampleGroup {
    StoryExampleGroup::new(title, examples)
}
