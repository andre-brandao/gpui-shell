/// Elements whose interactivity and visual style can be suppressed.
///
/// Every component that can be disabled (buttons, list items, checkboxes,
/// switches, tabs, ...) implements this trait so callers can reach for the
/// same method name on every one of them. The convention is that
/// `.disabled(true)`:
///
/// - greys out the component's text / icons (via
///   [`Color::Disabled`](crate::prelude::Color::Disabled)),
/// - suppresses hover / pressed styling,
/// - and makes any stored click / toggle handler a no-op on paint.
///
/// Components that store a `disabled: bool` field don't need to branch on
/// it in their handler closures; they just gate the paint-time styling.
pub trait Disableable {
    fn disabled(self, disabled: bool) -> Self;
}
