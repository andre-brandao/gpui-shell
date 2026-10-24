//! WiFi section for the Control Center.
//!
//! Owns the network list and the connect and password flow. It lives only
//! while the section is expanded, so its prompt state goes with it. Connect
//! progress and failures come from the service's `wifi_attempt`.

use gpui::{
    AnyElement, App, Context, FocusHandle, KeyDownEvent, MouseButton, MouseDownEvent, Window, div,
    prelude::*, px,
};
use services::{AccessPoint, NetworkCommand, NetworkData, WifiAttempt, WifiAttemptStatus};
use ui::{
    ActiveTheme, Color, Icon, IconName, IconSize, InputBuffer, Radius, Spacing, Spinner, TextSize,
    render_masked_input_line,
};

use crate::icons;
use crate::keybinds::{
    Backspace, Cancel, Confirm, CursorLeft, CursorRight, DeleteWordBack, SelectAll, SelectLeft,
    SelectRight, SelectWordLeft, SelectWordRight, WordLeft, WordRight,
};
use crate::state::{AppState, watch};

use super::list;

pub struct WifiSection {
    focus_handle: FocusHandle,
    /// Gets focus back when the prompt closes.
    parent_focus: FocusHandle,
    prompt: Option<PasswordPrompt>,
    /// Applied on the next render, which has the window.
    pending_focus: Option<PendingFocus>,
    /// The attempt last reacted to, so only a new outcome opens the prompt.
    seen_attempt: Option<WifiAttempt>,
}

struct PasswordPrompt {
    ssid: String,
    input: InputBuffer,
}

enum PendingFocus {
    Prompt,
    Parent,
}

impl WifiSection {
    pub fn new(parent_focus: FocusHandle, cx: &mut Context<Self>) -> Self {
        let network = AppState::network(cx);
        let attempt = network.get().wifi_attempt;
        // An outcome left from an earlier visit; a running attempt keeps going.
        if attempt
            .as_ref()
            .is_some_and(|a| a.status != WifiAttemptStatus::Connecting)
        {
            network.dispatch(NetworkCommand::DismissWifiAttempt);
        }
        watch(cx, network.subscribe(), |this, data, cx| {
            this.on_network(&data);
            cx.notify();
        });

        Self {
            focus_handle: cx.focus_handle(),
            parent_focus,
            prompt: None,
            pending_focus: None,
            seen_attempt: attempt,
        }
    }

    fn on_network(&mut self, data: &NetworkData) {
        if let Some(prompt) = &self.prompt
            && connected_ssid(data).as_deref() == Some(prompt.ssid.as_str())
        {
            self.close_prompt();
        }

        if data.wifi_attempt == self.seen_attempt {
            return;
        }
        self.seen_attempt = data.wifi_attempt.clone();

        // The key was missing or rejected: ask, unless already asking.
        if let Some(WifiAttempt {
            ssid,
            status: WifiAttemptStatus::NeedsPassword,
        }) = &data.wifi_attempt
            && self.prompt.as_ref().is_none_or(|p| &p.ssid != ssid)
            && data
                .wireless_access_points
                .iter()
                .any(|ap| &ap.ssid == ssid && !ap.public)
        {
            self.open_prompt(ssid.clone());
        }
    }

    fn open_prompt(&mut self, ssid: String) {
        self.prompt = Some(PasswordPrompt {
            ssid,
            input: InputBuffer::default(),
        });
        self.pending_focus = Some(PendingFocus::Prompt);
    }

    fn close_prompt(&mut self) {
        if self.prompt.take().is_some() {
            self.pending_focus = Some(PendingFocus::Parent);
        }
    }

    fn cancel_prompt(&mut self, cx: &mut Context<Self>) {
        self.close_prompt();
        AppState::network(cx).dispatch(NetworkCommand::DismissWifiAttempt);
        cx.notify();
    }

