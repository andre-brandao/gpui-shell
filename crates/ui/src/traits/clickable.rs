use gpui::{App, ClickEvent, CursorStyle, Window};

/// Elements that can be clicked.
pub trait Clickable {
    fn on_click(self, handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static) -> Self;
    /// Hover cursor. Defaults to [`CursorStyle::PointingHand`].
    fn cursor_style(self, cursor_style: CursorStyle) -> Self;
}
