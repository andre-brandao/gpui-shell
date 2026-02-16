//! MPRIS panel with a list of players and transport controls.

use futures_signals::signal::SignalExt;
use gpui::{App, Context, FontWeight, MouseButton, Window, div, img, prelude::*, px};
use services::{MprisCommand, MprisData, MprisSubscriber, PlaybackStatus, PlayerCommand};
use ui::{ActiveTheme, font_size, icon_size, radius, spacing};

mod icons {
    pub const HEADER: &str = "󰕾";
    pub const PLAY: &str = "󰐊";
    pub const PAUSE: &str = "󰏤";
    pub const PREV: &str = "󰒮";
    pub const NEXT: &str = "󰒭";
    pub const PLAYER: &str = "󰎈";
    pub const DURATION: &str = "󰥔";
}

/// Panel content for controlling media players exposed via MPRIS.
pub struct MprisPanel {
    subscriber: MprisSubscriber,
    data: MprisData,
}

impl MprisPanel {
    pub fn new(subscriber: MprisSubscriber, cx: &mut Context<Self>) -> Self {
        let data = subscriber.get();

        cx.spawn({
            let mut signal = subscriber.subscribe().to_stream();
            async move |this, cx| {
                use futures_util::StreamExt;
                while let Some(data) = signal.next().await {
                    let result = this.update(cx, |this, cx| {
                        this.data = data;
                        cx.notify();
                    });
                    if result.is_err() {
                        break;
                    }
                }
            }
        })
        .detach();

        Self { subscriber, data }
    }

    fn run_command(
        cx: &mut App,
        subscriber: MprisSubscriber,
        service_name: String,
        command: PlayerCommand,
    ) {
        cx.spawn(async move |_| {
            let _ = subscriber
                .dispatch(MprisCommand {
                    service_name,
                    command,
                })
                .await;
        })
        .detach();
    }

