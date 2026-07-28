//! Group - convenience constructors for flex containers with consistent
//! spacing. Lighter weight than [`h_flex`](super::stack::h_flex) /
//! [`v_flex`](super::stack::v_flex) which provide no gap.

use gpui::{Div, div, prelude::*};

use crate::theme::Spacing;

use crate::traits::StyledExt as _;

/// Horizontal group with small (4px) gap.
pub fn h_group() -> Div {
    div().flex().gap(Spacing::Small.pixels())
}

/// Horizontal group with medium (8px) gap.
pub fn h_group_lg() -> Div {
    div().flex().gap(Spacing::Medium.pixels())
}

/// Vertical group with small (4px) gap.
pub fn v_group() -> Div {
    div().v_flex().gap(Spacing::Small.pixels())
}

/// Vertical group with medium (8px) gap.
pub fn v_group_lg() -> Div {
    div().v_flex().gap(Spacing::Medium.pixels())
}
