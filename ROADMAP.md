# Roadmap to 1.0

GPUi Shell is at **v0.1.0**. This document is the public plan for getting to
**1.0**: what "1.0" means, the milestones on the way there, and where help is
most welcome.

It is a living document. Items move, get split, or get dropped as we learn.
If you think something is missing or mis-prioritised, open a
[discussion or issue](https://github.com/andre-brandao/gpui-shell/issues) -
that is exactly the feedback this file exists to collect.

## What 1.0 means

1.0 is a promise about **stability**, not about feature count. When we tag it:

1. **Your config keeps working.** `config.toml` and `theme.toml` keys are
   documented and stable. Breaking changes before 2.0 come with a deprecation
   period and a warning from `gpuishell --validate`.
2. **It survives a day of real use.** Services recover on their own from
   D-Bus hiccups, daemon restarts and compositor reloads, and nothing leaks
   threads, memory or file handles over a long session.
3. **It works on every supported compositor.** Hyprland, Niri and Mango each
   have the same bar, launcher and control-center features, including
   multi-monitor setups.
4. **It is easy to install.** Nix is first-class, and there are packages or
   build instructions for other common distros.
5. **It is easy to contribute to.** CI checks every PR, the architecture is
   documented, and there is a clear path from "I want to help" to a merged PR.

Out of scope for 1.0: X11 support, a plugin/scripting API (see
[After 1.0](#after-10)), and compositors other than the three above.

## How to read this

- Milestones are roughly ordered. Items inside a milestone can mostly be done
  in parallel.
- This file is the tracker. Only items that need a discussion get a GitHub
  issue, and those are linked from their item.

---

## v0.2: Contributor foundations

_Goal: make outside contributions safe and pleasant before asking for them._

- [ ] Add a LICENSE (GPL-3.0 proposed in #41, replacing #40).
- [x] CI workflow on every PR: `cargo fmt --check`, `cargo clippy -- -D warnings`,
      `cargo test`, `nix build`.
- [x] Expand `CONTRIBUTING.md`: dev setup (`nix develop`), crate layout, how to
      run the story gallery (`cargo run -p story`), commit style, and how to
      pick an issue.
- [x] Developer section on the docs site: development setup, and an
      architecture page with a hand-drawn diagram.
- [x] Temporary guides for the current Rust API:
      [adding a bar module](docs/src/content/docs/developer/unstable/bar-module.md)
      and [adding a launcher view](docs/src/content/docs/developer/unstable/launcher-view.md).
      They get rewritten once the module APIs settle (v0.4).
- [x] Issue templates: add a "compositor + version" field to the bug report.

## v0.3: Reliability

_Goal: the shell runs for days without a restart._

Services lifecycle (do these in order):

- [x] Service cancellation: stop/restart actually stops the old run. Today a
      quiet bus parks the listener forever, which leaks a thread and runtime
      on every restart, and stale listeners can still write data.
- [ ] Service supervision: retry with backoff instead of dying on the first
      error, replacing the hand-rolled retry loops in tray/mpris.
- [x] Command dispatch through a per-service job queue instead of a mix of
      inline calls, foreground `cx.spawn` and a thread per command.
- [ ] Compositor as a managed service, with a `RunToken` and a supervised
      reconnect after a compositor reload kills the IPC socket.

Known bugs:

- [ ] Workspaces widget doesn't scope to its own monitor (#33).
- [ ] Wallpaper picker decodes every wallpaper at full resolution and never
      frees them (#39).

Hardening:

- [ ] Remove `.expect`/`unwrap` panics on startup paths, and fail with a
      readable error instead (e.g. no compositor detected).
- [ ] Long-session soak test: script that runs the shell for 24h while
      sampling RSS, threads and fds (builds on the existing profiling
      scripts).

## v0.4: Bar modules & launcher

_Goal: a stable way to extend the bar and launcher, and a launcher good
enough to replace rofi/walker as a daily driver._

Module APIs. The design discussion is open now, and the code lands here:

- [ ] Bar module API: registration without editing a central enum,
      per-module config tables, the monitor a module is on, and a shape a
      post-1.0 plugin system (Lua, TypeScript or Rust, to be decided) can
      target unchanged.
- [ ] Launcher view API, with the same goals, plus `&mut self`, a real
      `Context` and async matching, so stateful views drop their
      interior-mutability workarounds and click and Enter take the same
      path.
- [ ] Port the built-in bar modules and launcher views to the new APIs.
- [ ] Rewrite the "Adding a bar module" and "Adding a launcher view" guides
      against the new APIs, and move them out of `developer/unstable/`.

Launcher quality:

- [ ] Fuzzy matching with ranking and match highlighting (e.g. port or use Zed's
      `fuzzy` crate) to replace per-view `contains` filters.
- [ ] Render path: `uniform_list` instead of measure-all, and parse the
      query once per change instead of once per row.
- [ ] Unify the launcher and panel window registries so only one surface is
      open at a time.

Input:

- [ ] Shared menu key context, so navigation and text editing are bound once
      instead of copied across the launcher, control center and dock picker.
- [ ] User-configurable keybindings in `config.toml`.

New launcher views, written against the new API (propose yours):

- [ ] Calculator, clipboard history, emoji picker, power/session menu.

## v0.5: Polish, performance & consistency

_Goal: every module feels finished, on every compositor._

Code health (each item independent):

- [ ] Registry macro for `Services`, so adding a service touches one list
      instead of five.
- [ ] Split hot fields (network throughput, sysinfo) out of whole-struct
      signals so they stop re-rendering every subscriber.
- [ ] Shared `ui::patterns::LauncherRow` and merged shell/web views, which
      are near line-for-line copies today.
- [ ] Rethink `memory_bytes()`. The hand-written size arithmetic in 12
      services drifts silently.

Parity & UX:

- [ ] Compositor feature matrix in the docs (Hyprland / Niri / Mango × every
      bar module and launcher view), with gaps filed as issues.
- [ ] Multi-monitor audit: bar, OSD, notifications and dock on hotplug and
      scale changes.
- [ ] Accessibility pass: keyboard reachability of every control-center
      control, and contrast checks for built-in themes.
- [ ] Every `ui::patterns` surface and bar module has a story in
      `cargo run -p story`.

## v0.6: Configuration

_Goal: change settings without opening an editor._

- [ ] Config view: browse and change settings (bar layout, modules,
      launcher, services) from the launcher instead of editing
      `config.toml` by hand. The shell never writes `config.toml`, so this
      needs a decision on where changes go: `state.toml` overrides, or
      opt-in writes that keep the user's comments and formatting.

## v0.9: Release candidate

_Goal: freeze the surface and get it in front of as many users as possible._

- [ ] Config schema audit: rename or remove anything we don't want to support
      for years, and document every key with its default.
- [ ] Deprecation mechanism: old keys still load, and `--validate` and a
      notification say what to change.
- [ ] Packaging beyond Nix: AUR package, and a static/portable release tarball.
      Fedora/Debian would be welcome too.
- [ ] aarch64-linux release builds (already stubbed in `release.yml`).
- [ ] Changelog generated from conventional commits.
- [ ] Replace the README's "early development" notice with a stability
      statement.

## 1.0

- [ ] All of the above are done or explicitly deferred.
- [ ] Two weeks on an RC with no release-blocking bugs.
- [ ] Tag, announce, celebrate.

---

## After 1.0

Ideas we're interested in but won't block 1.0 on. Discussion welcome:

- Plugin or scripting API for third-party bar modules and launcher views,
  built on the bar module and launcher view APIs from v0.4
- Lock screen and idle integration
- More compositors (Sway, river)
- Theme sharing / importing from other ecosystems (base16, matugen)

## How to get involved

1. Pick an unchecked item. If it links an issue, read the discussion there
   first.
2. For anything bigger than a small fix, open an issue to sketch your
   approach before writing code.
3. Open a PR against `dev`.

Not sure where to start? Open an issue describing what you use GPUi Shell for
and what bothers you most. That feedback shapes this roadmap too.
