# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with
code in this repository.

## Project Overview

GPUi Shell is a Wayland desktop shell/status bar built with GPUI (Zed's UI
framework) in Rust. It provides a system bar with modules, a command launcher,
and a control center panel. Supports Hyprland and Niri compositors.

## Development Environment

Requires Nix flakes. Enter the dev shell with `nix develop` or use direnv
(auto-loads via `.envrc`). Rust nightly (2024 edition) is required.

## Build Commands

```bash
cargo build              # Debug build
cargo run                # Run the application
cargo clippy             # Lint
cargo fmt                # Format
nix build                # Release build (LTO, size-optimized)
nix build .#debug        # Debug build via Nix
```

The binary is named `gpuishell`. Command-line arguments:

- `--input`, `-i` — Open launcher with pre-filled input

## Workspace Structure

Four crates in `crates/`:

- **app** — Main binary. Bar, launcher, control center, modules, and panel
  management.
- **services** — System integration layer. D-Bus bindings, compositor control,
  audio, network, bluetooth, power, tray, sysinfo. No GPUI dependency.
- **ui** — Shared UI components and the theme system.
- **assets** — Embedded SVG icons via `rust-embed`, implements GPUI's
  `AssetSource` trait.

## Architecture

### Services (Reactive Subscriber Pattern)

Each service in `crates/services/src/` follows the same pattern:

- Holds state in `futures_signals::signal::Mutable<T>` fields
- Exposes `subscribe()` returning `MutableSignalCloned<T>` for reactive updates
- Exposes `get()` for snapshot access
- Accepts mutations via a `dispatch(Command)` method with a typed command enum
  (e.g., `AudioCommand`, `NetworkCommand`)
- Services are collected in `Services` struct, initialized once at startup, then
  stored in `AppState` as a GPUI global

### AppState (Global Service Accessor)

Services are accessed via the `AppState` struct (`crates/app/src/state.rs`),
registered as a GPUI `Global`. Modules access services through static accessor
methods:

```rust
// Reactive subscription
let audio = AppState::audio(cx).subscribe();

// Snapshot access
let network = AppState::network(cx).get();

// Dispatch commands
AppState::compositor(cx).dispatch(CompositorCommand::FocusWorkspace(id));
```

### Bar and Modules

The bar (`crates/app/src/bar/`) uses Wayland layer shell protocol. Modules are
created via a factory in `crates/app/src/bar/modules/` that maps string names to
constructor functions. Each module accesses services via `AppState`.

### Launcher (Pluggable View System)

`crates/app/src/launcher/` implements a command launcher with prefix-based view
routing. Views implement the `LauncherView` trait
(`crates/app/src/launcher/view.rs`).

### Control Center

`crates/app/src/control_center/` provides popup panels for audio, brightness,
network, bluetooth, power, notifications, and media controls.

### Panel System

`crates/app/src/panel.rs` manages popup panels. Only one panel can be open at a
time — opening a new one closes the previous; toggling the same one closes it.

### OSD (On-Screen Display)

`crates/app/src/osd/` displays volume and brightness indicators on key press.

### Compositor Abstraction

`crates/services/src/compositor/` auto-detects the active compositor at runtime.
Commands go through `CompositorCommand` enum with backend implementations in
`hyprland.rs` and `niri.rs`.

### IPC / Single-Instance

`crates/app/src/ipc/` handles instance locking. The first instance acquires a
lock; subsequent invocations signal the primary instance to open the launcher.

## Key Dependencies

- **gpui** (git from zed-industries/zed, `wayland` feature) — UI framework
- **zbus** — D-Bus communication (network, bluetooth, upower, tray)
- **futures-signals** — Reactive state (`Mutable`, `MutableSignalCloned`)
- **tokio** — Async runtime

## Conventions

- Logging uses `tracing` macros; enable with `RUST_LOG=debug`
- All styling is done via GPUI builder patterns and the theme module
  (`crates/ui/src/theme/`) — no external CSS
- Theme colors are accessed via module paths like `theme::bg::PRIMARY`,
  `theme::accent::PRIMARY`
- Icons are SVGs in `crates/assets/icons/`, loaded through GPUI's asset system
