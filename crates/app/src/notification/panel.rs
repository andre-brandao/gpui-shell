use gpui::prelude::*;
use gpui::{Context, MouseButton, Render, ScrollHandle, Window, div, px};
use services::{NotificationCommand, NotificationData, NotificationSubscriber};
use ui::patterns::PanelSurface;
use ui::{ActiveTheme, Color, Icon, IconSize, Radius, Spacing, TextSize};

use crate::config::ActiveConfig;
use crate::state::watch;
use crate::windows::ShellWindows;

use super::card::{activate_notification, is_critical, notification_card_body};

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
                    let clicked = item.clone();
                    div()
                        .relative()
                        .w_full()
                        .p(Spacing::Medium.pixels())
                        .rounded(Radius::Large.pixels())
                        .bg(theme.colors.background)
                        .border_1()
                        .border_color(if is_critical(&item) {
                            theme.colors.status.error
                        } else {
                            theme.colors.border
                        })
                        .when(item.has_default_action(), |el| {
                            el.cursor_pointer()
                                .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                    activate_notification(&clicked, cx);
                                    // Out of the way of the window it focused.
                                    ShellWindows::close_panel(cx);
                                })
                        })
                        .child(notification_card_body(
                            &item,
                            cx,
                            true,
                            false,
                            &self.subscriber,
                        ))
                        .child(
                            div()
                                .absolute()
                                .top(px(8.0))
                                .right(px(8.0))
                                .cursor_pointer()
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |_, _, _, cx| {
                                        cx.stop_propagation();
                                        dismiss_subscriber
                                            .dispatch(NotificationCommand::Dismiss(id));
                                    }),
                                )
                                .child(
                                    Icon::new(config.icons.close())
                                        .size(IconSize::XSmall)
                                        .color(Color::Muted),
                                ),
                        )
                }))
                .into_any_element()
        } else {
            div()
                .py(Spacing::XXXLarge.pixels())
                .text_size(TextSize::Small.rems())
                .text_color(theme.colors.text_muted)
                .text_center()
                .child("No notifications")
                .into_any_element()
        };

        div()
            .id("notification-center")
            .size_full()
            .panel_surface(cx)
            .p(Spacing::Medium.pixels())
            .flex()
            .flex_col()
            .gap(Spacing::Medium.pixels())
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_size(TextSize::Small.rems())
                            .text_color(theme.colors.text)
                            .child("Notifications"),
                    )
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(Spacing::Medium.pixels())
                            .child(
                                div()
                                    .cursor_pointer()
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(move |_, _, _, _cx| {
                                            dnd_subscriber.dispatch(NotificationCommand::SetDnd(
                                                !dnd_enabled,
                                            ));
                                        }),
                                    )
                                    .child(
                                        Icon::new(config.icons.dnd()).size(IconSize::XSmall).color(
                                            Color::Custom(if dnd_enabled {
                                                theme.colors.accent
                                            } else {
                                                theme.colors.text
                                            }),
                                        ),
                                    ),
                            )
                            .child(
                                div()
                                    .cursor_pointer()
                                    .text_size(TextSize::Small.rems())
                                    .text_color(theme.colors.text)
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(move |_, _, _, _cx| {
                                            clear_subscriber
                                                .dispatch(NotificationCommand::DismissAll);
                                        }),
                                    )
                                    .child("Clear"),
                            ),
                    ),
            )
            .child(
                // Sized by content so the panel can fit it; scrolls at the cap.
                div()
                    .id("notification-center-list")
                    .flex_grow_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll_handle)
                    .flex()
                    .flex_col()
                    .gap(Spacing::XSmall.pixels())
                    .child(list_content),
            )
    }
}
