use gpui::{App, Context, MouseButton, Render, Window, div, prelude::*, px, Size};
use services::{NotificationCommand, NotificationData, NotificationSubscriber};
use ui::{ActiveTheme, font_size, icon_size, radius, spacing};

use crate::bar::modules::WidgetSlot;
use crate::config::{ActiveConfig, Config};
use crate::panel::{PanelConfig, panel_placement_from_event, toggle_panel};
use crate::state::{AppState, watch};

use super::dispatch_notification_command;
use super::pannel::NotificationCenter;

/// Notification widget for the bar.
pub struct NotificationWidget {
    slot: WidgetSlot,
    subscriber: NotificationSubscriber,
    data: NotificationData,
}

impl NotificationWidget {
    pub fn new(slot: WidgetSlot, cx: &mut Context<Self>) -> Self {
        let subscriber = AppState::notification(cx).clone();
        let data = subscriber.get();

        watch(cx, subscriber.subscribe(), |this, data, cx| {
            this.data = data;
            cx.notify();
        });

        Self {
            slot,
            subscriber,
            data,
        }
    }

    fn toggle_center(
        &self,
        event: &gpui::MouseDownEvent,
        window: &Window,
        cx: &mut App,
    ) {
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
}

impl Render for NotificationWidget {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let theme = cx.theme();
        let config = &cx.config().notification;
        let is_vertical = cx.config().bar.is_vertical();
        let unread = self.data.unread_count;

        let interactive_default = theme.interactive.default;
        let interactive_hover = theme.interactive.hover;
        let interactive_active = theme.interactive.active;
        let text_primary = theme.text.primary;
        let text_muted = theme.text.muted;
        let badge_color = theme.accent.primary;

        div()
            .id("notification-widget")
            .flex()
            .when(is_vertical, |this| this.flex_col())
            .items_center()
            .gap(px(spacing::XS))
            .px(px(spacing::XS))
            .py(px(3.0))
            .rounded(px(radius::SM))
            .cursor_pointer()
            .bg(interactive_default)
            .hover(move |el| el.bg(interactive_hover))
            .active(move |el| el.bg(interactive_active))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event, window, cx| this.toggle_center(event, window, cx)),
            )
            .child(
                div()
                    .text_size(px(if is_vertical {
                        icon_size::MD
                    } else {
                        icon_size::LG
                    }))
                    .text_color(if self.data.dnd {
                        text_muted
                    } else {
                        text_primary
                    })
                    .child(if self.data.dnd {
                        config.icons.bell_off.clone()
                    } else {
                        config.icons.bell.clone()
                    }),
            )
            .when(unread > 0, |el| {
                el.child(
                    div()
                        .text_size(px(font_size::XS))
                        .text_color(badge_color)
                        .child(unread.to_string()),
                )
            })
    }
}
