//! Volume and brightness slider components for the Control Center.

use gpui::{App, MouseButton, div, prelude::*, px};
use services::{AudioCommand, BrightnessCommand};
use ui::{ActiveTheme, Color, Icon, IconName, IconSize, Radius, Slider, Spacing, TextSize};

use crate::state::AppState;

use crate::icons;

/// Render the volume slider row
pub fn render_volume_slider(cx: &App) -> impl IntoElement {
    let audio = AppState::audio(cx).get();
    let volume = audio.sink_volume;
    let muted = audio.sink_muted;

    let icon = icons::volume_icon(volume, muted);

    let services_toggle = AppState::audio(cx).clone();
    let services_slider = AppState::audio(cx).clone();
    let services_dec = AppState::audio(cx).clone();
    let services_inc = AppState::audio(cx).clone();

    div()
        .flex()
        .items_center()
        .gap(Spacing::Medium.pixels())
        .w_full()
        // Icon (click to toggle mute)
        .child(render_slider_icon(
            "volume-icon",
            icon,
            muted,
            cx,
            move |_cx| {
                services_toggle.dispatch(AudioCommand::ToggleSinkMute);
            },
        ))
        // Slider
        .child(
            div().flex_1().child(
                Slider::new("volume-slider", volume as f32)
                    .min(0.0)
                    .max(100.0)
                    .step(1.0)
                    .on_change(move |value, _window, _cx| {
                        services_slider.dispatch(AudioCommand::SetSinkVolume(value as u8));
                    }),
            ),
        )
        // Percent
        .child(render_percentage_label(volume, cx))
        // +/- buttons
        .child(render_adjustment_buttons(
            "volume",
            cx,
            move |_cx| {
                services_dec.dispatch(AudioCommand::AdjustSinkVolume(-5));
            },
            move |_cx| {
                services_inc.dispatch(AudioCommand::AdjustSinkVolume(5));
            },
        ))
}

/// Render the brightness slider row (returns empty if no brightness control available)
pub fn render_brightness_slider(cx: &App) -> impl IntoElement {
    let theme = cx.theme();
    let brightness = AppState::brightness(cx).get();

    if brightness.max == 0 {
        return div().into_any_element();
    }

    let percent = brightness.percentage();

    let icon = icons::BRIGHTNESS;

    let services_slider = AppState::brightness(cx).clone();
    let services_dec = AppState::brightness(cx).clone();
    let services_inc = AppState::brightness(cx).clone();

    // Pre-compute colors
    let interactive_default = theme.colors.element_background;
    let text_primary = theme.colors.text;

    div()
        .flex()
        .items_center()
        .gap(Spacing::Medium.pixels())
        .w_full()
        // Icon
        .child(
            div()
                .id("brightness-icon")
                .w(px(28.))
                .h(px(28.))
                .rounded(Radius::Small.pixels())
                .flex()
                .items_center()
                .justify_center()
                .bg(interactive_default)
                .child(
                    Icon::new(icon)
                        .size(IconSize::XSmall)
                        .color(Color::Custom(text_primary)),
                ),
        )
        // Slider
        .child(
            div().flex_1().child(
                Slider::new("brightness-slider", percent as f32)
                    .min(0.0)
                    .max(100.0)
                    .step(1.0)
                    .on_change(move |value, _window, cx| {
                        // Brightness writes go through logind, so the dispatch
                        // is async - unlike the audio one.
                        let services = services_slider.clone();
                        cx.spawn(async move |_| {
                            let _ = services
                                .dispatch(BrightnessCommand::SetPercent(value as u8))
                                .await;
                        })
                        .detach();
                    }),
            ),
        )
        // Percent
        .child(render_percentage_label(percent, cx))
        // +/- buttons
        .child(render_adjustment_buttons(
            "brightness",
            cx,
            move |cx| {
                let s = services_dec.clone();
                cx.spawn(async move |_| {
                    let _ = s.dispatch(BrightnessCommand::Decrease(5)).await;
                })
                .detach();
            },
            move |cx| {
                let s = services_inc.clone();
                cx.spawn(async move |_| {
                    let _ = s.dispatch(BrightnessCommand::Increase(5)).await;
                })
                .detach();
            },
        ))
        .into_any_element()
}

/// Render a clickable slider icon
fn render_slider_icon(
    id: &'static str,
    icon: IconName,
    is_muted: bool,
    cx: &App,
    on_click: impl Fn(&mut App) + 'static,
) -> impl IntoElement {
    let theme = cx.theme();

    // Pre-compute colors for closures
    let interactive_default = theme.colors.element_background;
    let interactive_hover = theme.colors.element_hover;
    let status_error = theme.colors.status.error;
    let text_primary = theme.colors.text;

    let icon_color = if is_muted { status_error } else { text_primary };

    div()
        .id(id)
        .w(px(28.))
        .h(px(28.))
        .rounded(Radius::Small.pixels())
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
            Icon::new(icon)
                .size(IconSize::XSmall)
                .color(Color::Custom(icon_color)),
        )
}

/// Render the percentage label
fn render_percentage_label(percent: u8, cx: &App) -> impl IntoElement {
    let theme = cx.theme();

    div()
        .w(px(32.))
        .text_size(TextSize::XSmall.rems())
        .text_color(theme.colors.text_muted)
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
    let interactive_default = theme.colors.element_background;
    let interactive_hover = theme.colors.element_hover;
    let text_muted = theme.colors.text_muted;

    div()
        .id(id.into())
        .w(px(20.))
        .h(px(20.))
        .rounded(Radius::Small.pixels())
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
                .text_size(TextSize::XSmall.rems())
                .text_color(text_muted)
                .child(label),
        )
}
