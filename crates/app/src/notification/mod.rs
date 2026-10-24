//! Notification center and popup UI.

mod card;
mod config;
mod panel;
pub(crate) mod popup;
mod widget;

pub use config::{NotificationConfig, NotificationPopupPosition};
pub use popup::init;
pub use widget::NotificationWidget;
