//! Notification center and popup UI.

use std::sync::Mutex;

use futures_signals::signal::SignalExt;
use futures_util::StreamExt;
use gpui::{
    AnyWindowHandle, App, Bounds, Context, Entity, MouseButton, Point, Render, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, div, layer_shell::*,
    prelude::*, px,
};
use services::{Notification, NotificationCommand, NotificationData, NotificationSubscriber};
use ui::{ActiveTheme, font_size, icon_size, radius, spacing};

use crate::bar::widgets::WidgetSlot;
use crate::config::{ActiveConfig, Config};
use crate::panel::{PanelConfig, panel_placement, toggle_panel};
use crate::state::AppState;

const POPUP_WIDTH: f32 = 360.0;
const POPUP_HEIGHT: f32 = 320.0;
const POPUP_MARGIN: (f32, f32, f32, f32) = (42.0, 12.0, 0.0, 0.0);
const POPUP_STACK_LIMIT: usize = 4;
const POPUP_CARD_COLLAPSED_H: f32 = 44.0;
const POPUP_CARD_EXPANDED_H: f32 = 102.0;

mod icons {
    pub const BELL: &str = "󰂚";
    pub const BELL_OFF: &str = "󰂛";
    pub const CLOSE: &str = "󰅖";
    pub const DND: &str = "󰂛";
}

static POPUP_STATE: Mutex<Option<PopupWindowState>> = Mutex::new(None);

struct PopupWindowState {
    handle: AnyWindowHandle,
    view: Entity<NotificationPopupStack>,
}

/// Notification widget for the bar.
pub struct NotificationWidget {
    slot: WidgetSlot,
    subscriber: NotificationSubscriber,
    data: NotificationData,
}

impl NotificationWidget {
    pub fn new(slot: WidgetSlot, cx: &mut Context<Self>) -> Self {
        let subscriber = AppState::services(cx).notification.clone();
        let data = subscriber.get();

        cx.spawn({
            let mut signal = subscriber.subscribe().to_stream();
            async move |this, cx| {
                while let Some(data) = signal.next().await {
                    let ok = this
                        .update(cx, |this, cx| {
                            this.data = data;
                            cx.notify();
                        })
                        .is_ok();
                    if !ok {
                        break;
                    }
                }
            }
        })
        .detach();

        Self {
            slot,
            subscriber,
            data,
        }
    }

    fn toggle_center(&self, cx: &mut App) {
        let config = Config::global(cx);
        let (anchor, margin) = panel_placement(config.bar.position, self.slot);
        let subscriber = self.subscriber.clone();
        dispatch_notification_command(subscriber.clone(), NotificationCommand::MarkAllRead);

        let panel_config = PanelConfig {
            width: 420.0,
            height: 540.0,
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
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
                cx.listener(|this, _, _, cx| this.toggle_center(cx)),
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
                        icons::BELL_OFF
                    } else {
                        icons::BELL
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

struct NotificationCenter {
    subscriber: NotificationSubscriber,
    data: NotificationData,
}

impl NotificationCenter {
    fn new(subscriber: NotificationSubscriber, cx: &mut Context<Self>) -> Self {
        let data = subscriber.get();
        cx.spawn({
            let mut signal = subscriber.subscribe().to_stream();
            async move |this, cx| {
                while let Some(data) = signal.next().await {
                    let ok = this
                        .update(cx, |this, cx| {
                            this.data = data;
                            cx.notify();
                        })
                        .is_ok();
                    if !ok {
                        break;
                    }
                }
            }
        })
        .detach();

        Self { subscriber, data }
    }
}

impl Render for NotificationCenter {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let notifications = self.data.notifications.clone();
        let has_notifications = !notifications.is_empty();
        let dnd_enabled = self.data.dnd;
        let dnd_subscriber = self.subscriber.clone();
        let clear_subscriber = self.subscriber.clone();

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
                            .text_size(px(font_size::SM))
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
                                    .text_size(px(font_size::SM))
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
                                    .child(icons::DND),
                            )
                            .child(
                                div()
                                    .cursor_pointer()
                                    .text_size(px(font_size::SM))
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
                    .flex_1()
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::XS))
                    .when(!has_notifications, |el| {
                        el.child(
                            div()
                                .py(px(spacing::XL))
                                .text_size(px(font_size::SM))
                                .text_color(theme.text.muted)
                                .text_center()
                                .child("No notifications"),
                        )
                    })
                    .children(notifications.into_iter().map(|item| {
                        let dismiss_subscriber = self.subscriber.clone();
                        let id = item.id;
                        let app_name = item.app_name.clone();
                        let summary = item.summary.clone();
                        let body = item.body.clone();
                        div()
                            .flex()
                            .items_start()
                            .gap(px(spacing::SM))
                            .p(px(spacing::SM))
                            .rounded(px(radius::SM))
                            .bg(theme.bg.secondary)
                            .border_1()
                            .border_color(theme.border.subtle)
                            .child(
                                div()
                                    .flex_1()
                                    .flex()
                                    .flex_col()
                                    .gap(px(4.0))
                                    .child(
                                        div()
                                            .text_size(px(font_size::XS))
                                            .text_color(theme.text.secondary)
                                            .child(app_name),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(font_size::SM))
                                            .text_color(theme.text.primary)
                                            .child(summary),
                                    )
                                    .when(!body.is_empty(), |el| {
                                        el.child(
                                            div()
                                                .text_size(px(font_size::XS))
                                                .text_color(theme.text.muted)
                                                .child(body.clone()),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .cursor_pointer()
                                    .text_size(px(font_size::SM))
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
                                    .child(icons::CLOSE),
                            )
                    })),
            )
    }
}

struct NotificationPopupStack {
    subscriber: NotificationSubscriber,
    notifications: Vec<Notification>,
}

impl NotificationPopupStack {
    fn new(subscriber: NotificationSubscriber, notifications: Vec<Notification>) -> Self {
        Self {
            subscriber,
            notifications,
        }
    }
}

impl Render for NotificationPopupStack {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let items = self.notifications.clone();

