use gpui::prelude::*;
use gpui::{Context, MouseButton, Render, ScrollHandle, Window, div, px};
use services::{NotificationCommand, NotificationData, NotificationSubscriber};
use ui::{ActiveTheme, radius, spacing};

use crate::config::ActiveConfig;
use crate::state::watch;

use super::card::notification_card_body;
use super::dispatch_notification_command;

pub(super) struct NotificationCenter {
    subscriber: NotificationSubscriber,
    data: NotificationData,
    scroll_handle: ScrollHandle,
}

impl NotificationCenter {
    pub(super) fn new(subscriber: NotificationSubscriber, cx: &mut Context<Self>) -> Self {
        let data = subscriber.get();
        let scroll_handle = ScrollHandle::new();
        watch(cx, subscriber.subscribe(), |this, data, cx| {
            this.data = data;
            cx.notify();
        });

        Self {
            subscriber,
            data,
            scroll_handle,
        }
    }
}

impl Render for NotificationCenter {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let theme = cx.theme();
        let config = &cx.config().notification;
        let notifications = self.data.notifications.clone();
        let has_notifications = !notifications.is_empty();
        let dnd_enabled = self.data.dnd;
        let dnd_subscriber = self.subscriber.clone();
        let clear_subscriber = self.subscriber.clone();
        let list_content = if has_notifications {
            div()
                .children(notifications.into_iter().map(|item| {
                    let dismiss_subscriber = self.subscriber.clone();
                    let id = item.id;
                    div()
                        .relative()
                        .w_full()
                        .p(px(spacing::SM))
                        .rounded(px(radius::LG))
                        .bg(theme.bg.primary)
                        .border_1()
                        .border_color(theme.border.default)
                        .child(notification_card_body(&item, cx, true, &self.subscriber))
                        .child(
                            div()
                                .absolute()
                                .top(px(8.0))
                                .right(px(8.0))
                                .cursor_pointer()
                                .text_size(theme.font_sizes.sm)
                                .text_color(theme.text.muted)
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |_, _, _, _cx| {
                                        dispatch_notification_command(
                                            dismiss_subscriber.clone(),
                                            NotificationCommand::Dismiss(id),
                                        );
                                    }),
                                )
                                .child(config.icons.close.clone()),
                        )
                }))
                .into_any_element()
        } else {
            div()
                .py(px(spacing::XL))
                .text_size(theme.font_sizes.sm)
                .text_color(theme.text.muted)
                .text_center()
                .child("No notifications")
                .into_any_element()
        };

        div()
            .id("notification-center")
            .size_full()
            .bg(theme.bg.primary)
            .border_1()
            .border_color(theme.border.default)
            .rounded(px(radius::LG))
            .p(px(spacing::SM))
            .flex()
            .flex_col()
            .gap(px(spacing::SM))
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(theme.font_sizes.sm)
                            .text_color(theme.text.primary)
                            .child("Notifications"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(spacing::SM))
                            .child(
                                div()
                                    .cursor_pointer()
                                    .text_size(theme.font_sizes.sm)
                                    .text_color(if dnd_enabled {
                                        theme.accent.primary
                                    } else {
                                        theme.text.secondary
                                    })
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(move |_, _, _, _cx| {
                                            dispatch_notification_command(
                                                dnd_subscriber.clone(),
                                                NotificationCommand::SetDnd(!dnd_enabled),
                                            );
                                        }),
                                    )
                                    .child(config.icons.dnd.clone()),
                            )
                            .child(
                                div()
                                    .cursor_pointer()
                                    .text_size(theme.font_sizes.sm)
                                    .text_color(theme.text.secondary)
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(move |_, _, _, _cx| {
                                            dispatch_notification_command(
                                                clear_subscriber.clone(),
                                                NotificationCommand::DismissAll,
                                            );
                                        }),
                                    )
                                    .child("Clear"),
                            ),
                    ),
            )
            .child(
                div()
                    .id("notification-center-list")
                    .flex_1()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll_handle)
                    .flex()
                    .flex_col()
                    .gap(px(spacing::XS))
                    .child(list_content),
            )
    }
}
