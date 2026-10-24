---
title: Architecture
description: How GPUi Shell's crates, services and windows fit together.
---

GPUi Shell is a single process. `crates/services` talks to the system
(D-Bus, compositor IPC, PulseAudio, PipeWire) and exposes plain reactive
state. `crates/app` turns that state into GPUI layer-shell windows built from
`crates/ui`.

<a href="../../architecture.svg" target="_blank">
  <img src="../../architecture.svg" alt="GPUi Shell architecture: outside systems, crates/services, crates/app with its windows, and crates/ui" />
</a>

> **Why services don't depend on GPUI.** GPUi Shell came after earlier shells
> built with iced and Tauri + Svelte. Keeping the system integrations
> framework-agnostic means they survive a change of UI framework. That's a
> preference, not a rule: if tying them closer to GPUI's state model would
> help performance or stability, open an issue or PR.

## Crates

| Crate      | Owns                                                                               |
| ---------- | ---------------------------------------------------------------------------------- |
| `services` | One module per integration: state, listener, commands, lifecycle. No UI.            |
| `app`      | The binary: windows, widgets, keybindings, config, IPC.                             |
| `ui`       | `components` (`use ui::*`), shell `patterns` (`ui::patterns::X`), theme. No IO.     |
| `assets`   | Embedded Lucide SVG icons.                                                          |
| `story`    | `cargo run -p story`: a gallery of every component and pattern. Not shipped.        |

## State flow

Every service works the same way (the strip at the bottom of the diagram):

- **Read:** `AppState::audio(cx).subscribe()`, passed to `state::watch`, which
  updates the widget and calls `cx.notify()`. Use `.get()` for a one-off read.
- **Write:** widgets call `dispatch(Command)`, never mutate state. It queues
  the command on the service's `JobQueue` (`services/src/jobs.rs`), whose
  worker runs commands one at a time; the result comes back through the same
  signal. `request(Command)` awaits the outcome instead, and the queue's
  in-flight list drives progress indicators.
- **Lifecycle:** `ManagedService` (`services/src/lifecycle.rs`). Each
  `start()` gets a fresh `RunToken`; the run, and its command worker, end
  the moment it is stopped or restarted (`token.run_until_cancelled`), not at
  the next event. Startup mode per service is `eager`, `lazy` or
  `off` (`[services]` in config), and the `;s` launcher view controls them
  at runtime.

The compositor is the exception: it connects at startup, can't be stopped,
and picks its backend (Hyprland, Niri or Mango) at runtime.

## Windows

Every surface is its own layer-shell window:

| Surface             | Layer   | Instances                                              |
| ------------------- | ------- | ------------------------------------------------------ |
| Bar, Dock           | Top     | One per display                                        |
| Panels              | Overlay | One at a time (control center, tray, media, sysinfo)   |
| Launcher            | Overlay | Singleton                                              |
| OSD, notifications  | Overlay | Transient; the shell is the notification daemon        |

Windows are matched to compositor monitors by output name
(`state::monitor_for_display`), never by position or order.

## Config and IPC

- The shell never writes `config.toml`. What it persists goes to
  `state.toml`, which overrides config. The theme view also saves
  `theme.toml`.
- Both files hot-reload. A broken file keeps the last good value and raises a
  notification, and `gpuishell --validate` shows the error.
- A second `gpuishell` invocation talks to the running one over
  `$XDG_RUNTIME_DIR/gpuishell.sock` (it toggles the launcher).

## Adding things

- **Bar module:** a directory in `app/src/bar/modules/`, registered in
  `modules/registry.rs`.
- **Launcher view:** a directory in `app/src/launcher/views/` implementing
  `LauncherView`, registered in `views/mod.rs`.
- **Service:** a module in `services/src/` implementing `ManagedService`,
  registered in `app/src/state.rs` and the `[services]` config.
- **Component or pattern:** in `crates/ui`, with a story in `crates/story`.

If you add or remove a service or window, update
`docs/public/architecture.svg` in the same PR. It's plain SVG written by
Claude, so ask an AI agent to update it or edit it by hand. The header comment
documents the grid.