    fn select_network(&mut self, ap: &AccessPoint, cx: &mut Context<Self>) {
        if !ap.public && !ap.known {
            self.open_prompt(ap.ssid.clone());
        } else {
            self.close_prompt();
            // Known networks connect with NM's saved credentials.
            AppState::network(cx).dispatch(NetworkCommand::Connect {
                ssid: ap.ssid.clone(),
                password: None,
            });
        }
        cx.notify();
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        let network = AppState::network(cx);
        let Some(prompt) = &self.prompt else {
            return;
        };
        if prompt.input.is_empty() || is_connecting(&network.get(), &prompt.ssid) {
            return;
        }
        network.dispatch(NetworkCommand::Connect {
            ssid: prompt.ssid.clone(),
            password: Some(prompt.input.text().to_string()),
        });
    }

    fn edit(&mut self, cx: &mut Context<Self>, edit: impl FnOnce(&mut InputBuffer)) {
        if let Some(prompt) = &mut self.prompt {
            edit(&mut prompt.input);
            cx.notify();
        }
    }

    fn on_key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        if keystroke.modifiers.control || keystroke.modifiers.alt {
            return;
        }
        let ch = keystroke
            .key_char
            .as_ref()
            .and_then(|s| s.chars().next())
            .or_else(|| {
                let mut chars = keystroke.key.chars();
                chars.next().filter(|_| chars.next().is_none())
            });
        if let Some(ch) = ch.filter(|ch| !ch.is_control()) {
            self.edit(cx, |input| input.insert_str(&ch.to_string()));
        }
    }
}

impl Render for WifiSection {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match self.pending_focus.take() {
            Some(PendingFocus::Prompt) => self.focus_handle.focus(window, cx),
            Some(PendingFocus::Parent) => self.parent_focus.focus(window, cx),
            None => {}
        }

        let network = AppState::network(cx).get();
        // The prompt takes the whole section, so the list cannot reorder
        // under it.
        if let Some(prompt) = &self.prompt {
            let attempt = network
                .wifi_attempt
                .as_ref()
                .filter(|a| a.ssid == prompt.ssid)
                .map(|a| &a.status);
            let ap = network
                .wireless_access_points
                .iter()
                .find(|ap| ap.ssid == prompt.ssid);
            return self.render_prompt(attempt, ap, cx);
        }

        let connected_name = connected_ssid(&network);
        let wifi_enabled = network.wifi_enabled;
        let scanning = network.scanning;

        // Connected first, then by signal strength
        let mut aps = network.wireless_access_points.clone();
        aps.sort_by_key(|ap| {
            (
                connected_name.as_ref() != Some(&ap.ssid),
                std::cmp::Reverse(ap.strength),
            )
        });

        let mut rows: Vec<AnyElement> = Vec::with_capacity(aps.len());
        for (idx, ap) in aps.into_iter().enumerate() {
            let attempt = network
                .wifi_attempt
                .as_ref()
                .filter(|a| a.ssid == ap.ssid)
                .map(|a| &a.status);
            let connected = connected_name.as_ref() == Some(&ap.ssid);
            let error = match attempt {
                Some(WifiAttemptStatus::Failed(message)) => Some(message.clone()),
                // Open networks get no prompt, so say it here.
                Some(WifiAttemptStatus::NeedsPassword) => Some("Connection failed".into()),
                _ => None,
            };
            let connecting = attempt == Some(&WifiAttemptStatus::Connecting);
            rows.push(
                self.render_network_item(idx, ap, connected, connecting, cx)
                    .into_any_element(),
            );
            if let Some(error) = error {
                rows.push(render_error(error, cx).into_any_element());
            }
        }

        list::section(
            IconName::Wifi,
            "WiFi",
            |header| {
                header
                    .when_some(connected_name.clone(), |el, name| {
                        el.child(
                            div()
                                .text_size(TextSize::XSmall.rems())
                                .text_color(cx.theme().colors.text)
                                .child(format!("- {}", name)),
                        )
                    })
                    .child(render_refresh_button(scanning, cx))
            },
            cx,
        )
        .when(wifi_enabled && scanning, |el| {
            el.child(list::hint("Scanning...", cx))
        })
        .when(!wifi_enabled, |el| {
            el.child(list::empty_state("WiFi is off", cx))
        })
        .when(wifi_enabled && rows.is_empty(), |el| {
            el.child(list::empty_state("No networks found", cx))
        })
        .when(wifi_enabled && !rows.is_empty(), |el| {
            el.child(list::list("wifi-networks-list", cx).children(rows))
        })
        .into_any_element()
    }
}

