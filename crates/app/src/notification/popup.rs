use std::sync::Mutex;

use futures_signals::signal::SignalExt;
use futures_util::StreamExt;
use gpui::{
    AnyWindowHandle, App, Bounds, Context, Entity, MouseButton, Point, Render, Size, Window,
    WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, div, layer_shell::*,
    prelude::*, px,
};
use services::{Notification, NotificationCommand, NotificationSubscriber};
use ui::{ActiveTheme, font_size, radius, spacing};

use crate::config::ActiveConfig;
use crate::state::AppState;

use super::card::notification_card_body;
use super::config::NotificationConfig;
use super::dispatch_notification_command;

static POPUP_STATE: Mutex<Option<PopupWindowState>> = Mutex::new(None);

struct PopupWindowState {
    handle: AnyWindowHandle,
    view: Entity<NotificationPopupStack>,
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
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let theme = cx.theme();
        let config = &cx.config().notification;
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

                        div()
                            .h(px(config.popup_card_collapsed_height))
                            .overflow_hidden()
                            .bg(theme.bg.primary)
                            .border_1()
                            .border_color(theme.border.default)
                            .rounded(px(radius::LG))
                            .p(px(spacing::SM))
                            .hover(move |el| {
                                el.h(px(config.popup_card_expanded_height))
                                    .bg(theme.bg.elevated)
                                    .border_color(theme.accent.primary)
                            })
                            .child(notification_card_body(&notification, cx, true))
                            .child(
                                div()
                                    .absolute()
                                    .top(px(8.0))
                                    .right(px(8.0))
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
                                    .child(config.icons.close.clone()),
                            )
                    })),
            )
    }
}

fn popup_window_options(config: &NotificationConfig) -> WindowOptions {
    WindowOptions {
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: Point::new(px(0.), px(0.)),
            size: Size::new(px(config.popup_width), px(config.popup_height)),
        })),
        app_id: Some("gpuishell-notification-popup".to_string()),
        window_background: WindowBackgroundAppearance::Transparent,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "notification-popup".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::RIGHT,
            exclusive_zone: None,
            margin: Some((
                px(config.popup_margin_top),
                px(config.popup_margin_right),
                px(config.popup_margin_bottom),
                px(config.popup_margin_left),
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
    let config = cx.config().notification.clone();
    let notifications = subscriber.popup_notifications(config.popup_stack_limit);
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
    if let Ok(handle) = cx.open_window(popup_window_options(&config), move |_, cx| {
        cx.new(|_| NotificationPopupStack::new(sub, notifications))
    }) {
        let view = handle.update(cx, |_, _, cx| cx.entity().clone()).unwrap();
        *guard = Some(PopupWindowState {
            handle: handle.into(),
            view,
        });
    }
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
