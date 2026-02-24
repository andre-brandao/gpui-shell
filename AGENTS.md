# AGENTS.md

AI agent guidance for working with GPUi Shell.

## What is GPUi Shell?

A Wayland desktop shell/status bar built with GPUI (Zed's UI framework) in Rust.
Provides a system bar, command launcher, and control center for Hyprland and Niri
compositors.

## Quick Start

```bash
nix develop        # Enter dev shell (or use direnv)
cargo run          # Run the app
cargo clippy       # Lint
nix build          # Release build
```

Binary: `gpuishell --input "text"` opens launcher with pre-filled text.

## Codebase Structure

```
crates/
├── app/         Main binary, UI components (bar, launcher, panels)
├── services/    System integration (D-Bus, compositor, audio, network, etc.)
├── ui/          Shared components and theme system
└── assets/      Embedded SVG icons
```

## Key Patterns

### Services (Reactive State)

Services in `crates/services/src/` use `futures_signals::signal::Mutable<T>`:

```rust
// Subscribe for reactive updates
let audio = AppState::audio(cx).subscribe();

// Get snapshot
let network = AppState::network(cx).get();

// Send commands
AppState::compositor(cx).dispatch(CompositorCommand::FocusWorkspace(id));
```

Services are global singletons accessed via `AppState` (`crates/app/src/state.rs`).

### UI Components

- **Bar** (`crates/app/src/bar/`) — Wayland layer shell status bar with pluggable
  modules
- **Launcher** (`crates/app/src/launcher/`) — Prefix-based command launcher
  (views implement `LauncherView` trait)
- **Control Center** (`crates/app/src/control_center/`) — Popup panels for
  system controls
- **Panel System** (`crates/app/src/panel.rs`) — Only one panel open at a time
- **OSD** (`crates/app/src/osd/`) — Volume/brightness on-screen indicators

### Compositor Support

`crates/services/src/compositor/` auto-detects Hyprland or Niri at runtime.
Commands use `CompositorCommand` enum with backend-specific implementations.

## Important Conventions

- **Async timers**: Use `cx.background_executor().timer(duration).await` (NOT
  `tokio::time::sleep`)
- **Theme colors**: Access via `cx.theme().colors()` and `cx.theme().status()`
  - Use `rems()` for text, `px()` for spacing
- **Icons**: SVGs in `crates/assets/icons/`, loaded via GPUI asset system
- **Logging**: `tracing` macros, enable with `RUST_LOG=debug`
- **Styling**: Pure GPUI builder patterns (no CSS)

## Tech Stack

- **gpui** — UI framework (from zed-industries/zed, wayland feature)
- **zbus** — D-Bus communication
- **futures-signals** — Reactive state primitives
- **tokio** — Async runtime
