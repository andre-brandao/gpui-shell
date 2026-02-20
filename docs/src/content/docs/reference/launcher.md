---
title: Launcher
description: Launcher configuration reference.
---

The `[launcher]` section controls the command launcher window dimensions and placement.

## Options

| Option          | Type    | Default | Description                              |
| --------------- | ------- | ------- | ---------------------------------------- |
| `width`         | `float` | `600.0` | Launcher window width in pixels.         |
| `height`        | `float` | `450.0` | Launcher window height in pixels.        |
| `margin_top`    | `float` | `100.0` | Top margin for layer-shell placement.    |
| `margin_right`  | `float` | `0.0`   | Right margin for layer-shell placement.  |
| `margin_bottom` | `float` | `0.0`   | Bottom margin for layer-shell placement. |
| `margin_left`   | `float` | `0.0`   | Left margin for layer-shell placement.   |

## Example

```toml
[launcher]
width = 700.0
height = 500.0
margin_top = 150.0
```

## Keyboard Shortcuts

The launcher supports vim-style navigation via Ctrl key combinations alongside standard keys.

### Navigation

| Key              | Ctrl Alternative       | Action          |
| ---------------- | ---------------------- | --------------- |
| `Up`             | `Ctrl+K` / `Ctrl+P`   | Move up         |
| `Down`           | `Ctrl+J` / `Ctrl+N`   | Move down       |
| `Page Up`        | `Ctrl+U`              | Page up         |
| `Page Down`      | `Ctrl+D`              | Page down       |
| `Enter`          |                        | Confirm / Run   |
| `Escape`         |                        | Clear / Close   |

### Editing

| Key                | Ctrl Alternative       | Action              |
| ------------------ | ---------------------- | ------------------- |
| `Left`             | `Ctrl+H`              | Move cursor left    |
| `Right`            | `Ctrl+L` / `Ctrl+F`   | Move cursor right   |
| `Ctrl+Left`        | `Ctrl+B`              | Move word left      |
| `Ctrl+Right`       | `Ctrl+W`              | Move word right     |
| `Backspace`        |                        | Delete character    |
| `Ctrl+Backspace`   |                        | Delete word back    |
| `Ctrl+A`           |                        | Select all          |
| `Shift+Left`       |                        | Select left         |
| `Shift+Right`      |                        | Select right        |
| `Ctrl+Shift+Left`  |                        | Select word left    |
| `Ctrl+Shift+Right` |                        | Select word right   |
