//! VPN section for the Control Center: saved VPN and WireGuard profiles.

use gpui::{App, MouseButton, div, prelude::*};
use services::{NetworkCommand, NetworkSubscriber, VpnProfile};
use ui::{ActiveTheme, IconName, IconSize, Spacing, Spinner, TextSize};
use zbus::zvariant::OwnedObjectPath;

use crate::state::AppState;

use super::list;

/// Render the VPN section (profile list)
pub fn render_vpn_section(cx: &App) -> impl IntoElement {
    let service = AppState::network(cx);
    let profiles = service.get().vpn_profiles;
    let pending = service.pending();

    list::section(IconName::Shield, "VPN", |header| header, cx)
        .when(profiles.is_empty(), |el| {
            el.child(list::empty_state("No VPN profiles", cx))
        })
        .when(!profiles.is_empty(), |el| {
            el.child(list::list("vpn-profiles-list", cx).children(
                profiles.into_iter().enumerate().map(|(idx, vpn)| {
                    let activity = vpn_activity(&pending, &vpn.path);
                    render_vpn_item(idx, vpn, activity, service, cx)
                }),
            ))
        })
}

/// What the queue is doing to a profile right now, as a row label.
fn vpn_activity(pending: &[NetworkCommand], path: &OwnedObjectPath) -> Option<&'static str> {
    pending.iter().rev().find_map(|command| match command {
        NetworkCommand::ConnectVpn(p) if p == path => Some("Connecting..."),
        NetworkCommand::DisconnectVpn(p) if p == path => Some("Disconnecting..."),
        _ => None,
    })
}

/// One profile row; clicking it toggles the profile.
fn render_vpn_item(
    index: usize,
    vpn: VpnProfile,
    activity: Option<&'static str>,
    service: &NetworkSubscriber,
    cx: &App,
) -> impl IntoElement {
    let theme = cx.theme();
    let text_muted = theme.colors.text_muted;
    let status_success = theme.colors.status.success;
    let active = vpn.active;
    let toggle = {
        let service = service.clone();
        let path = vpn.path.clone();
        move |_: &mut App| {
            service.dispatch(if active {
                NetworkCommand::DisconnectVpn(path.clone())
            } else {
                NetworkCommand::ConnectVpn(path.clone())
            })
        }
    };

    list::row(format!("vpn-{index}"), active, cx)
        .when(activity.is_none() && !active, |el| {
            let toggle = toggle.clone();
            el.on_mouse_down(MouseButton::Left, move |_, _, cx| toggle(cx))
        })
        .child(list::row_icon(
            format!("vpn-icon-{index}"),
            IconName::Shield,
            active,
            if active { "Connected" } else { "Disconnected" },
            cx,
        ))
        .child(list::row_label(vpn.name, cx))
        .when_some(activity, |el, label| {
            el.child(
                div()
                    .flex()
                    .items_center()
                    .gap(Spacing::XSmall.pixels())
                    .text_size(TextSize::XSmall.rems())
                    .text_color(text_muted)
                    .child(label)
                    .child(Spinner::new().size(IconSize::XSmall)),
            )
        })
        .when(activity.is_none() && !active, |el| {
            el.child(list::badge(
                format!("vpn-connect-{index}"),
                IconName::ChevronRight,
                text_muted,
                "Connect",
            ))
        })
        .when(activity.is_none() && active, |el| {
            el.child(list::action_button(
                format!("vpn-disconnect-{index}"),
                IconName::Close,
                status_success,
                "Disconnect",
                toggle,
                cx,
            ))
        })
}
