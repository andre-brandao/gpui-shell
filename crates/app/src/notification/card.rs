use gpui::prelude::*;
use gpui::{Context, MouseButton, div, img, px};
use services::{Notification, NotificationCommand, NotificationSubscriber};
use ui::{ActiveTheme, radius, spacing};

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
    let icon_source = notification.app_icon_path.as_ref().map(|p| {
        tracing::debug!(
            "Loading icon for {}: {} -> {:?}",
            app_name,
            notification.app_icon,
            p
        );
        p.clone()
    });
    let image_source = notification.image_path.clone();
    let summary = notification.summary.clone();
    let body = notification.body.clone();
    let actions = notification.actions.clone();
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
                .flex_shrink_0()
                .rounded(px(radius::SM))
                .bg(urgency_color)
                .h_full(),
        )
        .child(
            // Icon/Image area: Show image if available, otherwise show app icon or fallback
            match (&icon_source, &image_source, show_image) {
                // If we have both icon and image, show a larger image with small icon overlay
                (Some(icon), Some(img_src), true) => div()
                    .w(px(64.0))
                    .h(px(64.0))
                    .flex_shrink_0()
                    .rounded(px(radius::MD))
                    .overflow_hidden()
                    .relative()
                    .child(
                        img(img_src.clone())
                            .size_full()
                            .object_fit(gpui::ObjectFit::Cover),
                    )
                    .child(
                        div()
                            .absolute()
                            .bottom(px(2.0))
                            .right(px(2.0))
                            .size(px(20.0))
                            .rounded(px(radius::SM))
                            .bg(theme.bg.primary)
                            .border_1()
                            .border_color(theme.border.default)
                            .flex()
                            .items_center()
                            .justify_center()
                            .child(
                                img(icon.clone())
                                    .size(px(14.0))
                                    .object_fit(gpui::ObjectFit::Contain),
                            ),
                    )
                    .into_any_element(),
                // If only image, show larger image
                (_, Some(img_src), true) => div()
                    .w(px(64.0))
                    .h(px(64.0))
                    .flex_shrink_0()
                    .rounded(px(radius::MD))
                    .overflow_hidden()
                    .child(
                        img(img_src.clone())
                            .size_full()
                            .object_fit(gpui::ObjectFit::Cover),
                    )
                    .into_any_element(),
                // Otherwise show app icon or fallback in 64x64px container
                _ => div()
                    .size(px(64.0))
                    .flex_shrink_0()
                    .rounded(px(radius::MD))
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
                                    .size(px(28.0))
                                    .object_fit(gpui::ObjectFit::Contain)
                                    .into_any_element()
                            })
                            .unwrap_or_else(|| {
                                div()
                                    .text_size(theme.font_sizes.md)
                                    .text_color(theme.text.secondary)
                                    .child(icon_fallback(&app_name, &app_icon_name))
                                    .into_any_element()
                            }),
                    )
                    .into_any_element(),
            },
        )
        .child(
            div()
                .flex_1()
                .flex()
                .flex_col()
                .gap(px(4.0))
                .overflow_hidden()
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(spacing::SM))
                        .child(
                            div()
                                .flex_1()
                                .text_size(theme.font_sizes.xs)
                                .text_color(theme.text.secondary)
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .child(app_name),
                        )
                        .child(
                            div()
                                .text_size(theme.font_sizes.xs)
                                .text_color(theme.text.muted)
                                .flex_shrink_0()
                                .child(timestamp),
                        ),
                )
                .child(
                    div()
                        .text_size(theme.font_sizes.sm)
                        .text_color(theme.text.primary)
                        .line_height(px(18.0))
                        .child(summary),
                )
                .when(!body.is_empty(), |el| {
                    el.child(
                        div()
                            .text_size(theme.font_sizes.xs)
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
                                    .text_size(theme.font_sizes.xs)
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
                }),
        )
        .into_any_element()
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
