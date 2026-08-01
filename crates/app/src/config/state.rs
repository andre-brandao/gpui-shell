//! Values the shell persists on its own, kept out of `config.toml`.
//!
//! `config.toml` is hand-written, so the app never writes to it: a dock pin
//! toggled from the UI or a service mode set from `;s` lands in `state.toml`
//! instead. That keeps the two writers apart - a config we failed to parse is
//! never replaced by defaults the next time something is pinned.
//!
//! Each field here overrides its `config.toml` counterpart rather than
//! replacing it, so a hand-written value stays live until the UI overrides it.

use std::collections::BTreeMap;
use std::path::PathBuf;

use gpui::{App, Global};
use serde::{Deserialize, Serialize};
use services::ServiceMode;

use super::ActiveConfig;
use super::persistence::{config_dir, parse_toml, write_toml};

/// State written by the shell itself.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    /// Overrides `dock.pinned`. Stays `None` until something is pinned from
    /// the UI, so a hand-written list keeps working until then.
    pinned: Option<Vec<String>>,
    /// Overrides `[services]`, per service.
    services: BTreeMap<String, ServiceMode>,
}

impl Global for State {}

impl State {
    /// Load `state.toml` into the global. A broken one is not worth a fuss:
    /// the config values it overrides still apply.
    pub(super) fn init(cx: &mut App) {
        let state = load().unwrap_or_else(|err| {
            tracing::warn!("Failed to load state, using config values: {err:#}");
            State::default()
        });
        cx.set_global(state);
    }

    /// Pinned dock entries: what the UI last saved, else what `config.toml`
    /// declares.
    pub fn pinned(cx: &App) -> &[String] {
        cx.global::<Self>()
            .pinned
            .as_deref()
            .unwrap_or(&cx.config().dock.pinned)
    }

    /// Replace the pinned list and persist it.
    pub fn set_pinned(pinned: Vec<String>, cx: &mut App) {
        cx.global_mut::<Self>().pinned = Some(pinned);
        Self::save(cx);
    }

    /// Startup mode for a service: what the UI last saved, else what
    /// `config.toml` declares, else eager.
    pub fn service_mode(name: &str, cx: &App) -> ServiceMode {
        resolve_mode(&cx.global::<Self>().services, &cx.config().services, name)
    }

    /// Set a service's startup mode and persist it.
    pub fn set_service_mode(name: &str, mode: ServiceMode, cx: &mut App) {
        cx.global_mut::<Self>()
            .services
            .insert(name.to_lowercase(), mode);
        Self::save(cx);
    }

    fn save(cx: &App) {
        let state = cx.global::<Self>();
        if let Err(err) = state_path().and_then(|path| write_toml(&path, state)) {
            tracing::warn!("Failed to persist state: {err:#}");
        }
    }
}

/// Service names are keyed lowercased in both maps.
fn resolve_mode(
    state: &BTreeMap<String, ServiceMode>,
    config: &BTreeMap<String, ServiceMode>,
    name: &str,
) -> ServiceMode {
    let name = name.to_lowercase();
    state
        .get(&name)
        .or_else(|| config.get(&name))
        .copied()
        .unwrap_or_default()
}

pub(super) fn state_path() -> anyhow::Result<PathBuf> {
    Ok(config_dir()?.join("state.toml"))
}

fn load() -> anyhow::Result<State> {
    Ok(parse_toml(&state_path()?)?.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn modes(pairs: &[(&str, ServiceMode)]) -> BTreeMap<String, ServiceMode> {
        pairs
            .iter()
            .map(|(name, mode)| ((*name).to_string(), *mode))
            .collect()
    }

    #[test]
    fn state_overrides_config_which_overrides_the_default() {
        let state = modes(&[("tray", ServiceMode::Eager)]);
        let config = modes(&[("tray", ServiceMode::Off), ("audio", ServiceMode::Lazy)]);

        // The UI had the last word on tray.
        assert_eq!(resolve_mode(&state, &config, "Tray"), ServiceMode::Eager);
        // Nothing in state, so the hand-written value stands.
        assert_eq!(resolve_mode(&state, &config, "Audio"), ServiceMode::Lazy);
        // Neither file mentions it.
        assert_eq!(
            resolve_mode(&state, &config, "Bluetooth"),
            ServiceMode::Eager
        );
    }

    #[test]
    fn state_round_trips_through_toml() {
        let state = State {
            pinned: Some(vec!["firefox.desktop".to_string()]),
            services: modes(&[("tray", ServiceMode::Off)]),
        };

        let encoded = toml::to_string_pretty(&state).expect("state serializes");
        let decoded: State = toml::from_str(&encoded).expect("state parses back");

        assert_eq!(
            decoded.pinned.as_deref(),
            Some(&["firefox.desktop".to_string()][..])
        );
        assert_eq!(decoded.services.get("tray"), Some(&ServiceMode::Off));
    }

    /// An empty pinned list is a real answer - "the user unpinned everything" -
    /// and must not fall back to the config's list.
    #[test]
    fn an_empty_pinned_list_still_overrides_the_config() {
        let state = State {
            pinned: Some(Vec::new()),
            ..State::default()
        };

        assert_eq!(state.pinned.as_deref(), Some(&[][..]));
    }
}
