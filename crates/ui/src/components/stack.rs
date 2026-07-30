//! Layout helpers: pre-configured horizontal and vertical flex containers.

use gpui::{Div, div};

use crate::traits::StyledExt;

/// Horizontal flex row with centered children.
#[track_caller]
pub fn h_flex() -> Div {
    div().h_flex()
}

/// Vertical flex column.
#[track_caller]
pub fn v_flex() -> Div {
    div().v_flex()
}
