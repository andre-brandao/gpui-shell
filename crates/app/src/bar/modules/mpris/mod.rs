//! MPRIS widget showing active media state and opening a players panel.

mod config;
pub use config::MprisConfig;

use gpui::{App, Context, MouseButton, Window, div, prelude::*, px};
use services::{MprisData, PlaybackStatus};
use ui::{ActiveTheme, radius};

use super::style;
use crate::bar::modules::WidgetSlot;
use crate::config::{ActiveConfig, Config};
use crate::panel::{PanelConfig, panel_placement, toggle_panel};
use crate::state::AppState;
use crate::state::watch;

mod panel;
pub use panel::MprisPanel;

mod icons {
    pub const PLAYING: &str = "󰐊";
    pub const PAUSED: &str = "󰏤";
    pub const STOPPED: &str = "󰓛";
}

/// Bar widget for media status and controls.
pub struct Mpris {
    slot: WidgetSlot,
    subscriber: services::MprisSubscriber,
    data: MprisData,
}

impl Mpris {
    pub fn new(slot: WidgetSlot, cx: &mut Context<Self>) -> Self {
        let subscriber = AppState::mpris(cx).clone();
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

    fn toggle_panel(&self, cx: &mut App) {
        let subscriber = self.subscriber.clone();
        let config = Config::global(cx);
        let (anchor, margin) = panel_placement(config.bar.position, self.slot);
        let config = PanelConfig {
            width: 380.0,
            height: 420.0,
            anchor,
            margin,
            namespace: "mpris-panel".to_string(),
        };

        toggle_panel("mpris", config, cx, move |cx| {
            MprisPanel::new(subscriber, cx)
        });
    }

    fn primary_player(&self) -> Option<&services::MprisPlayerData> {
        self.data
            .players
            .iter()
            .find(|p| p.state == PlaybackStatus::Playing)
            .or_else(|| self.data.players.first())
    }

    fn icon(&self) -> &'static str {
        match self.primary_player().map(|p| p.state) {
            Some(PlaybackStatus::Playing) => icons::PLAYING,
            Some(PlaybackStatus::Paused) => icons::PAUSED,
            Some(PlaybackStatus::Stopped) => icons::STOPPED,
            None => icons::STOPPED,
        }
    }

    fn label(&self) -> String {
        let Some(player) = self.primary_player() else {
            return "No media".to_string();
        };

        if let Some(metadata) = &player.metadata {
            let value = metadata.to_string();
            if !value.is_empty() {
                return value;
            }
        }

        player
            .service
            .rsplit('.')
            .next()
            .unwrap_or("Player")
            .to_string()
    }
}

impl Render for Mpris {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let is_vertical = cx.config().bar.is_vertical();
        let config = &cx.config().bar.modules.mpris;
        let icon = self.icon();
        let label = self.label();

        // Pre-compute colors for closures
        let interactive_default = theme.interactive.default;
        let interactive_hover = theme.interactive.hover;
        let interactive_active = theme.interactive.active;
        let text_primary = theme.text.primary;
        let text_secondary = theme.text.secondary;

        div()
            .id("mpris-widget")
            .flex()
            .when(is_vertical, |this| this.flex_col())
            .items_center()
            .gap(px(style::CHIP_GAP))
            .px(px(style::chip_padding_x(is_vertical)))
            .py(px(style::CHIP_PADDING_Y))
            .max_w(px(config.max_width))
            .rounded(px(radius::SM))
            .cursor_pointer()
            .bg(interactive_default)
            .hover(move |s| s.bg(interactive_hover))
            .active(move |s| s.bg(interactive_active))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                    this.toggle_panel(cx);
                }),
            )
            .child(
                div()
                    .text_size(px(style::icon(is_vertical)))
                    .text_color(text_primary)
                    .child(icon),
            )
            .when(!is_vertical, |this| {
                this.child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .text_ellipsis()
                        .text_size(px(style::label(is_vertical)))
                        .text_color(text_secondary)
                        .child(label),
                )
            })
    }
}
