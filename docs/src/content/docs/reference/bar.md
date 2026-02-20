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
| `end`      | `string[]` | `["Clock", "Systray", "KeyboardLayout", "Settings"]` | Widgets in the end section.                     |

Vertical layout is used when `position` is `left` or `right`. Horizontal layout is used for `top` or `bottom`.

## Example

```toml
[bar]
size = 40.0
position = "top"
start = ["LauncherBtn", "Workspaces"]
center = ["Clock"]
end = ["Systray", "Battery", "Settings"]
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