impl WifiSection {
    /// Render a single network item in the list
    fn render_network_item(
        &self,
        index: usize,
        ap: AccessPoint,
        connected: bool,
        connecting: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let secured = !ap.public;
        let known = ap.known;
        let text_muted = theme.colors.text_muted;
        let status_success = theme.colors.status.success;
        let lock_tooltip = if known {
            "Secured (saved)"
        } else {
            "Secured (password required)"
        };
        let disconnect = {
            let service = AppState::network(cx).clone();
            let ssid = ap.ssid.clone();
            move |_: &mut App| service.dispatch(NetworkCommand::Disconnect(ssid.clone()))
        };
        let forget = {
            let service = AppState::network(cx).clone();
            let ssid = ap.ssid.clone();
            move |_: &mut App| service.dispatch(NetworkCommand::Forget(ssid.clone()))
        };

        list::row(format!("wifi-{index}"), connected, cx)
            .when(!connected && !connecting, |el| {
                let ap = ap.clone();
                el.on_mouse_down(
                    MouseButton::Left,
                    cx.listener(move |this, _: &MouseDownEvent, _, cx| {
                        this.select_network(&ap, cx)
                    }),
                )
            })
            .child(list::row_icon(
                format!("wifi-signal-{index}"),
                icons::wifi_signal_icon(ap.strength),
                connected,
                format!("Signal strength: {}%", ap.strength),
                cx,
            ))
            .child(list::row_label(ap.ssid.clone(), cx))
            .when(known && !connected, |el| {
                el.child(list::badge(
                    format!("wifi-known-{index}"),
                    IconName::Check,
                    status_success,
                    "Saved network",
                ))
            })
            .when(secured, |el| {
                el.child(list::badge(
                    format!("wifi-lock-{index}"),
                    IconName::Lock,
                    if known { status_success } else { text_muted },
                    lock_tooltip,
                ))
            })
            .when(connecting, |el| {
                el.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(Spacing::XSmall.pixels())
                        .text_size(TextSize::XSmall.rems())
                        .text_color(text_muted)
                        .child("Connecting...")
                        .child(Spinner::new().size(IconSize::XSmall)),
                )
            })
            .when(!connected && !connecting, |el| {
                el.child(list::badge(
                    format!("wifi-connect-{index}"),
                    IconName::ChevronRight,
                    text_muted,
                    "Connect",
                ))
            })
            .when(known && !connecting, |el| {
                el.child(list::action_button(
                    format!("wifi-forget-{index}"),
                    IconName::Trash,
                    text_muted,
                    "Forget network",
                    forget,
                    cx,
                ))
            })
            .when(connected, |el| {
                el.child(list::action_button(
                    format!("wifi-disconnect-{index}"),
                    IconName::Close,
                    status_success,
                    "Disconnect",
                    disconnect,
                    cx,
                ))
            })
    }

    /// Password view, shown in place of the network list.
    fn render_prompt(
        &self,
        attempt: Option<&WifiAttemptStatus>,
        ap: Option<&AccessPoint>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(prompt) = &self.prompt else {
            return div().into_any_element();
        };
        let theme = cx.theme();
        let connecting = attempt == Some(&WifiAttemptStatus::Connecting);
        let can_submit = !connecting && !prompt.input.is_empty();
        let known = ap.is_some_and(|ap| ap.known);
        let error = match attempt {
            Some(WifiAttemptStatus::NeedsPassword) if known => {
                Some("The saved password was not accepted".to_string())
            }
            Some(WifiAttemptStatus::NeedsPassword) => Some("Wrong or missing password".to_string()),
            Some(WifiAttemptStatus::Failed(message)) => Some(message.clone()),
            _ => None,
        };
        let subtitle = match ap {
            Some(ap) => format!("Secured · signal {}%", ap.strength),
            None => "Out of range".to_string(),
        };
        let password_line = render_masked_input_line(&prompt.input, "Password", '•', cx);

        let bg_primary = theme.colors.background;
        let accent_primary = theme.colors.accent;
        let text_primary = theme.colors.text;
        let text_muted = theme.colors.text_muted;
        let status_error = theme.colors.status.error;
        let hover_bg = theme.colors.element_hover;

        div()
            .id("wifi-password")
            .track_focus(&self.focus_handle)
            .key_context("WifiPassword")
            .on_action(
                cx.listener(|this, _: &Backspace, _, cx| this.edit(cx, InputBuffer::backspace)),
            )
            .on_action(cx.listener(|this, _: &DeleteWordBack, _, cx| {
                this.edit(cx, InputBuffer::delete_word_back)
            }))
            .on_action(
                cx.listener(|this, _: &CursorLeft, _, cx| this.edit(cx, |i| i.move_left(false))),
            )
            .on_action(
                cx.listener(|this, _: &CursorRight, _, cx| this.edit(cx, |i| i.move_right(false))),
            )
            .on_action(
                cx.listener(|this, _: &WordLeft, _, cx| this.edit(cx, |i| i.move_word_left(false))),
            )
            .on_action(
                cx.listener(|this, _: &WordRight, _, cx| {
                    this.edit(cx, |i| i.move_word_right(false))
                }),
            )
            .on_action(cx.listener(|this, _: &SelectWordLeft, _, cx| {
                this.edit(cx, |i| i.move_word_left(true))
            }))
            .on_action(cx.listener(|this, _: &SelectWordRight, _, cx| {
                this.edit(cx, |i| i.move_word_right(true))
            }))
            .on_action(
                cx.listener(|this, _: &SelectLeft, _, cx| this.edit(cx, |i| i.move_left(true))),
            )
            .on_action(
                cx.listener(|this, _: &SelectRight, _, cx| this.edit(cx, |i| i.move_right(true))),
            )
            .on_action(
                cx.listener(|this, _: &SelectAll, _, cx| this.edit(cx, InputBuffer::select_all)),
            )
            .on_action(cx.listener(|this, _: &Cancel, _, cx| this.cancel_prompt(cx)))
            .on_action(cx.listener(|this, _: &Confirm, _, cx| this.submit(cx)))
            .on_key_down(
                cx.listener(|this, event: &KeyDownEvent, _, cx| this.on_key_down(event, cx)),
            )
            .w_full()
            .flex()
            .flex_col()
            .gap(Spacing::Large.pixels())
            // Network header
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(Spacing::Medium.pixels())
                    .child(
                        Icon::new(IconName::Lock)
                            .size(IconSize::Small)
                            .color(Color::Accent),
                    )
                    .child(
                        div()
                            .flex_1()
                            .flex()
                            .flex_col()
                            .gap(px(2.))
                            .overflow_hidden()
                            .child(
                                div()
                                    .text_size(TextSize::Small.rems())
                                    .text_color(text_primary)
                                    .font_weight(gpui::FontWeight::MEDIUM)
                                    .child(prompt.ssid.clone()),
                            )
                            .child(
                                div()
                                    .text_size(TextSize::XSmall.rems())
                                    .text_color(text_muted)
                                    .child(subtitle),
                            ),
                    )
                    .child(
                        div()
                            .id("wifi-cancel")
                            .w(px(22.))
                            .h(px(22.))
                            .rounded(Radius::Small.pixels())
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(move |s| s.bg(hover_bg))
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(|this, _: &MouseDownEvent, _, cx| {
                                    this.cancel_prompt(cx)
                                }),
                            )
                            .child(
                                Icon::new(IconName::Close)
                                    .size(IconSize::XSmall)
                                    .color(Color::Muted),
                            ),
                    ),
            )
            // Password field
            .child(
                div()
                    .w_full()
                    .px(Spacing::Medium.pixels())
                    .py(Spacing::Small.pixels())
                    .bg(bg_primary)
                    .rounded(Radius::Small.pixels())
                    .border_1()
                    .border_color(if error.is_some() {
                        status_error
                    } else {
                        accent_primary
                    })
                    .text_size(TextSize::Small.rems())
                    .text_color(text_primary)
                    .child(password_line),
            )
            .when_some(error, |el, err| {
                el.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(Spacing::Medium.pixels())
                        .px(Spacing::Medium.pixels())
                        .py(Spacing::Small.pixels())
                        .rounded(Radius::Small.pixels())
                        .bg(status_error.opacity(0.12))
                        .child(
                            Icon::new(IconName::CircleAlert)
                                .size(IconSize::XSmall)
                                .color(Color::Custom(status_error)),
                        )
                        .child(
                            div()
                                .flex_1()
                                .text_size(TextSize::XSmall.rems())
                                .text_color(status_error)
                                .child(err),
                        ),
                )
            })
            // Connect
            .child(
                div()
                    .id("wifi-connect")
                    .w_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(Spacing::Medium.pixels())
                    .py(Spacing::Small.pixels())
                    .bg(accent_primary)
                    .rounded(Radius::Small.pixels())
                    .text_size(TextSize::Small.rems())
                    .text_color(bg_primary)
                    .when(!can_submit, |el| el.opacity(0.6))
                    .when(can_submit, |el| {
                        el.cursor_pointer().on_mouse_down(
                            MouseButton::Left,
                            cx.listener(|this, _: &MouseDownEvent, _, cx| this.submit(cx)),
                        )
                    })
                    .when(connecting, |el| {
                        el.child(
                            Spinner::new()
                                .size(IconSize::XSmall)
                                .color(Color::Custom(bg_primary)),
                        )
                    })
                    .child(if connecting {
                        "Connecting..."
                    } else {
                        "Connect"
                    }),
            )
            .child(
                div()
                    .text_center()
                    .text_size(TextSize::XSmall.rems())
                    .text_color(text_muted)
                    .child("Enter to connect · Esc to cancel"),
            )
            .into_any_element()
    }
}

