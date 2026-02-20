use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use futures_signals::signal::SignalExt;
use futures_util::StreamExt;
use gpui::{
    AnyWindowHandle, App, Bounds, Context, DisplayId, Entity, MouseButton, Point, Render, Size,
    Window, WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, div,
    layer_shell::*, prelude::*, px,
};
use services::{Notification, NotificationCommand, NotificationSubscriber};
use ui::{ActiveTheme, font_size, radius, spacing};

use crate::config::ActiveConfig;
use crate::state::AppState;

use super::card::notification_card_body;
use super::config::{NotificationConfig, NotificationPopupPosition};
use super::dispatch_notification_command;

static POPUP_STATE: LazyLock<Mutex<HashMap<Option<DisplayId>, PopupWindowState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

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
                            .overflow_hidden()
                            .bg(theme.bg.primary)
                            .border_1()
                            .border_color(theme.border.default)
                            .rounded(px(radius::LG))
                            .p(px(spacing::SM))
                            .hover(move |el| {
                                el.bg(theme.interactive.hover)
                                    .border_color(theme.accent.primary)
                            })
                            .child(notification_card_body(
                                &notification,
                                cx,
                                false,
                                &self.subscriber,
                            ))
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

fn popup_window_options(config: &NotificationConfig, display_id: Option<DisplayId>) -> WindowOptions {
    let margin = popup_margin(config);
    WindowOptions {
        display_id,
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
            anchor: popup_anchor(config.popup_position),
            exclusive_zone: None,
            margin: Some((px(margin.0), px(margin.1), px(margin.2), px(margin.3))),
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        focus: false,
        ..Default::default()
    }
}

fn popup_anchor(position: NotificationPopupPosition) -> Anchor {
    match position {
        NotificationPopupPosition::TopLeft => Anchor::TOP | Anchor::LEFT,
        NotificationPopupPosition::TopRight => Anchor::TOP | Anchor::RIGHT,
        NotificationPopupPosition::BottomLeft => Anchor::BOTTOM | Anchor::LEFT,
        NotificationPopupPosition::BottomRight => Anchor::BOTTOM | Anchor::RIGHT,
    }
}

fn popup_margin(config: &NotificationConfig) -> (f32, f32, f32, f32) {
    match config.popup_position {
        NotificationPopupPosition::TopLeft => {
            (config.popup_margin_top, 0.0, 0.0, config.popup_margin_left)
        }
        NotificationPopupPosition::TopRight => {
            (config.popup_margin_top, config.popup_margin_right, 0.0, 0.0)
        }
        NotificationPopupPosition::BottomLeft => (
            0.0,
            0.0,
            config.popup_margin_bottom,
            config.popup_margin_left,
        ),
        NotificationPopupPosition::BottomRight => (
            0.0,
            config.popup_margin_right,
            config.popup_margin_bottom,
            0.0,
        ),
    }
}

fn popup_targets(cx: &App) -> Vec<Option<DisplayId>> {
    let displays = cx.displays();
    if displays.is_empty() {
        vec![None]
    } else {
        displays.into_iter().map(|display| Some(display.id())).collect()
    }
}

fn close_popups(cx: &mut App) {
    let mut guard = POPUP_STATE.lock().unwrap();
    for (_, state) in guard.drain() {
        let _ = cx.update_window(state.handle, |_, window, _cx| {
            window.remove_window();
        });
    }
}

fn sync_popup(subscriber: &NotificationSubscriber, cx: &mut App) {
    let config = cx.config().notification.clone();
    let notifications = subscriber.popup_notifications(config.popup_stack_limit);
    if notifications.is_empty() {
        close_popups(cx);
        return;
    }

    let targets = popup_targets(cx);
    let mut guard = POPUP_STATE.lock().unwrap();

    guard.retain(|display_id, state| {
        if targets.contains(display_id) {
            true
        } else {
            let _ = cx.update_window(state.handle, |_, window, _cx| {
                window.remove_window();
            });
            false
        }
    });

    for display_id in targets {
        if let Some(existing) = guard.get(&display_id) {
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
                continue;
            }
        }

        guard.remove(&display_id);
        let sub = subscriber.clone();
        let notifications = notifications.clone();
        if let Ok(handle) =
            cx.open_window(popup_window_options(&config, display_id), move |_, cx| {
                cx.new(|_| NotificationPopupStack::new(sub, notifications))
            })
        {
            let view = handle.update(cx, |_, _, cx| cx.entity().clone()).unwrap();
            guard.insert(
                display_id,
                PopupWindowState {
                    handle: handle.into(),
                    view,
                },
            );
        }
    }
}

pub fn init(cx: &mut App) {
    let subscriber = AppState::notification(cx).clone();
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
