//! Keyboard actions and configurable key bindings.
//!
//! Bindings are loaded from `~/.config/gpuishell/keybinds.toml`. If the file
//! does not exist, sensible defaults are used. User overrides replace the
//! default keystrokes for a given action; unmentioned actions keep defaults.

use std::collections::HashMap;
use std::path::PathBuf;

use gpui::{App, Global, KeyBinding, actions};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Actions
// ---------------------------------------------------------------------------

actions!(
    keybinds,
    [
        Cancel,
        Confirm,
        CursorUp,
        CursorDown,
        PageUp,
        PageDown,
        Backspace,
        DeleteWordBack,
        CursorLeft,
        CursorRight,
        WordLeft,
        WordRight,
        SelectAll,
        SelectWordLeft,
        SelectWordRight,
        SelectLeft,
        SelectRight,
        TabComplete,
    ]
);

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Per-context binding overrides: action name → list of keystrokes.
type BindingMap = HashMap<String, Vec<String>>;

/// User-configurable keybindings loaded from `keybinds.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct KeybindsConfig {
    pub launcher: BindingMap,
    pub control_center: BindingMap,
}

impl Global for KeybindsConfig {}

// ---------------------------------------------------------------------------
// Persistence
// ---------------------------------------------------------------------------

pub fn keybinds_path() -> anyhow::Result<PathBuf> {
    let dir = if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
        PathBuf::from(xdg).join("gpuishell")
    } else if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".config").join("gpuishell")
    } else {
        anyhow::bail!("Unable to determine config path (XDG_CONFIG_HOME/HOME not set)");
    };
    Ok(dir.join("keybinds.toml"))
}

pub fn load_keybinds() -> anyhow::Result<KeybindsConfig> {
    let path = keybinds_path()?;
    if !path.exists() {
        return Ok(KeybindsConfig::default());
    }
    let raw = std::fs::read_to_string(&path)?;
    Ok(toml::from_str(&raw)?)
}

// ---------------------------------------------------------------------------
// Dynamic binding builder
// ---------------------------------------------------------------------------

/// Emit `KeyBinding`s for one action in a given context.
///
/// Uses a macro because `KeyBinding::new` requires a concrete `Action` type.
macro_rules! emit_bindings {
    ($bindings:expr, $overrides:expr, $context:expr, $name:expr, $defaults:expr, $action:expr) => {{
        let keys = $overrides.get($name);
        if let Some(user_keys) = keys {
            for keystroke in user_keys {
                $bindings.push(KeyBinding::new(keystroke.as_str(), $action, Some($context)));
            }
        } else {
            for keystroke in $defaults {
                $bindings.push(KeyBinding::new(*keystroke, $action, Some($context)));
            }
        }
    }};
}

/// Build shared editing bindings for a context.
fn build_shared_editing(overrides: &BindingMap, context: &str) -> Vec<KeyBinding> {
    let mut b = Vec::new();
    emit_bindings!(b, overrides, context, "cancel", &["escape"], Cancel);
    emit_bindings!(b, overrides, context, "confirm", &["enter"], Confirm);
    emit_bindings!(b, overrides, context, "backspace", &["backspace"], Backspace);
    emit_bindings!(
        b,
        overrides,
        context,
        "delete_word_back",
        &["ctrl-backspace"],
        DeleteWordBack
    );
    emit_bindings!(
        b,
        overrides,
        context,
        "cursor_left",
        &["left", "ctrl-h"],
        CursorLeft
    );
    emit_bindings!(
        b,
        overrides,
        context,
        "cursor_right",
        &["right", "ctrl-f", "ctrl-l"],
        CursorRight
    );
    emit_bindings!(
        b,
        overrides,
        context,
        "word_left",
        &["ctrl-left", "ctrl-b"],
        WordLeft
    );
    emit_bindings!(
        b,
        overrides,
        context,
        "word_right",
        &["ctrl-right", "ctrl-w"],
        WordRight
    );
    emit_bindings!(
        b,
        overrides,
        context,
        "select_word_left",
        &["ctrl-shift-left"],
        SelectWordLeft
    );
    emit_bindings!(
        b,
        overrides,
        context,
        "select_word_right",
        &["ctrl-shift-right"],
        SelectWordRight
    );
    emit_bindings!(b, overrides, context, "select_all", &["ctrl-a"], SelectAll);
    emit_bindings!(
        b,
        overrides,
        context,
        "select_left",
        &["shift-left"],
        SelectLeft
    );
    emit_bindings!(
        b,
        overrides,
        context,
        "select_right",
        &["shift-right"],
        SelectRight
    );
    b
}

// ---------------------------------------------------------------------------
// Registration
// ---------------------------------------------------------------------------

/// Register keybindings with GPUI, merging defaults with user config.
pub fn register(config: &KeybindsConfig, cx: &mut App) {
    // Launcher: shared editing + navigation
    let mut launcher = build_shared_editing(&config.launcher, "Launcher");
    emit_bindings!(
        launcher,
        &config.launcher,
        "Launcher",
        "cursor_up",
        &["up", "ctrl-k", "ctrl-p"],
        CursorUp
    );
    emit_bindings!(
        launcher,
        &config.launcher,
        "Launcher",
        "cursor_down",
        &["down", "ctrl-j", "ctrl-n"],
        CursorDown
    );
    emit_bindings!(
        launcher,
        &config.launcher,
        "Launcher",
        "page_up",
        &["pageup", "ctrl-u"],
        PageUp
    );
    emit_bindings!(
        launcher,
        &config.launcher,
        "Launcher",
        "page_down",
        &["pagedown", "ctrl-d"],
        PageDown
    );
    emit_bindings!(
        launcher,
        &config.launcher,
        "Launcher",
        "tab_complete",
        &["tab"],
        TabComplete
    );
    cx.bind_keys(launcher);

    // ControlCenter: shared editing only
    let control_center = build_shared_editing(&config.control_center, "ControlCenter");
    cx.bind_keys(control_center);
}
