//! Layout helpers: pre-configured horizontal and vertical flex containers.
//!
//! Both helpers delegate to [`crate::traits::StyledExt`], so the
//! `div().h_flex()` form (used inside builders that already have a
//! `Div` in hand) and the bare-call `h_flex()` form produce identical
//! output.

use gpui::{Div, div};

use crate::traits::StyledExt;

/// Horizontal flex row with centered children. Equivalent to
/// `div().h_flex()` from [`StyledExt`].
#[track_caller]
pub fn h_flex() -> Div {
    div().h_flex()
}

/// Vertical flex column. Equivalent to `div().v_flex()` from
/// [`StyledExt`].
#[track_caller]
pub fn v_flex() -> Div {
    div().v_flex()
}
