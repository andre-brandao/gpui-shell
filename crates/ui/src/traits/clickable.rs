use gpui::{App, ClickEvent, CursorStyle, Window};

/// Elements that can be clicked.
///
/// Mirrors GPUI's own click-handler signature so components can forward
/// caller closures directly to the underlying `InteractiveElement::on_click`
/// without an adapter layer. The trait exists so every click-able engram
/// component exposes the same `.on_click(...)` method, regardless of
/// whether internally it stores the handler as `on_toggle` (e.g.
/// [`Disclosure`](crate::components::disclosure::Disclosure)) or
/// `on_pressed` or anything else.
pub trait Clickable {
    fn on_click(self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self;
    /// Sets the cursor style shown while hovering the element. Defaults to
    /// [`CursorStyle::PointingHand`] for every current implementor; the hook
    /// exists so callers can downgrade to e.g. [`CursorStyle::Arrow`] for
    /// chrome that should not advertise itself as clickable.
    fn cursor_style(self, cursor_style: CursorStyle) -> Self;
}
