mod messages;
mod service;

use gpui::App;

pub use service::IpcSubscriber;

impl IpcSubscriber {
    pub fn start(mut self, cx: &mut App) {
        let mut receiver = self.start_listener();

        cx.spawn(async move |cx| {
            // Keep the subscriber alive so the socket file isn't removed.
            let _ipc_guard = self;

            tracing::info!("IPC listener started");

            while let Some(message) = receiver.recv().await {
                cx.update(move |cx| {
                    messages::handle_message(message, cx);
                });
            }

            tracing::warn!("IPC listener ended unexpectedly");
        })
        .detach();
    }
}
