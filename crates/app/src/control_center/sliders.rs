//! Volume and brightness slider components for the Control Center.

use gpui::{App, Entity, MouseButton, div, prelude::*, px};
use services::{AudioCommand, BrightnessCommand, Services};
use ui::{ActiveTheme, Slider, font_size, icon_size, radius, spacing};

use super::icons;

/// Render the volume slider row
pub fn render_volume_slider(
    services: &Services,
    volume_slider: &Entity<Slider>,
    cx: &App,
) -> impl IntoElement {
    let audio = services.audio.get();
    let volume = audio.sink_volume;
    let muted = audio.sink_muted;

    let icon = icons::volume_icon(volume, muted);

    let services_toggle = services.clone();
    let services_dec = services.clone();
    let services_inc = services.clone();

    div()
        .flex()
        .items_center()
        .gap(px(spacing::SM))
        .w_full()
        // Icon (click to toggle mute)
        .child(render_slider_icon(
            "volume-icon",
            icon,
            muted,
            cx,
            move |_cx| {
                services_toggle.audio.dispatch(AudioCommand::ToggleSinkMute);
            },
        ))
        // Slider
        .child(div().flex_1().child(volume_slider.clone()))
        // Percent
        .child(render_percentage_label(volume, cx))
        // +/- buttons
        .child(render_adjustment_buttons(
            "volume",
            cx,
            move |_cx| {
                services_dec
                    .audio
                    .dispatch(AudioCommand::AdjustSinkVolume(-5));
            },
            move |_cx| {
                services_inc
                    .audio
                    .dispatch(AudioCommand::AdjustSinkVolume(5));
            },
        ))
}

/// Render the brightness slider row (returns empty if no brightness control available)
pub fn render_brightness_slider(
    services: &Services,
    brightness_slider: &Entity<Slider>,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let brightness = services.brightness.get();

    if brightness.max == 0 {
        return div().into_any_element();
    }

    let percent = brightness.percentage();

    let icon = if percent < 33 {
        icons::BRIGHTNESS_LOW
    } else if percent < 66 {
        icons::BRIGHTNESS
    } else {
        icons::BRIGHTNESS_HIGH
    };

    let services_dec = services.clone();
    let services_inc = services.clone();

    // Pre-compute colors
    let interactive_default = theme.interactive.default;
    let text_primary = theme.text.primary;

    div()
        .flex()
        .items_center()
        .gap(px(spacing::SM))
        .w_full()
        // Icon
        .child(
            div()
                .id("brightness-icon")
                .w(px(28.))
                .h(px(28.))
                .rounded(px(radius::SM))
                .flex()
                .items_center()
                .justify_center()
                .bg(interactive_default)
                .child(
                    div()
                        .text_size(px(icon_size::SM))
                        .text_color(text_primary)
                        .child(icon),
                ),
        )
        // Slider
        .child(div().flex_1().child(brightness_slider.clone()))
        // Percent
        .child(render_percentage_label(percent, cx))
        // +/- buttons
        .child(render_adjustment_buttons(
            "brightness",
            cx,
            move |cx| {
                let s = services_dec.clone();
                cx.spawn(async move |_| {
                    let _ = s.brightness.dispatch(BrightnessCommand::Decrease(5)).await;
                })
                .detach();
            },
            move |cx| {
                let s = services_inc.clone();
                cx.spawn(async move |_| {
                    let _ = s.brightness.dispatch(BrightnessCommand::Increase(5)).await;
                })
                .detach();
            },
        ))
        .into_any_element()
}

/// Render a clickable slider icon
fn render_slider_icon(
    id: &'static str,
    icon: &'static str,
    is_muted: bool,
    cx: &App,
    on_click: impl Fn(&mut App) + 'static,
) -> impl IntoElement {
    let theme = cx.theme();

    // Pre-compute colors for closures
    let interactive_default = theme.interactive.default;
    let interactive_hover = theme.interactive.hover;
    let status_error = theme.status.error;
    let text_primary = theme.text.primary;

    let icon_color = if is_muted { status_error } else { text_primary };

    div()
        .id(id)
        .w(px(28.))
        .h(px(28.))
        .rounded(px(radius::SM))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .bg(interactive_default)
        .hover(move |s| s.bg(interactive_hover))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            on_click(cx);
        })
        .child(
            div()
                .text_size(px(icon_size::SM))
                .text_color(icon_color)
                .child(icon),
        )
}

/// Render the percentage label
fn render_percentage_label(percent: u8, cx: &App) -> impl IntoElement {
    let theme = cx.theme();

    div()
        .w(px(32.))
        .text_size(px(font_size::XS))
        .text_color(theme.text.muted)
        .text_right()
        .child(format!("{}%", percent))
}

/// Render +/- adjustment buttons
fn render_adjustment_buttons(
    id_prefix: &'static str,
    cx: &App,
    on_decrease: impl Fn(&mut App) + 'static,
    on_increase: impl Fn(&mut App) + 'static,
) -> impl IntoElement {
    div()
        .flex()
        .gap(px(2.))
        .child(render_adjustment_button(
            format!("{}-dec", id_prefix),
            "−",
            cx,
            on_decrease,
        ))
        .child(render_adjustment_button(
            format!("{}-inc", id_prefix),
            "+",
            cx,
            on_increase,
        ))
}

/// Render a single adjustment button (+ or -)
fn render_adjustment_button(
    id: impl Into<gpui::ElementId>,
    label: &'static str,
    cx: &App,
    on_click: impl Fn(&mut App) + 'static,
) -> impl IntoElement {
    let theme = cx.theme();

    // Pre-compute colors for closures
    let interactive_default = theme.interactive.default;
    let interactive_hover = theme.interactive.hover;
    let text_muted = theme.text.muted;

    div()
        .id(id.into())
        .w(px(20.))
        .h(px(20.))
        .rounded(px(radius::SM))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .bg(interactive_default)
        .hover(move |s| s.bg(interactive_hover))
        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
            on_click(cx);
        })
        .child(
            div()
                .text_size(px(font_size::XS))
                .text_color(text_muted)
                .child(label),
        )
}
