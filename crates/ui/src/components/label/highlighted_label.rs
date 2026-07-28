//! [`HighlightedLabel`] - a label with specific characters highlighted,
//! typically used to show search match positions.

use std::ops::Range;

use crate::theme::{ActiveTheme, Color};
use gpui::{
    App, FontWeight, HighlightStyle, IntoElement, ParentElement, RenderOnce, SharedString,
    StyledText, UnderlineStyle, Window, px,
};

use crate::components::label::label_like::{LabelCommon, LabelLike, LineHeightStyle};
use crate::theme::TextSize;

/// A label that highlights specific characters (identified by byte position).
#[derive(IntoElement)]
#[must_use = "HighlightedLabel does nothing unless rendered"]
pub struct HighlightedLabel {
    base: LabelLike,
    label: SharedString,
    highlight_indices: Vec<usize>,
}

impl HighlightedLabel {
    /// Construct a label with the given characters highlighted.
    /// Characters are identified by UTF-8 byte position.
    pub fn new(label: impl Into<SharedString>, highlight_indices: Vec<usize>) -> Self {
        Self {
            base: LabelLike::new(),
            label: label.into(),
            highlight_indices,
        }
    }

    /// Construct a label with the given byte ranges highlighted.
    pub fn from_ranges(label: impl Into<SharedString>, highlight_ranges: &[Range<usize>]) -> Self {
        let label = label.into();
        let highlight_indices = highlight_ranges
            .iter()
            .flat_map(|range| {
                let mut indices = Vec::new();
                let mut index = range.start;
                while index < range.end {
                    indices.push(index);
                    index += label[index..].chars().next().map_or(0, |c| c.len_utf8());
                }
                indices
            })
            .collect();

        Self {
            base: LabelLike::new(),
            label,
            highlight_indices,
        }
    }
}

impl LabelCommon for HighlightedLabel {
    fn size(mut self, size: TextSize) -> Self {
        self.base = self.base.size(size);
        self
    }

    fn weight(mut self, weight: FontWeight) -> Self {
        self.base = self.base.weight(weight);
        self
    }

    fn line_height_style(mut self, line_height_style: LineHeightStyle) -> Self {
        self.base = self.base.line_height_style(line_height_style);
        self
    }

    fn color(mut self, color: Color) -> Self {
        self.base = self.base.color(color);
        self
    }

    fn strikethrough(mut self) -> Self {
        self.base = self.base.strikethrough();
        self
    }

    fn italic(mut self) -> Self {
        self.base = self.base.italic();
        self
    }

    fn underline(mut self) -> Self {
        self.base = self.base.underline();
        self
    }

    fn alpha(mut self, alpha: f32) -> Self {
        self.base = self.base.alpha(alpha);
        self
    }

    fn truncate(mut self) -> Self {
        self.base = self.base.truncate();
        self
    }

    fn single_line(mut self) -> Self {
        self.base = self.base.single_line();
        self
    }
}

/// Convert a list of highlight byte indices into contiguous ranges.
fn highlight_ranges(
    text: &str,
    indices: &[usize],
    style: HighlightStyle,
) -> Vec<(Range<usize>, HighlightStyle)> {
    let mut highlight_indices = indices.iter().copied().peekable();
    let mut highlights: Vec<(Range<usize>, HighlightStyle)> = Vec::new();

    while let Some(start_ix) = highlight_indices.next() {
        let mut end_ix = start_ix;
        loop {
            end_ix += text[end_ix..].chars().next().map_or(0, |c| c.len_utf8());
            if highlight_indices.next_if(|&ix| ix == end_ix).is_none() {
                break;
            }
        }
        highlights.push((start_ix..end_ix, style));
    }

    highlights
}

impl RenderOnce for HighlightedLabel {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let highlight_color = cx.theme().colors().text_accent;

        let highlights = highlight_ranges(
            &self.label,
            &self.highlight_indices,
            HighlightStyle {
                color: Some(highlight_color),
                font_weight: Some(FontWeight::BOLD),
                underline: Some(UnderlineStyle {
                    thickness: px(1.0),
                    color: Some(highlight_color.opacity(0.4)),
                    wavy: false,
                }),
                ..Default::default()
            },
        );

        let mut text_style = window.text_style();
        text_style.color = self.base.color.hsla(cx.theme().colors());

        self.base
            .child(StyledText::new(self.label).with_default_highlights(&text_style, highlights))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ranges(text: &str, indices: &[usize]) -> Vec<Range<usize>> {
        highlight_ranges(text, indices, HighlightStyle::default())
            .into_iter()
            .map(|(r, _)| r)
            .collect()
    }

    #[test]
    fn empty_indices_produces_no_runs() {
        assert!(ranges("hello", &[]).is_empty());
    }

    #[test]
    fn single_index_produces_one_char_run() {
        assert_eq!(ranges("hello", &[1]), vec![1..2]);
    }

    #[test]
    fn contiguous_indices_merge_into_one_run() {
        assert_eq!(ranges("hello", &[0, 1, 2]), vec![0..3]);
    }

    #[test]
    fn discontiguous_indices_produce_separate_runs() {
        assert_eq!(ranges("hello", &[0, 2, 4]), vec![0..1, 2..3, 4..5]);
    }

    #[test]
    fn mixed_gaps_split_runs() {
        assert_eq!(ranges("abcdef", &[0, 1, 3, 4, 5]), vec![0..2, 3..6]);
    }

    #[test]
    fn multibyte_character_indices_advance_by_utf8_len() {
        let text = "héllo";
        assert_eq!(ranges(text, &[0, 1]), vec![0..3]);
        assert_eq!(ranges(text, &[1]), vec![1..3]);
    }

    #[test]
    fn adjacent_runs_over_multibyte_merge() {
        let text = "éé";
        assert_eq!(ranges(text, &[0, 2]), vec![0..4]);
    }

    #[test]
    fn final_index_at_last_byte_produces_trailing_run() {
        assert_eq!(ranges("abc", &[2]), vec![2..3]);
    }
}
