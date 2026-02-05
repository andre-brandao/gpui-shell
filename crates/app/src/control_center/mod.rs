//! Control Center panel using Zed UI styling.
//!
//! Provides quick toggles, audio/brightness sliders, and network/power cards.

mod bluetooth;
mod connectivity_toggles;
pub mod icons;
mod power;
mod slider;
mod wifi;

use crate::control_center::slider::{Slider, SliderEvent};
use futures_signals::signal::SignalExt;
use gpui::{
    Context, Entity, FocusHandle, Focusable, ScrollHandle, Window, div, prelude::*, px, rems,
};
use services::{AudioCommand, BrightnessCommand, Services};
use ui::{IconButton, IconButtonShape, IconName, prelude::*};

pub struct ControlCenter {
    services: Services,
    scroll: ScrollHandle,
    focus: FocusHandle,
    volume_slider: Entity<Slider>,
    brightness_slider: Entity<Slider>,
    wifi_expanded: bool,
    bt_expanded: bool,
}

impl ControlCenter {
    pub fn new(services: Services, cx: &mut Context<Self>) -> Self {
        let scroll = ScrollHandle::new();
        let focus = cx.focus_handle();

        let audio = services.audio.get();
        let volume_slider = cx.new(|_| {
            Slider::new()
                .min(0.0)
                .max(100.0)
                .step(1.0)
                .default_value(audio.sink_volume as f32)
        });

        let brightness = services.brightness.get();
        let brightness_slider = cx.new(|_| {
            Slider::new()
                .min(0.0)
                .max(100.0)
                .step(1.0)
                .default_value(brightness.percentage() as f32)
        });

        // Keep sliders in sync with services
        let audio_srv = services.audio.clone();
        cx.spawn({
            let slider = volume_slider.clone();
            let mut signal = services.audio.subscribe().to_stream();
            async move |_, cx| {
                use futures_util::StreamExt;
                while signal.next().await.is_some() {
                    let v = audio_srv.get().sink_volume as f32;
                    let _ = slider.update(cx, |s, cx| s.set_value(v, cx));
                }
            }
        })
        .detach();

        let bright_srv = services.brightness.clone();
        cx.spawn({
            let slider = brightness_slider.clone();
            let mut signal = services.brightness.subscribe().to_stream();
            async move |_, cx| {
                use futures_util::StreamExt;
                while signal.next().await.is_some() {
                    let v = bright_srv.get().percentage() as f32;
                    let _ = slider.update(cx, |s, cx| s.set_value(v, cx));
                }
            }
        })
        .detach();

        // Dispatch slider events
        let services_clone = services.clone();
        cx.subscribe(
            &volume_slider,
            move |_, _, event: &SliderEvent, _| match event {
                SliderEvent::Change(v) => {
                    let _ = services_clone
                        .audio
                        .dispatch(AudioCommand::SetSinkVolume(*v as u8));
                }
            },
        )
        .detach();

        let services_clone = services.clone();
        cx.subscribe(
            &brightness_slider,
            move |_, _, event: &SliderEvent, cx| match event {
                SliderEvent::Change(v) => {
                    let services = services_clone.clone();
                    let val = *v as u8;
                    cx.spawn(async move |_, _| {
                        let _ = services
                            .brightness
                            .dispatch(BrightnessCommand::SetPercent(val))
                            .await;
                    })
                    .detach();
                }
            },
        )
        .detach();

        Self {
            services,
            scroll,
            focus,
            volume_slider,
            brightness_slider,
            wifi_expanded: false,
            bt_expanded: false,
        }
    }
}