/// SSID of the active WiFi connection.
fn connected_ssid(data: &NetworkData) -> Option<String> {
    data.active_connections.iter().find_map(|c| match c {
        services::ActiveConnectionInfo::WiFi { name, .. } => Some(name.clone()),
        _ => None,
    })
}

fn is_connecting(data: &NetworkData, ssid: &str) -> bool {
    data.wifi_attempt
        .as_ref()
        .is_some_and(|a| a.ssid == ssid && a.status == WifiAttemptStatus::Connecting)
}

/// Failure line under a network row.
fn render_error(message: String, cx: &App) -> impl IntoElement {
    div()
        .px(Spacing::Medium.pixels())
        .text_size(TextSize::XSmall.rems())
        .text_color(cx.theme().colors.status.error)
        .child(message)
}

/// Render a refresh button for rescanning networks. It spins, and ignores
/// clicks, while a scan is running.
fn render_refresh_button(scanning: bool, cx: &App) -> impl IntoElement {
    let services = AppState::network(cx).clone();
    let interactive_hover = cx.theme().colors.element_hover;

    list::header_button("wifi-refresh", cx)
        .when(scanning, |el| {
            el.child(
                Spinner::new()
                    .icon(IconName::Refresh)
                    .size(IconSize::XSmall)
                    .color(Color::Muted),
            )
        })
        .when(!scanning, |el| {
            el.cursor_pointer()
                .hover(move |s| s.bg(interactive_hover))
                .on_mouse_down(MouseButton::Left, move |_, _, _| {
                    services.dispatch(NetworkCommand::RequestScan);
                })
                .child(
                    Icon::new(IconName::Refresh)
                        .size(IconSize::XSmall)
                        .color(Color::Muted),
                )
        })
}
