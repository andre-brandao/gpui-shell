use gpui::{AnyElement, App, Context, MouseButton, Render, Window, div, prelude::*, px};
use services::{NotificationCommand, NotificationData, NotificationSubscriber};
use ui::ActiveTheme;

use crate::bar::modules::{BarWidget, style};
use crate::config::{ActiveConfig, Config};
use crate::panel::toggle_widget_panel;
use crate::state::{AppState, watch};

use super::center::NotificationCenter;
use super::dispatch_notification_command;

/// Notification widget for the bar.
pub struct NotificationWidget {
    subscriber: NotificationSubscriber,
    data: NotificationData,
}

impl NotificationWidget {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let subscriber = AppState::notification(cx).clone();
        let data = subscriber.get();

        watch(cx, subscriber.subscribe(), |this, data, cx| {
            this.data = data;
            cx.notify();
        });

        Self { subscriber, data }
    }

    fn toggle_center(&self, event: &gpui::MouseDownEvent, window: &Window, cx: &mut App) {
        let config = Config::global(cx);
        let center_width = config.notification.center_width;
        let center_height = config.notification.center_height;
        let subscriber = self.subscriber.clone();
        dispatch_notification_command(subscriber.clone(), NotificationCommand::MarkAllRead, cx);

        toggle_widget_panel(
            "notification-center",
            gpui::Size::new(center_width, center_height),
            "notification-center",
            event,
            window,
            cx,
            move |cx| NotificationCenter::new(subscriber, cx),
        );
    }

    fn render_widget_content(
        &self,
        theme: &ui::Theme,
        icon: String,
        is_vertical: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let unread = self.data.unread_count;
        let badge_color = theme.accent.primary;

        div()
            .id("notification-widget")
            .flex()
            .when(is_vertical, |el| el.flex_col())
            .items_center()
            .justify_center()
            .gap(px(style::CHIP_GAP))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event, window, cx| this.toggle_center(event, window, cx)),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .text_size(px(style::icon(is_vertical)))
                    .text_color(if self.data.dnd {
                        theme.text.muted
                    } else {
                        theme.text.primary
                    })
                    .child(icon),
            )
            .when(unread > 0, |el| {
                el.child(
                    div()
                        .flex_shrink_0()
                        .text_size(theme.font_sizes.xs)
                        .text_color(badge_color)
                        .child(unread.to_string()),
                )
            })
            .into_any_element()
    }
}

impl BarWidget for NotificationWidget {
    fn is_interactive(&self) -> bool {
        true
    }

    fn render_vertical(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let config = &cx.config().notification;
        let icon = if self.data.dnd {
            config.icons.bell_off.clone()
        } else {
            config.icons.bell.clone()
        };
        self.render_widget_content(&theme, icon, true, cx)
    }

    fn render_horizontal(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        let config = &cx.config().notification;
        let icon = if self.data.dnd {
            config.icons.bell_off.clone()
        } else {
            config.icons.bell.clone()
        };
        self.render_widget_content(&theme, icon, false, cx)
    }
}

impl Render for NotificationWidget {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        self.render_bar_widget(window, cx)
    }
}