    fn status_text(state: PlaybackStatus) -> &'static str {
        match state {
            PlaybackStatus::Playing => "Playing",
            PlaybackStatus::Paused => "Paused",
            PlaybackStatus::Stopped => "Stopped",
        }
    }

    fn title_for(player: &services::MprisPlayerData) -> String {
        player
            .metadata
            .as_ref()
            .and_then(|m| m.title.clone())
            .filter(|t| !t.is_empty())
            .unwrap_or_else(|| {
                player
                    .service
                    .rsplit('.')
                    .next()
                    .unwrap_or("Unknown Player")
                    .to_string()
            })
    }

    fn subtitle_for(player: &services::MprisPlayerData) -> String {
        let artist = player
            .metadata
            .as_ref()
            .and_then(|m| m.artists.clone())
            .map(|a| a.join(", "))
            .unwrap_or_default();

        let status = Self::status_text(player.state);
        if artist.is_empty() {
            status.to_string()
        } else {
            format!("{artist} - {status}")
        }
    }

    fn sorted_players(&self) -> Vec<services::MprisPlayerData> {
        let mut players = self.data.players.clone();
        players.sort_by_key(|p| match p.state {
            PlaybackStatus::Playing => 0u8,
            PlaybackStatus::Paused => 1u8,
            PlaybackStatus::Stopped => 2u8,
        });
        players
    }

    fn format_duration(us: Option<i64>) -> String {
        let total_secs = us.unwrap_or(0).max(0) / 1_000_000;
        let minutes = total_secs / 60;
        let seconds = total_secs % 60;
        format!("{minutes}:{seconds:02}")
    }

    fn render_control_button(
        id: impl Into<gpui::ElementId>,
        label: &'static str,
        cx: &App,
        on_click: impl Fn(&mut App) + 'static,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let interactive_default = theme.interactive.default;
        let interactive_hover = theme.interactive.hover;
        let text_primary = theme.text.primary;

        div()
            .id(id.into())
            .w(px(28.))
            .h(px(24.))
            .rounded(px(radius::SM))
            .cursor_pointer()
            .flex()
            .items_center()
            .justify_center()
            .bg(interactive_default)
            .hover(move |el| el.bg(interactive_hover))
            .on_mouse_down(MouseButton::Left, move |_, _, cx| on_click(cx))
            .child(
                div()
                    .text_size(px(icon_size::SM))
                    .text_color(text_primary)
                    .child(label),
            )
    }

    fn render_player_card(
        &self,
        player: services::MprisPlayerData,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let is_playing = player.state == PlaybackStatus::Playing;
        let title = Self::title_for(&player);
        let subtitle = Self::subtitle_for(&player);
        let play_icon = if is_playing {
            icons::PAUSE
        } else {
            icons::PLAY
        };
        let service_name = player.service.clone();
        let subscriber = self.subscriber.clone();

        let status_color = match player.state {
            PlaybackStatus::Playing => theme.status.success,
            PlaybackStatus::Paused => theme.status.warning,
            PlaybackStatus::Stopped => theme.text.muted,
        };

        let volume = player
            .volume
            .map(|v| format!("{:.0}%", v.clamp(0.0, 100.0)))
            .unwrap_or_else(|| "--".to_string());
        let duration = Self::format_duration(player.duration_us);

        let can_control = player.can_control;
        let service_short = service_name
            .rsplit('.')
            .next()
            .unwrap_or("player")
            .to_string();

        let prev_service = service_name.clone();
        let pp_service = service_name.clone();
        let next_service = service_name.clone();
        let dec_service = service_name.clone();
        let inc_service = service_name.clone();

        let prev_sub = subscriber.clone();
        let pp_sub = subscriber.clone();
        let next_sub = subscriber.clone();
        let dec_sub = subscriber.clone();
        let inc_sub = subscriber;

        div()
            .w_full()
            .p(px(spacing::SM))
            .bg(theme.bg.secondary)
            .rounded(px(radius::MD))
            .border_1()
            .border_color(theme.border.subtle)
            .flex()
            .flex_col()
            .gap(px(spacing::SM))
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(spacing::SM))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(spacing::SM))
                            .child(
                                player
                                    .art_url
                                    .clone()
                                    .map(|source| {
                                        div()
                                            .size(px(34.0))
                                            .rounded(px(radius::SM))
                                            .overflow_hidden()
                                            .border_1()
                                            .border_color(theme.border.subtle)
                                            .child(img(source).size_full())
                                            .into_any_element()
                                    })
                                    .unwrap_or_else(|| {
                                        div()
                                            .size(px(34.0))
                                            .rounded(px(radius::SM))
                                            .bg(theme.bg.tertiary)
                                            .border_1()
                                            .border_color(theme.border.subtle)
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .child(
                                                div()
                                                    .text_size(px(icon_size::SM))
                                                    .text_color(theme.text.primary)
                                                    .child(icons::PLAYER),
                                            )
                                            .into_any_element()
                                    }),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .overflow_hidden()
                                    .child(
                                        div()
                                            .text_size(px(font_size::SM))
                                            .text_color(theme.text.primary)
                                            .font_weight(FontWeight::MEDIUM)
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .child(title),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(font_size::XS))
                                            .text_color(theme.text.secondary)
                                            .overflow_hidden()
                                            .text_ellipsis()
                                            .child(subtitle),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .text_size(px(font_size::XS))
                            .text_color(status_color)
                            .child(format!("{}  {}", Self::status_text(player.state), volume)),
                    ),
            )
            .when(player.duration_us.is_some(), |el| {
                el.child(
                    div()
                        .w_full()
                        .flex()
                        .items_center()
                        .gap(px(spacing::XS))
                        .child(
                            div()
                                .text_size(px(font_size::XS))
                                .text_color(theme.text.muted)
                                .child(icons::DURATION),
                        )
                        .child(
                            div()
                                .text_size(px(font_size::XS))
                                .text_color(theme.text.muted)
                                .child(duration),
                        ),
                )
            })
            .child(
                div()
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(spacing::XS))
                    .child(
                        div()
                            .text_size(px(font_size::XS))
                            .text_color(theme.text.muted)
                            .child(service_short),
                    )
                    .child(
                        div()
                            .flex()
                            .gap(px(spacing::XS))
                            .when(can_control, |el| {
                                el.child(Self::render_control_button(
                                    format!("mpris-prev-{}", service_name),
                                    icons::PREV,
                                    cx,
                                    move |cx| {
                                        Self::run_command(
                                            cx,
                                            prev_sub.clone(),
                                            prev_service.clone(),
                                            PlayerCommand::Prev,
                                        );
                                    },
                                ))
                                .child(Self::render_control_button(
                                    format!("mpris-play-{}", pp_service),
                                    play_icon,
                                    cx,
                                    move |cx| {
                                        Self::run_command(
                                            cx,
                                            pp_sub.clone(),
                                            pp_service.clone(),
                                            PlayerCommand::PlayPause,
                                        );
                                    },
                                ))
                                .child(
                                    Self::render_control_button(
                                        format!("mpris-next-{}", next_service),
                                        icons::NEXT,
                                        cx,
                                        move |cx| {
                                            Self::run_command(
                                                cx,
                                                next_sub.clone(),
                                                next_service.clone(),
                                                PlayerCommand::Next,
                                            );
                                        },
                                    ),
                                )
                            })
                            .when(can_control && player.volume.is_some(), |el| {
                                el.child(Self::render_control_button(
                                    format!("mpris-dec-{}", dec_service),
                                    "−",
                                    cx,
                                    move |cx| {
                                        let value = player.volume.unwrap_or(0.0) - 5.0;
                                        Self::run_command(
                                            cx,
                                            dec_sub.clone(),
                                            dec_service.clone(),
                                            PlayerCommand::Volume(value),
                                        );
                                    },
                                ))
                                .child(
                                    Self::render_control_button(
                                        format!("mpris-inc-{}", inc_service),
                                        "+",
                                        cx,
                                        move |cx| {
                                            let value = player.volume.unwrap_or(0.0) + 5.0;
                                            Self::run_command(
                                                cx,
                                                inc_sub.clone(),
                                                inc_service.clone(),
                                                PlayerCommand::Volume(value),
                                            );
                                        },
                                    ),
                                )
                            }),
                    ),
            )
    }
}

