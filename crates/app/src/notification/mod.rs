//! Notification center and popup UI.

mod card;
mod center;
mod config;
mod popup;
mod popup_controller;
mod widget;

pub use config::{NotificationConfig, NotificationPopupPosition};
pub use popup::init;
pub use widget::NotificationWidget;

use gpui::App;
use services::{NotificationCommand, NotificationSubscriber};

fn dispatch_notification_command(
    subscriber: NotificationSubscriber,
    command: NotificationCommand,
    cx: &mut App,
) {
    cx.spawn(async move |_| {
        let _ = subscriber.dispatch(command).await;
    })
    .detach();
}
