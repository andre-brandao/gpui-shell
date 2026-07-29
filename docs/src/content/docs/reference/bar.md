---
title: Bar
description: Bar configuration reference.
---

The `[bar]` section controls the status bar position, size, and widget layout.

## Options

| Option     | Type       | Default                                            | Description                                        |
| ---------- | ---------- | -------------------------------------------------- | -------------------------------------------------- |
| `size`     | `float`    | `32.0`                                             | Bar thickness in pixels.                           |
| `position` | `string`   | `"left"`                                           | Screen edge: `left`, `right`, `top`, or `bottom`.  |
| `start`    | `string[]` | `["LauncherBtn", "Workspaces", "SysInfo"]`         | Widgets in the start section.                      |
| `center`   | `string[]` | `["ActiveWindow"]`                                 | Widgets in the center section.                     |
| `end`      | `string[]` | `["Clock", "Mpris", "Notifications", "Systray", "KeyboardLayout", "Settings"]` | Widgets in the end section.                     |

Vertical layout is used when `position` is `left` or `right`. Horizontal layout is used for `top` or `bottom`.

## Example

```toml
[bar]
size = 40.0
position = "top"
start = ["LauncherBtn", "Workspaces"]
center = ["Clock"]
end = ["Mpris", "Systray", "KeyboardLayout", "Settings"]
```

## Widget names

| Name             | Aliases                 |
| ---------------- | ----------------------- |
| `LauncherBtn`    | `Launcher`              |
| `Workspaces`     |                         |
| `ActiveWindow`   | `WindowTitle`           |
| `SysInfo`        |                         |
| `Clock`          |                         |
| `Systray`        | `Tray`                  |
| `KeyboardLayout` |                         |
| `Settings`       | `Info`, `ControlCenter` |
| `Battery`        |                         |

Unknown names are ignored and logged as warnings.

## Module Configuration

Each module can be configured in the `[bar.modules.<module_name>]` section.

### Clock Module

| Option                | Type     | Default                | Description                                   |
| --------------------- | -------- | ---------------------- | --------------------------------------------- |
| `format_horizontal`   | `string` | `"%d/%m/%Y %H:%M:%S"` | Time format for horizontal bars (strftime).  |
| `format_vertical`     | `string` | `"%H\n%M\n%S"`        | Time format for vertical bars (strftime).    |

### Battery Module

| Option            | Type    | Default | Description                    |
| ----------------- | ------- | ------- | ------------------------------ |
| `show_icon`       | `bool`  | `true`  | Display battery icon.          |
| `show_percentage` | `bool`  | `true`  | Display battery percentage.    |

### Workspaces Module

| Option        | Type   | Default | Description              |
| ------------- | ------ | ------- | ------------------------ |
| `show_icons`  | `bool` | `true`  | Display workspace icons. |
| `show_numbers`| `bool` | `true`  | Display workspace numbers.|

### System Info Module

| Option      | Type   | Default | Description           |
| ----------- | ------ | ------- | --------------------- |
| `show_cpu`  | `bool` | `true`  | Display CPU usage.    |
| `show_memory`| `bool` | `true`  | Display memory usage. |
| `show_temp` | `bool` | `false` | Display CPU temperature. |

### System Tray Module

| Option      | Type    | Default | Description           |
| ----------- | ------- | ------- | --------------------- |
| `icon_size` | `float` | `16.0`  | Tray icon size in pixels. |

### Media Player Module (Mpris)

| Option      | Type    | Default | Description                 |
| ----------- | ------- | ------- | --------------------------- |
| `show_cover`| `bool`  | `true`  | Display album cover art.    |
| `max_width` | `float` | `220.0` | Maximum widget width in pixels. |

### Active Window Module

| Option        | Type      | Default | Description                           |
| ------------- | --------- | ------- | ------------------------------------- |
| `max_length`  | `integer` | `64`    | Maximum characters to display in title. |
| `show_app_icon` | `bool`  | `true`  | Display application icon.             |

### Keyboard Layout Module

| Option     | Type   | Default | Description              |
| ---------- | ------ | ------- | ------------------------ |
| `show_flag`| `bool` | `false` | Display flag emoji for language. |

### Launcher Button Module

| Option | Type     | Default | Description          |
| ------ | -------- | ------- | -------------------- |
| `icon` | `string` | `"layers"` | Icon for the button. See [Icons](#icons). |

### Icons

Anywhere the config takes an icon, the value is either:

- **a built-in name** — the file stems under `crates/assets/icons/`, without
  the `.svg`: `icon = "layers"`, `icon = "battery_low"`.
- **a path to your own file** — anything containing `/`, or starting with
  `~`: `icon = "~/.config/gpuishell/icons/mine.svg"`. An `.svg` is tinted by
  the theme like a built-in; PNG and JPG render as-is. A relative path needs
  a `./` prefix, so a bare name is never mistaken for one.

A value that resolves to neither is ignored with a warning in the log, and
the built-in icon is used — a stale icon never invalidates the rest of your
config.

### Settings Module

The Settings module has no configuration options.

## Configuration Example with Modules

```toml
[bar]
size = 40.0
position = "top"
start = ["LauncherBtn", "Workspaces"]
center = ["Clock"]
end = ["Mpris", "Systray", "KeyboardLayout", "Settings"]

[bar.modules.clock]
format_horizontal = "%H:%M"

[bar.modules.battery]
show_percentage = true

[bar.modules.mpris]
show_cover = false
max_width = 300.0

[bar.modules.keyboard_layout]
show_flag = true
```