        div()
            .id("notification-popup-stack")
            .size_full()
            .p(px(spacing::SM))
            .child(
                div()
                    .size_full()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::XS))
                    .children(items.into_iter().map(|notification| {
                        let dismiss_subscriber = self.subscriber.clone();
                        let id = notification.id;
                        let app_name = notification.app_name.clone();
                        let summary = notification.summary.clone();
                        let body = notification.body.clone();

                        div()
                            .h(px(POPUP_CARD_COLLAPSED_H))
                            .overflow_hidden()
                            .bg(theme.bg.primary)
                            .border_1()
                            .border_color(theme.border.default)
                            .rounded(px(radius::LG))
                            .p(px(spacing::SM))
                            .hover(move |el| {
                                el.h(px(POPUP_CARD_EXPANDED_H))
                                    .bg(theme.bg.elevated)
                                    .border_color(theme.accent.primary)
                            })
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .text_size(px(font_size::XS))
                                            .text_color(theme.text.secondary)
                                            .child(app_name),
                                    )
                                    .child(
                                        div()
                                            .cursor_pointer()
                                            .text_size(px(font_size::SM))
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
                                            .child(icons::CLOSE),
                                    ),
                            )
                            .child(
                                div()
                                    .text_size(px(font_size::SM))
                                    .text_color(theme.text.primary)
                                    .child(summary),
                            )
                            .when(!body.is_empty(), |el| {
                                el.child(
                                    div()
                                        .text_size(px(font_size::XS))
                                        .text_color(theme.text.muted)
                                        .line_height(px(16.0))
                                        .child(body),
                                )
                            })
                    })),
            )
    }
}

fn popup_window_options() -> WindowOptions {
    WindowOptions {
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: Point::new(px(0.), px(0.)),
            size: Size::new(px(POPUP_WIDTH), px(POPUP_HEIGHT)),
        })),
        app_id: Some("gpuishell-notification-popup".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "notification-popup".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::RIGHT,
            exclusive_zone: None,
            margin: Some((
                px(POPUP_MARGIN.0),
                px(POPUP_MARGIN.1),
                px(POPUP_MARGIN.2),
                px(POPUP_MARGIN.3),
            )),
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        focus: false,
        ..Default::default()
    }
}

fn close_popup(cx: &mut App) {
    let mut guard = POPUP_STATE.lock().unwrap();
    if let Some(state) = guard.take() {
        let _ = cx.update_window(state.handle, |_, window, _cx| {
            window.remove_window();
        });
    }
}

fn sync_popup(subscriber: &NotificationSubscriber, cx: &mut App) {
    let notifications = subscriber.popup_notifications(POPUP_STACK_LIMIT);
    if notifications.is_empty() {
        close_popup(cx);
        return;
    }

    let mut guard = POPUP_STATE.lock().unwrap();
    if let Some(existing) = guard.as_ref() {
        let view = existing.view.clone();
        let handle = existing.handle;
        let updated = cx
            .update_window(handle, |_, _window, cx| {
                view.update(cx, |popup, cx| {
                    popup.notifications = notifications.clone();
                    cx.notify();
                });
            })
            .is_ok();
        if updated {
            return;
        }
    }

    let sub = subscriber.clone();
    if let Ok(handle) = cx.open_window(popup_window_options(), move |_, cx| {
        cx.new(|_| NotificationPopupStack::new(sub, notifications))
    }) {
        let view = handle.update(cx, |_, _, cx| cx.entity().clone()).unwrap();
        *guard = Some(PopupWindowState {
            handle: handle.into(),
            view,
        });
    }
}

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

pub fn init(cx: &mut App) {
    let subscriber = AppState::services(cx).notification.clone();
    cx.spawn({
        let mut signal = subscriber.subscribe().to_stream();
        let service = subscriber.clone();
        async move |cx| {
            while signal.next().await.is_some() {
                cx.update(|cx| sync_popup(&service, cx));
            }
        }
    })
    .detach();
}
