//! Notification center and popup UI.

mod card;
mod config;
mod pannel;
mod popup;
mod widget;

pub use config::{NotificationConfig, NotificationPopupPosition};
pub use popup::init;
pub use widget::NotificationWidget;

use services::{NotificationCommand, NotificationSubscriber};

fn dispatch_notification_command(subscriber: NotificationSubscriber, command: NotificationCommand) {
    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to create tokio runtime for notification command");
        rt.block_on(async move {
            let _ = subscriber.dispatch(command).await;
        });
    });
}
