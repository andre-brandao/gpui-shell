//! Scrollbar thumb geometry - shared by the list-like components that
//! overlay an engram-styled scrollbar ([`VirtualList`], [`VariableList`]).
//!
//! The generic `Scrollbar` component handles the axis-aware overflow case;
//! the list components need the same thumb math for a custom overlay that
//! sits alongside the list primitive (`gpui::uniform_list` or `gpui::list`).
//! Extracting [`ThumbMetrics`] here keeps both components' decoration path
//! and drag-handler path driving from a single source of truth - drift
//! between the two would mean a dragged thumb that doesn't track the
//! cursor.

use gpui::{Pixels, px};

pub(crate) const SCROLLBAR_THICKNESS: Pixels = px(10.0);
pub(crate) const SCROLLBAR_MIN_THUMB_RATIO: f32 = 0.08;

/// Thumb sizing + bidirectional mapping between thumb-top and content
/// scroll offset. Returns `None` when the viewport or content is empty,
/// or when the content fully fits inside the viewport (no scrollbar
/// needed).
#[derive(Clone, Copy)]
pub(crate) struct ThumbMetrics {
    pub thumb_h: f32,
    pub travel: f32,
    pub max_scroll: f32,
}

impl ThumbMetrics {
    pub(crate) fn compute(viewport: Pixels, content: Pixels) -> Option<Self> {
        let viewport = viewport.as_f32();
        let content = content.as_f32();
        let max_scroll = content - viewport;
        if viewport <= 0.0 || max_scroll <= 0.0 {
            return None;
        }
        let ratio = (viewport / content).clamp(SCROLLBAR_MIN_THUMB_RATIO, 1.0);
        let thumb_h = viewport * ratio;
        let travel = (viewport - thumb_h).max(0.0);
        Some(Self {
            thumb_h,
            travel,
            max_scroll,
        })
    }

    pub(crate) fn thumb_top_for_scroll(&self, scroll: f32) -> f32 {
        let r = (scroll / self.max_scroll).clamp(0.0, 1.0);
        self.travel * r
    }

    pub(crate) fn scroll_for_thumb_top(&self, thumb_top: Pixels) -> Pixels {
        if self.travel <= 0.0 {
            return px(0.0);
        }
        px((thumb_top.as_f32() / self.travel).clamp(0.0, 1.0) * self.max_scroll)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_returns_none_when_viewport_is_zero() {
        assert!(ThumbMetrics::compute(px(0.0), px(1000.0)).is_none());
    }

    #[test]
    fn compute_returns_none_when_content_fits_viewport() {
        // No overflow -> no scrollbar needed.
        assert!(ThumbMetrics::compute(px(500.0), px(500.0)).is_none());
        assert!(ThumbMetrics::compute(px(500.0), px(400.0)).is_none());
    }

    #[test]
    fn compute_sizes_thumb_proportionally_for_long_content() {
        // viewport = 200, content = 1000 -> ratio 0.2, thumb_h = 40.
        let m = ThumbMetrics::compute(px(200.0), px(1000.0)).unwrap();
        assert_eq!(m.thumb_h, 40.0);
        assert_eq!(m.travel, 160.0);
        assert_eq!(m.max_scroll, 800.0);
    }

    #[test]
    fn compute_enforces_minimum_thumb_ratio_for_very_long_content() {
        // viewport = 100, content = 100_000 -> natural ratio 0.001, floored to
        // SCROLLBAR_MIN_THUMB_RATIO (0.08) so the thumb stays grabbable.
        let m = ThumbMetrics::compute(px(100.0), px(100_000.0)).unwrap();
        assert_eq!(m.thumb_h, 100.0 * SCROLLBAR_MIN_THUMB_RATIO);
    }

    #[test]
    fn thumb_top_for_scroll_maps_extremes_to_track_ends() {
        let m = ThumbMetrics::compute(px(200.0), px(1000.0)).unwrap();
        assert_eq!(m.thumb_top_for_scroll(0.0), 0.0);
        assert_eq!(m.thumb_top_for_scroll(m.max_scroll), m.travel);
    }

    #[test]
    fn thumb_top_for_scroll_clamps_out_of_range_input() {
        let m = ThumbMetrics::compute(px(200.0), px(1000.0)).unwrap();
        assert_eq!(m.thumb_top_for_scroll(-100.0), 0.0);
        assert_eq!(m.thumb_top_for_scroll(m.max_scroll + 500.0), m.travel);
    }

    #[test]
    fn scroll_for_thumb_top_inverts_thumb_top_for_scroll() {
        let m = ThumbMetrics::compute(px(200.0), px(1000.0)).unwrap();
        for &s in &[0.0_f32, 100.0, 400.0, 800.0] {
            let thumb_top = px(m.thumb_top_for_scroll(s));
            let round_trip = m.scroll_for_thumb_top(thumb_top).as_f32();
            assert!(
                (round_trip - s).abs() < 1e-3,
                "scroll {s} round-tripped to {round_trip}"
            );
        }
    }

    #[test]
    fn scroll_for_thumb_top_clamps_out_of_range_input() {
        let m = ThumbMetrics::compute(px(200.0), px(1000.0)).unwrap();
        assert_eq!(m.scroll_for_thumb_top(px(-50.0)), px(0.0));
        let over = m.scroll_for_thumb_top(px(m.travel + 100.0));
        assert!((over.as_f32() - m.max_scroll).abs() < 1e-3);
    }
}