impl Render for MprisPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let players = self.sorted_players();
        let is_empty = players.is_empty();

        div()
            .id("mpris-panel")
            .w_full()
            .h_full()
            .p(px(spacing::LG))
            .bg(theme.bg.primary)
            .border_1()
            .border_color(theme.border.default)
            .rounded(px(radius::LG))
            .overflow_hidden()
            .child(
                div()
                    .w_full()
                    .h_full()
                    .flex()
                    .flex_col()
                    .gap(px(spacing::MD))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(px(spacing::SM))
                            .child(
                                div()
                                    .text_size(px(icon_size::XL))
                                    .text_color(theme.text.primary)
                                    .child(icons::HEADER),
                            )
                            .child(
                                div()
                                    .text_size(px(font_size::LG))
                                    .text_color(theme.text.primary)
                                    .font_weight(FontWeight::BOLD)
                                    .child("Media Players"),
                            ),
                    )
                    .when(is_empty, |el| {
                        el.child(
                            div()
                                .flex_1()
                                .flex()
                                .items_center()
                                .justify_center()
                                .text_size(px(font_size::SM))
                                .text_color(theme.text.muted)
                                .child("No MPRIS players detected"),
                        )
                    })
                    .when(!is_empty, |el| {
                        el.child(
                            div()
                                .flex_1()
                                .overflow_hidden()
                                .flex()
                                .flex_col()
                                .gap(px(spacing::SM))
                                .children(players.into_iter().map(|player| {
                                    self.render_player_card(player, cx).into_any_element()
                                })),
                        )
                    }),
            )
    }
}
