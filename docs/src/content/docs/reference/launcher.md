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