impl Focusable for ControlCenter {
    fn focus_handle(&self, _cx: &gpui::App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for ControlCenter {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = cx.theme().colors();
        let audio = self.services.audio.get();

        div()
            .id("control-center")
            .track_focus(&self.focus)
            .w_full()
            .h_full()
            .p(px(8.0))
            .bg(colors.background)
            .border_1()
            .border_color(colors.border)
            .rounded(px(10.0))
            .flex()
            .flex_col()
            .gap(px(6.0))
            .overflow_y_scroll()
            .track_scroll(&self.scroll)
            // Connectivity toggles row (compact, split 50/50)
            .child(connectivity_toggles::row(
                &self.services,
                self.wifi_expanded,
                self.bt_expanded,
                cx,
            ))
            // Connectivity lists (full width)
            .when(self.wifi_expanded, |el| {
                el.child(wifi::wifi_list_panel(&self.services, cx))
            })
            .when(self.bt_expanded, |el| {
                el.child(bluetooth::bluetooth_list_panel(&self.services, cx))
            })
            // Audio + display stack
            .child(audio_display_card(
                &self.services,
                audio.sink_volume,
                &self.volume_slider,
                &self.brightness_slider,
                cx,
            ))
            // Power
            .child(power::power_card(&self.services, cx))
    }
}

fn volume_row(
    services: &Services,
    volume: u8,
    slider: &Entity<Slider>,
    cx: &mut Context<'_, ControlCenter>,
) -> impl IntoElement {
    let colors = cx.theme().colors();

    let dec = cx.listener({
        let services = services.clone();
        move |_, _, _, _| {
            let new_v = volume.saturating_sub(5);
            let _ = services.audio.dispatch(AudioCommand::SetSinkVolume(new_v));
        }
    });
    let inc = cx.listener({
        let services = services.clone();
        move |_, _, _, _| {
            let new_v = volume.saturating_add(5).min(100);
            let _ = services.audio.dispatch(AudioCommand::SetSinkVolume(new_v));
        }
    });

    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .p(px(6.0))
        .bg(colors.surface_background)
        .rounded(px(8.0))
        .child(
            div()
                .text_size(rems(0.9))
                .text_color(colors.text)
                .child(icons::SPEAKER),
        )
        .child(div().flex_1().child(slider.clone()))
        .child(
            IconButton::new("vol-dec", IconName::Dash)
                .shape(IconButtonShape::Square)
                .on_click(dec),
        )
        .child(
            IconButton::new("vol-inc", IconName::Plus)
                .shape(IconButtonShape::Square)
                .on_click(inc),
        )
}

fn brightness_row(
    services: &Services,
    slider: &Entity<Slider>,
    cx: &mut Context<'_, ControlCenter>,
) -> impl IntoElement {
    let colors = cx.theme().colors();

    let dec = cx.listener({
        let services = services.clone();
        move |_, _, _, cx| {
            let services = services.clone();
            cx.spawn(async move |_, _| {
                let _ = services
                    .brightness
                    .dispatch(BrightnessCommand::Decrease(5))
                    .await;
            })
            .detach();
        }
    });
    let inc = cx.listener({
        let services = services.clone();
        move |_, _, _, cx| {
            let services = services.clone();
            cx.spawn(async move |_, _| {
                let _ = services
                    .brightness
                    .dispatch(BrightnessCommand::Increase(5))
                    .await;
            })
            .detach();
        }
    });

    div()
        .flex()
        .items_center()
        .gap(px(6.0))
        .p(px(6.0))
        .bg(colors.surface_background)
        .rounded(px(8.0))
        .child(
            div()
                .text_size(rems(0.9))
                .text_color(colors.text)
                .child(icons::BRIGHTNESS),
        )
        .child(div().flex_1().child(slider.clone()))
        .child(
            IconButton::new("bri-dec", IconName::Dash)
                .shape(IconButtonShape::Square)
                .on_click(dec),
        )
        .child(
            IconButton::new("bri-inc", IconName::Plus)
                .shape(IconButtonShape::Square)
                .on_click(inc),
        )
}

fn audio_display_card(
    services: &Services,
    volume: u8,
    volume_slider: &Entity<Slider>,
    brightness_slider: &Entity<Slider>,
    cx: &mut Context<'_, ControlCenter>,
) -> impl IntoElement {
    let colors = cx.theme().colors();

    div()
        .flex()
        .flex_col()
        .gap(px(8.0))
        .p(px(8.0))
        .bg(colors.surface_background)
        .rounded(px(8.0))
        .child(
            div().flex().items_center().gap(px(8.0)).child(
                div()
                    .text_size(rems(0.9))
                    .text_color(colors.text)
                    .child("Sound & Display"),
            ),
        )
        .child(volume_row(services, volume, volume_slider, cx))
        .child(brightness_row(services, brightness_slider, cx))
}
