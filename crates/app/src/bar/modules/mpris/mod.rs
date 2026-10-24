//! MPRIS widget showing active media state and opening a players panel.

mod config;
pub use config::MprisConfig;

use gpui::{AnyElement, App, ClickEvent, Context, Pixels, Point, Window, prelude::*, px};
use services::{MprisData, PlaybackStatus};
use ui::{
    ActiveTheme, ButtonCommon, ButtonLike, ButtonStyle, Clickable, Color, Icon, IconName,
    LabelCommon,
};

use super::{BarWidget, style};
use crate::config::{ActiveConfig, Config};
use crate::state::AppState;
use crate::state::watch;
use crate::windows::{PanelConfig, PanelSize, ShellWindows, panel_placement_from_event};

mod panel;
pub use panel::MprisPanel;

/// Bar widget for media status and controls.
pub struct Mpris {
    subscriber: services::MprisSubscriber,
    data: MprisData,
}

impl Mpris {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let subscriber = AppState::mpris(cx).clone();
        let data = subscriber.get();

        watch(cx, subscriber.subscribe(), |this, data, cx| {
            this.data = data;
            cx.notify();
        });

        Self { subscriber, data }
    }

    fn toggle_panel(&self, at: Point<Pixels>, window: &Window, cx: &mut App) {
        let subscriber = self.subscriber.clone();
        let config = Config::global(cx);
        let size = PanelSize::FitHeight {
            width: 380.0,
            max_height: 600.0,
        };
        let (anchor, margin) =
            panel_placement_from_event(config.bar.position, at, window, cx, size.initial());
        let config = PanelConfig {
            size,
            anchor,
            margin,
            namespace: "mpris-panel".to_string(),
        };

        ShellWindows::toggle_panel(cx, window, "mpris", config, move |cx| {
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

    fn icon(&self) -> IconName {
        match self.primary_player().map(|p| p.state) {
            Some(PlaybackStatus::Playing) => IconName::Play,
            Some(PlaybackStatus::Paused) => IconName::Pause,
            Some(PlaybackStatus::Stopped) => IconName::Stop,
            None => IconName::Stop,
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

    fn render_widget_content(
        &self,
        theme: &ui::Theme,
        icon: IconName,
        label: Option<String>,
        is_vertical: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let max_width = cx.config().bar.modules.mpris.max_width;

        ButtonLike::new("mpris-widget")
            .style(ButtonStyle::Transparent)
            .on_click(cx.listener(|this, event: &ClickEvent, window, cx| {
                this.toggle_panel(event.position(), window, cx);
            }))
            .child(
                style::stack(is_vertical)
                    .justify_center()
                    .gap(px(style::CHIP_GAP))
                    .max_w(px(max_width))
                    .overflow_hidden()
                    .child(
                        Icon::new(icon)
                            .size(style::icon(is_vertical))
                            .color(Color::Default),
                    )
                    .when_some(label, |el, label| {
                        el.child(style::bar_label(label, is_vertical, theme.colors.text).truncate())
                    }),
            )
            .into_any_element()
    }
}

impl BarWidget for Mpris {
    fn is_interactive(&self) -> bool {
        true
    }

    fn render_vertical(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        self.render_widget_content(&theme, self.icon(), None, true, cx)
    }

    fn render_horizontal(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let theme = cx.theme().clone();
        self.render_widget_content(&theme, self.icon(), Some(self.label()), false, cx)
    }
}

impl Render for Mpris {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.render_bar_widget(window, cx)
    }
}
