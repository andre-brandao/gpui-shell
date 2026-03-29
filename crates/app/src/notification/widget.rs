use gpui::{AnyElement, App, Context, MouseButton, Render, Size, Window, div, prelude::*, px};
use services::{NotificationCommand, NotificationData, NotificationSubscriber};
use ui::ActiveTheme;

use crate::bar::modules::{BarWidget, style};
use crate::config::{ActiveConfig, Config};
use crate::panel::{PanelConfig, panel_placement_from_event, toggle_panel};
use crate::state::{AppState, watch};

use super::dispatch_notification_command;
use super::pannel::NotificationCenter;

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
        let notification_config = &config.notification;
        let panel_size = Size::new(
            px(notification_config.center_width),
            px(notification_config.center_height),
        );
        let (anchor, margin) =
            panel_placement_from_event(config.bar.position, event, window, cx, panel_size);
        let subscriber = self.subscriber.clone();
        dispatch_notification_command(subscriber.clone(), NotificationCommand::MarkAllRead);

        let panel_config = PanelConfig {
            width: notification_config.center_width,
            height: notification_config.center_height,
            anchor,
            margin,
            namespace: "notification-center".to_string(),
        };

        toggle_panel("notification-center", panel_config, cx, move |cx| {
            NotificationCenter::new(subscriber, cx)
        });
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
            .gap(px(style::CHIP_GAP))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event, window, cx| this.toggle_center(event, window, cx)),
            )
            .child(
                div()
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
