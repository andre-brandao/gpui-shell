# AGENTS.md

This file provides guidance to AI coding agents when working with code in this
repository.

## Project Overview

GPUi Shell is a Wayland desktop shell/status bar built with GPUI (Zed's UI
framework) in Rust. It provides a system bar with widgets, a command launcher,
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

The binary is named `gpuishell`. It accepts `--input "query"` to open the
launcher with pre-filled input.

## Workspace Structure

Four crates in `crates/`:

- **app** — Main binary. Bar, launcher, control center, widgets, and panel
  management.
- **services** — System integration layer. D-Bus bindings, compositor control,
  audio, network, bluetooth, power, tray, sysinfo. No GPUI dependency.

## Architecture

### Services (Reactive Subscriber Pattern)

Each service in `crates/services/` follows the same pattern:

- Holds state in `futures_signals::signal::Mutable<T>` fields
- Exposes `subscribe()` returning `MutableSignalCloned<T>` for reactive updates
- Exposes `get()` for snapshot access
- Accepts mutations via a `dispatch(Command)` method with a typed command enum
  (e.g., `AudioCommand`, `NetworkCommand`)
- Services are collected in `Services` struct (`crates/services/src/lib.rs`),
  initialized once at startup, then shared with all widgets

### Bar and Widgets

The bar (`crates/app/src/bar.rs`) uses Wayland layer shell protocol, anchored to
the top at 32px height. Widgets are created via a factory in
`crates/app/src/widgets/registry.rs` that maps string names to constructor
functions. Each widget receives the shared `Services` instance.

### Launcher (Pluggable View System)

`crates/app/src/launcher/` implements a command launcher with prefix-based view
routing. Views implement the `LauncherView` trait
(`crates/app/src/launcher/view.rs`).

### Panel System

`crates/app/src/panel.rs` manages popup panels (control center, sysinfo detail).
Only one panel can be open at a time — opening a new one closes the previous;
toggling the same one closes it.

### Compositor Abstraction

`crates/services/src/compositor/` auto-detects the active compositor at runtime.
Commands go through `CompositorCommand` enum with backend implementations in
`hyprland.rs` and `niri.rs`.

### Single-Instance IPC

`crates/services/src/shell/` handles instance locking. The first instance
acquires a lock; subsequent invocations signal the primary instance to open the
launcher.

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
  or as characters from icon fonts

## Rust Coding Guidelines

- Prioritize code correctness and clarity. Speed and efficiency are secondary
  unless otherwise specified.
- Do not write organizational or summary comments. Comments should only explain
  "why" when the reason is tricky or non-obvious.
- Prefer implementing functionality in existing files unless it is a new logical
  component. Avoid creating many small files.
- Avoid using functions that panic like `unwrap()`, instead use `?` to propagate
  errors.
- Be careful with indexing operations which may panic if indexes are out of
  bounds.
- Never silently discard errors with `let _ =` on fallible operations. Always
  handle errors appropriately:
  - Propagate errors with `?` when the calling function should handle them
  - Use `.log_err()` or similar when you need to ignore errors but want
    visibility
  - Use explicit error handling with `match` or `if let Err(...)` when you need
    custom logic
- When implementing async operations that may fail, ensure errors propagate to
  the UI layer so users get meaningful feedback.
- Never create files with `mod.rs` paths — prefer `src/some_module.rs` instead
  of `src/some_module/mod.rs`.
- When creating new crates, prefer specifying the library root path in
  `Cargo.toml` using `[lib] path = "...rs"` instead of the default `lib.rs`, to
  maintain consistent and descriptive naming.
- Avoid creative additions unless explicitly requested.
- Use full words for variable names (no abbreviations like "q" for "queue").
- Use variable shadowing to scope clones in async contexts for clarity,
  minimizing the lifetime of borrowed references:
  ```rust
  executor.spawn({
      let task_ran = task_ran.clone();
      async move {
          *task_ran.borrow_mut() = true;
      }
  });
  ```

## GPUI Framework Reference

### Context Types

Context types allow interaction with global state, windows, entities, and system
services. They are typically passed to functions as `cx`. When a function takes
callbacks they come after the `cx` parameter.

