use gpui::prelude::*;
use gpui::{Context, MouseButton, div, img, px};
use services::{Notification, NotificationCommand, NotificationSubscriber};
use ui::{ActiveTheme, font_size, radius, spacing};

use super::dispatch_notification_command;

pub(super) fn notification_card_body<V>(
    notification: &Notification,
    cx: &Context<V>,
    show_image: bool,
    subscriber: &NotificationSubscriber,
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
    let actions = notification.actions.clone();
    let timestamp = format_notification_time(notification.timestamp_ms);
    let urgency_color = urgency_color(notification.urgency, cx);

    div()
        .w_full()
        .flex()
        .items_stretch()
        .gap(px(spacing::SM))
        .child(
            div()
                .w(px(3.0))
                .flex_shrink_0()
                .rounded(px(radius::SM))
                .bg(urgency_color),
        )
        .child(
            div()
                .size(px(28.0))
                .flex_shrink_0()
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
                .when(!actions.is_empty(), |el| {
                    el.child(
                        div()
                            .mt(px(4.0))
                            .flex()
                            .flex_wrap()
                            .gap(px(spacing::XS))
                            .children(actions.into_iter().map(|(key, label)| {
                                let sub = subscriber.clone();
                                let notification_id = notification.id;
                                div()
                                    .px(px(spacing::SM))
                                    .py(px(2.0))
                                    .rounded(px(radius::SM))
                                    .bg(theme.bg.tertiary)
                                    .border_1()
                                    .border_color(theme.border.subtle)
                                    .text_size(px(font_size::XS))
                                    .text_color(theme.text.secondary)
                                    .cursor_pointer()
                                    .hover(move |el| {
                                        el.bg(theme.interactive.hover)
                                            .text_color(theme.text.primary)
                                    })
                                    .on_mouse_down(MouseButton::Left, move |_, _, _| {
                                        dispatch_notification_command(
                                            sub.clone(),
                                            NotificationCommand::InvokeAction(
                                                notification_id,
                                                key.clone(),
                                            ),
                                        );
                                    })
                                    .child(label)
                            })),
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

fn urgency_color<V>(urgency: u8, cx: &Context<V>) -> gpui::Hsla {
    let theme = cx.theme();
    match urgency {
        2 => theme.status.error,
        0 => theme.text.muted,
        _ => theme.accent.primary,
    }
}
