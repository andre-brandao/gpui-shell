use std::collections::HashSet;
use std::time::Duration;

use gpui::App;
use services::{
    Notification, NotificationCloseReason, NotificationData, NotificationEvent,
    NotificationSubscriber, NotificationTimeout,
};

fn notification_key(notification: &Notification) -> (u32, u64) {
    (notification.id, notification.revision)
}

#[derive(Default)]
pub(super) struct PopupController {
    active: Vec<(u32, u64)>,
    scheduled: HashSet<(u32, u64)>,
}

impl PopupController {
    pub(super) fn handle_event(
        &mut self,
        event: NotificationEvent,
        data: &NotificationData,
        subscriber: &NotificationSubscriber,
        cx: &mut App,
    ) {
        self.prune(data);

        match event {
            NotificationEvent::Added(notification) | NotificationEvent::Replaced(notification) => {
                if data.dnd || !notification.state.is_open() {
                    return;
                }

                let key = notification_key(&notification);
                self.active.retain(|active| active.0 != notification.id);
                self.active.insert(0, key);

                if self.scheduled.insert(key) {
                    schedule_expiration_timer(subscriber, notification, cx);
                }
            }
            NotificationEvent::Closed { id, revision, .. } => {
                self.active
                    .retain(|active| active.0 != id || active.1 != revision);
            }
            NotificationEvent::Removed(id) => {
                self.active.retain(|active| active.0 != id);
            }
            NotificationEvent::DndChanged(enabled) => {
                if enabled {
                    self.active.clear();
                }
            }
        }
    }

    pub(super) fn active_notifications(
        &self,
        data: &NotificationData,
        limit: usize,
    ) -> Vec<Notification> {
        self.active
            .iter()
            .filter_map(|(id, revision)| {
                data.notifications
                    .iter()
                    .find(|notification| {
                        notification.id == *id
                            && notification.revision == *revision
                            && notification.state.is_open()
                    })
                    .cloned()
            })
            .take(limit)
            .collect()
    }

    fn prune(&mut self, data: &NotificationData) {
        let open_keys: HashSet<(u32, u64)> = data
            .notifications
            .iter()
            .filter(|notification| notification.state.is_open())
            .map(notification_key)
            .collect();

        self.active.retain(|key| open_keys.contains(key));
        self.scheduled.retain(|key| open_keys.contains(key));
    }
}

fn schedule_expiration_timer(
    subscriber: &NotificationSubscriber,
    notification: Notification,
    cx: &mut App,
) {
    let timeout = match notification.timeout {
        NotificationTimeout::Default => Duration::from_secs(5),
        NotificationTimeout::Never => return,
        NotificationTimeout::Millis(ms) => Duration::from_millis(ms),
    };

    let service = subscriber.clone();
    cx.spawn(async move |cx| {
        cx.background_executor().timer(timeout).await;
        let _ = service
            .close_notification(
                notification.id,
                notification.revision,
                NotificationCloseReason::Expired,
            )
            .await;
    })
    .detach();
}
