#![allow(clippy::single_range_in_vec_init)]

use crate::prelude::*;

pub struct HighlightedLabelStory;

impl Render for HighlightedLabelStory {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap(Spacing::Large.pixels()).child(example_group(
            "HighlightedLabel",
            vec![
                example(
                    "Character indices",
                    HighlightedLabel::new("hello world", vec![0, 1, 2, 3, 4]).into_any_element(),
                ),
                example(
                    "Search match (fuzzy)",
                    HighlightedLabel::new("components.rs", vec![0, 4, 8, 10]).into_any_element(),
                ),
                example(
                    "Byte ranges",
                    v_flex()
                        .gap(Spacing::Small.pixels())
                        .child(HighlightedLabel::from_ranges("search_term_here", &[0..6]))
                        .child(HighlightedLabel::from_ranges(
                            "find the needle in the haystack",
                            &[9..15],
                        ))
                        .into_any_element(),
                ),
                example(
                    "With modifiers",
                    HighlightedLabel::new("italic highlighted", vec![0, 1, 2, 3, 4, 5])
                        .italic()
                        .size(TextSize::Large)
                        .into_any_element(),
                ),
            ],
        ))
    }
}

pub fn build(_window: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_cx| HighlightedLabelStory).into()
}
