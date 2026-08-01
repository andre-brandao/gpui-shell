//! DescriptionList - a key-value display for structured information.

use crate::theme::{ActiveTheme, Color, Spacing};
use gpui::{
    AnyElement, App, IntoElement, RenderOnce, SharedString, Styled, Window, div, prelude::*, px,
};

use crate::components::label::{Label, LabelCommon};
use crate::components::stack::{h_flex, v_flex};
use crate::theme::TextSize;

/// A single key-value entry in a [`DescriptionList`].
pub struct DescriptionEntry {
    label: SharedString,
    value: AnyElement,
}

impl DescriptionEntry {
    pub fn new(label: impl Into<SharedString>, value: impl IntoElement) -> Self {
        Self {
            label: label.into(),
            value: value.into_any_element(),
        }
    }
}

/// A vertical list of label-value pairs.
#[derive(IntoElement)]
#[must_use = "DescriptionList does nothing unless rendered"]
pub struct DescriptionList {
    entries: Vec<DescriptionEntry>,
    label_width: f32,
    bordered: bool,
}

impl DescriptionList {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            label_width: 120.0,
            bordered: false,
        }
    }

    /// Set the fixed width of the label column in pixels.
    pub fn label_width(mut self, width: f32) -> Self {
        self.label_width = width;
        self
    }

    /// Draw a border between entries.
    pub fn bordered(mut self, bordered: bool) -> Self {
        self.bordered = bordered;
        self
    }

    /// Add a key-value entry.
    pub fn entry(mut self, label: impl Into<SharedString>, value: impl IntoElement) -> Self {
        self.entries.push(DescriptionEntry::new(label, value));
        self
    }

    /// Add multiple entries at once.
    pub fn entries(mut self, entries: impl IntoIterator<Item = DescriptionEntry>) -> Self {
        self.entries.extend(entries);
        self
    }
}

impl Default for DescriptionList {
    fn default() -> Self {
        Self::new()
    }
}

impl RenderOnce for DescriptionList {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = cx.theme().colors();
        let border_color = colors.border_variant;
        let entry_count = self.entries.len();

        v_flex()
            .w_full()
            .children(self.entries.into_iter().enumerate().map(|(i, entry)| {
                let row = h_flex()
                    .w_full()
                    .gap(Spacing::Medium.pixels())
                    .py(Spacing::Small.pixels())
                    .child(
                        div().w(px(self.label_width)).flex_shrink_0().child(
                            Label::new(entry.label)
                                .size(TextSize::Small)
                                .color(Color::Muted),
                        ),
                    )
                    .child(h_flex().flex_1().items_center().child(entry.value));

                if self.bordered && i < entry_count - 1 {
                    row.border_b_1().border_color(border_color)
                } else {
                    row
                }
            }))
    }
}