- `App` — root context, providing access to global state and entity read/update.
- `Context<T>` — provided when updating an `Entity<T>`. Derefs into `App`.
- `AsyncApp` and `AsyncWindowContext` — provided by `cx.spawn` and
  `cx.spawn_in`. Can be held across await points.

### Window

`Window` provides access to window state. Passed as `window` and comes before
`cx` when present. Used for focus management, dispatching actions, drawing, and
getting user input state.

### Entities

An `Entity<T>` is a handle to state of type `T`:

- `thing.entity_id()` returns `EntityId`
- `thing.downgrade()` returns `WeakEntity<T>`
- `thing.read(cx)` returns `&T`
- `thing.read_with(cx, |thing: &T, cx: &App| ...)` returns the closure's value
- `thing.update(cx, |thing: &mut T, cx: &mut Context<T>| ...)` allows mutation
- `thing.update_in(cx, |thing: &mut T, window: &mut Window, cx: &mut Context<T>| ...)`
  — same as `update` with `Window` access

Within closures, use the inner `cx` instead of the outer `cx` to avoid multiple
borrow issues. Trying to update an entity while it's already being updated will
panic.

When `read_with`, `update`, or `update_in` are used with an async context, the
return value is wrapped in `anyhow::Result`.

`WeakEntity<T>` has the same methods but always returns `anyhow::Result` (fails
if entity no longer exists). Useful for avoiding memory leaks with mutually
recursive handles.

### Concurrency

All entity use and UI rendering occurs on a single foreground thread.

- `cx.spawn(async move |cx| ...)` — runs async on the foreground thread. `cx` is
  `&mut AsyncApp`. When outer cx is `Context<T>`:
  `cx.spawn(async move |this, cx| ...)` where `this: WeakEntity<T>`.
- `cx.background_spawn(async move { ... })` — runs on background threads.

Both return `Task<R>`. If dropped, work is cancelled. To prevent this:

- Await it in another async context
- `task.detach()` or `task.detach_and_log_err(cx)`
- Store the task in a field (halts when struct drops)

`Task::ready(value)` creates a task that immediately provides a value.

### Timers

Use `cx.background_executor().timer(duration).await` for delays — not
`tokio::time::sleep` or `smol::Timer::after`. GPUI runs its own executor.

### Elements and Rendering

The `Render` trait renders state into a flexbox element tree:

```rust
struct TextWithBorder(SharedString);

impl Render for TextWithBorder {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div().border_1().child(self.0.clone())
    }
}
```

`SharedString` is `&'static str` or `Arc<str>`. UI components that are
constructed just to become elements can implement `RenderOnce` instead (takes
ownership of `self`, receives `&mut App`). Use `#[derive(IntoElement)]` with
these.

Style methods are similar to Tailwind CSS. Use `.when(condition, |this| ...)`
and `.when_some(option, |this, value| ...)` for conditional attributes/children.

### Input Events

Register handlers via `.on_click(|event, window, cx: &mut App| ...)`. Use
`cx.listener` to update the current entity:
`.on_click(cx.listener(|this: &mut T, event, window, cx: &mut Context<T>| ...))`.

### Actions

Dispatched via keyboard or code:
`window.dispatch_action(SomeAction.boxed_clone(), cx)` or
`focus_handle.dispatch_action(&SomeAction, window, cx)`.

Define no-data actions with `actions!(namespace, [SomeAction])`. Otherwise use
the `Action` derive macro. Doc comments on actions are displayed to the user.

Register handlers with `.on_action(|action, window, cx| ...)`, often with
`cx.listener`.

### Notify

Call `cx.notify()` when a view's state changes in a way that may affect
rendering. This triggers rerender and any `cx.observe` callbacks.

### Entity Events

Emit events with `cx.emit(event)` during entity update. Declare with
`impl EventEmitter<EventType> for EntityType {}`.

Subscribe with
`cx.subscribe(other_entity, |this, other_entity, event, cx| ...)`, which returns
a `Subscription` that deregisters on drop. Store subscriptions in a
`_subscriptions: Vec<Subscription>` field.
