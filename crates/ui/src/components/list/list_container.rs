use gpui::{AnyElement, App, IntoElement, RenderOnce, SharedString, Window, prelude::*, px};

use crate::components::label::{Color, Label, LabelCommon};
use crate::{spacing, v_flex};

/// Message displayed when a list has no children.
pub enum EmptyMessage {
    Text(SharedString),
    Element(AnyElement),
}

impl From<String> for EmptyMessage {
    fn from(s: String) -> Self {
        EmptyMessage::Text(SharedString::from(s))
    }
}

impl From<&str> for EmptyMessage {
    fn from(s: &str) -> Self {
        EmptyMessage::Text(SharedString::from(s.to_owned()))
    }
}

impl From<AnyElement> for EmptyMessage {
    fn from(e: AnyElement) -> Self {
        EmptyMessage::Element(e)
    }
}

/// A container for displaying a collection of list items with an optional
/// header and empty state.
#[derive(IntoElement)]
pub struct List {
    empty_message: EmptyMessage,
    header: Option<AnyElement>,
    toggle: Option<bool>,
    children: Vec<AnyElement>,
}

impl Default for List {
    fn default() -> Self {
        Self::new()
    }
}

impl List {
    pub fn new() -> Self {
        Self {
            empty_message: EmptyMessage::Text("No items".into()),
            header: None,
            toggle: None,
            children: Vec::new(),
        }
    }

    pub fn empty_message(mut self, message: impl Into<EmptyMessage>) -> Self {
        self.empty_message = message.into();
        self
    }

    pub fn header(mut self, header: impl Into<Option<AnyElement>>) -> Self {
        self.header = header.into();
        self
    }

    pub fn toggle(mut self, toggle: impl Into<Option<bool>>) -> Self {
        self.toggle = toggle.into();
        self
    }
}

impl ParentElement for List {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for List {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        v_flex()
            .w_full()
            .py(px(spacing::XS))
            .children(self.header)
            .map(|this| match (self.children.is_empty(), self.toggle) {
                (false, _) => this.children(self.children),
                (true, Some(false)) => this,
                (true, _) => match self.empty_message {
                    EmptyMessage::Text(text) => this
                        .px(px(spacing::SM))
                        .child(Label::new(text).color(Color::Muted)),
                    EmptyMessage::Element(element) => this.child(element),
                },
            })
    }
}
