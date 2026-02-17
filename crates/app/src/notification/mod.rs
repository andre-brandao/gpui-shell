//! Notification center and popup UI.

use std::sync::Mutex;

use futures_signals::signal::SignalExt;
use futures_util::StreamExt;
use gpui::{
    AnyWindowHandle, App, Bounds, Context, Entity, MouseButton, Point, Render, ScrollHandle, Size,
    StatefulInteractiveElement as _, Window, WindowBackgroundAppearance, WindowBounds, WindowKind,
    WindowOptions, div, img, layer_shell::*, prelude::*, px,
};
use services::{Notification, NotificationCommand, NotificationData, NotificationSubscriber};
use ui::{ActiveTheme, font_size, icon_size, radius, spacing};

use crate::bar::modules::WidgetSlot;
use crate::config::{ActiveConfig, Config};
use crate::panel::{PanelConfig, panel_placement, toggle_panel};
use crate::state::AppState;

const POPUP_WIDTH: f32 = 360.0;
const POPUP_HEIGHT: f32 = 320.0;
const POPUP_MARGIN: (f32, f32, f32, f32) = (42.0, 12.0, 0.0, 0.0);
const POPUP_STACK_LIMIT: usize = 4;
const POPUP_CARD_COLLAPSED_H: f32 = 92.0;
const POPUP_CARD_EXPANDED_H: f32 = 170.0;

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
    scroll_handle: ScrollHandle,
}

impl NotificationCenter {
    fn new(subscriber: NotificationSubscriber, cx: &mut Context<Self>) -> Self {
        let data = subscriber.get();
        let scroll_handle = ScrollHandle::new();
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
            subscriber,
            data,
            scroll_handle,
        }
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
                        .child(notification_card_body(&item, cx, true))
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
                                .child(icons::CLOSE),
                        )
                }))
                .into_any_element()
        } else {
            div()
                .py(px(spacing::XL))
                .text_size(px(font_size::SM))
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
                                    .child(icons::CLOSE),
                            )
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

fn image_source_from(value: &str) -> Option<String> {
    if is_image_source(value) {
        Some(value.to_string())
    } else {
        None
    }
}

fn is_image_source(value: &str) -> bool {
    value.starts_with('/')
        || value.starts_with("file://")
        || value.starts_with("http://")
        || value.starts_with("https://")
}

fn icon_fallback(app_name: &str, app_icon_name: &str) -> String {
    if !app_icon_name.is_empty() {
        return app_icon_name.chars().take(2).collect();
    }
    app_name
        .chars()
        .find(|c| c.is_alphanumeric())
        .map(|c| c.to_ascii_uppercase().to_string())
        .unwrap_or_else(|| "?".to_string())
}

fn format_notification_time(timestamp_ms: i64) -> String {
    use chrono::{Local, TimeZone};
    Local
        .timestamp_millis_opt(timestamp_ms)
        .single()
        .map(|dt| dt.format("%H:%M").to_string())
        .unwrap_or_default()
}

fn notification_card_body<V>(
    notification: &Notification,
    cx: &Context<V>,
    show_image: bool,
) -> gpui::AnyElement {
    let theme = cx.theme();
    let app_name = notification.app_name.clone();
    let app_icon_name = notification.app_icon.clone();
    let icon_source = notification
        .app_icon_path
        .clone()
        .or_else(|| image_source_from(&notification.app_icon));
    let image_source = notification
        .image_path
        .clone()
        .filter(|source| is_image_source(source));
    let summary = notification.summary.clone();
    let body = notification.body.clone();
    let timestamp = format_notification_time(notification.timestamp_ms);
    let urgency_color = urgency_color(notification.urgency, cx);

    div()
        .w_full()
        .flex()
        .items_start()
        .gap(px(spacing::SM))
        .child(
            div()
                .w(px(3.0))
                .h_full()
                .rounded(px(radius::SM))
                .bg(urgency_color),
        )
        .child(
            div()
                .size(px(28.0))
                .rounded(px(radius::SM))
                .bg(theme.bg.secondary)
                .border_1()
                .border_color(theme.border.subtle)
                .flex()
                .items_center()
                .justify_center()
                .child(
                    icon_source
                        .map(|src| {
                            img(src)
                                .size(px(18.0))
                                .rounded(px(radius::SM))
                                .into_any_element()
                        })
                        .unwrap_or_else(|| {
                            div()
                                .text_size(px(font_size::XS))
                                .text_color(theme.text.secondary)
                                .child(icon_fallback(&app_name, &app_icon_name))
                                .into_any_element()
                        }),
                ),
        )
        .child(
            div()
                .flex_1()
                .w_full()
                .pr(px(spacing::XL))
                .flex()
                .flex_col()
                .gap(px(3.0))
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
                                .text_size(px(font_size::XS))
                                .text_color(theme.text.muted)
                                .child(timestamp),
                        ),
                )
                .child(
                    div()
                        .whitespace_normal()
                        .text_size(px(font_size::SM))
                        .text_color(theme.text.primary)
                        .line_height(px(18.0))
                        .child(summary),
                )
                .when(!body.is_empty(), |el| {
                    el.child(
                        div()
                            .whitespace_normal()
                            .text_size(px(font_size::XS))
                            .text_color(theme.text.muted)
                            .line_height(px(16.0))
                            .child(body),
                    )
                })
                .when(show_image, |el| {
                    el.when_some(image_source, |el, source| {
                        el.child(
                            div()
                                .mt(px(4.0))
                                .h(px(56.0))
                                .max_w_full()
                                .rounded(px(radius::SM))
                                .overflow_hidden()
                                .child(img(source).h_full().w_auto()),
                        )
                    })
                }),
        )
        .into_any_element()
}

fn urgency_color<V>(urgency: u8, cx: &Context<V>) -> gpui::Hsla {
    let theme = cx.theme();
    match urgency {
        2 => theme.status.error,
        0 => theme.text.muted,
        _ => theme.accent.primary,
    }
}
