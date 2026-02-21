---
title: Theme
description: Theme configuration reference.
---

Theme colors are loaded from:

- `$XDG_CONFIG_HOME/gpuishell/theme.toml`
- or `~/.config/gpuishell/theme.toml` (fallback)

`theme.toml` is written automatically when you apply a theme from the launcher. If missing or invalid, the default theme is used.

## Color format

Color values accept:

- `#RRGGBB` (e.g., `#007ACC`)
- `#RRGGBBAA` (e.g., `#007ACCFF` with alpha channel)

## Configuration sections

### Background colors (`bg`)

Used for main backgrounds and container elements.

| Field | Default | Description |
|-------|---------|-------------|
| `primary` | `#1E1E1E` | Main/darkest background for large containers |
| `secondary` | `#252526` | Secondary background for cards and sections |
| `tertiary` | `#2D2D2D` | Tertiary background for inputs and hover states |
| `elevated` | `#333333` | Elevated background for dropdowns and tooltips |

### Text colors (`text`)

Used for text and foreground elements.

| Field | Default | Description |
|-------|---------|-------------|
| `primary` | `#FFFFFF` | Primary text (brightest, highest contrast) |
| `secondary` | `#CCCCCC` | Secondary text (slightly muted) |
| `muted` | `#888888` | Muted text for labels and hints |
| `disabled` | `#6E6E6E` | Disabled/inactive text |
| `placeholder` | `#6E6E6E` | Placeholder text in inputs |

### Border colors (`border`)

Used for borders and outlines.

| Field | Default | Description |
|-------|---------|-------------|
| `default` | `#3C3C3C` | Standard border color |
| `subtle` | `#2D2D2D` | Subtle/less visible borders |
| `focused` | `#007ACC` | Focused/active border (accent color) |

### Accent colors (`accent`)

Brand/accent colors used for highlights and interactions.

| Field | Default | Description |
|-------|---------|-------------|
| `primary` | `#007ACC` | Primary accent (Zed blue) |
| `selection` | `#094771` | Selection background color |
| `hover` | `#1177BB` | Hover state accent color |

### Status colors (`status`)

Semantic colors for status indicators.

| Field | Default | Description |
|-------|---------|-------------|
| `success` | `#4ADE80` | Success/positive state (green) |
| `warning` | `#FBBF24` | Warning state (amber) |
| `error` | `#F87171` | Error/critical state (red) |
| `info` | `#60A5FA` | Info state (blue) |

### Interactive colors (`interactive`)

Colors for buttons, toggles, and interactive elements.

| Field | Default | Description |
|-------|---------|-------------|
| `default` | `#3B3B3B` | Default/idle state |
| `hover` | `#454545` | Hover state |
| `active` | `#505050` | Active/pressed state |
| `toggle_on` | `#007ACC` | Toggle in "on" state |
| `toggle_on_hover` | `#1177BB` | Toggle "on" state hover |

### Font size

| Field | Default | Description |
|-------|---------|-------------|
| `font_size_base` | `13.0` | Base font size in pixels (all other sizes are calculated from this) |

## Example configuration

```toml
[bg]
primary = "#1E1E1E"
secondary = "#252526"
tertiary = "#2D2D2D"
elevated = "#333333"

[text]
primary = "#FFFFFF"
secondary = "#CCCCCC"
muted = "#888888"
disabled = "#6E6E6E"
placeholder = "#6E6E6E"

[border]
default = "#3C3C3C"
subtle = "#2D2D2D"
focused = "#007ACC"

[accent]
primary = "#007ACC"
selection = "#094771"
hover = "#1177BB"

[status]
success = "#4ADE80"
warning = "#FBBF24"
error = "#F87171"
info = "#60A5FA"

[interactive]
default = "#3B3B3B"
hover = "#454545"
active = "#505050"
toggle_on = "#007ACC"
toggle_on_hover = "#1177BB"

font_size_base = 13.0
```

## Font size scaling

The `font_size_base` value determines all other font sizes used in the application:

- **xs** (extra small): base × 0.77
- **sm** (small): base × 0.85
- **md** (medium): base × 1.08
- **lg** (large): base × 1.23
- **xl** (extra large): base × 1.38

For example, with `font_size_base = 13.0`:
- xs ≈ 10px
- sm ≈ 11px
- md ≈ 14px
- lg ≈ 16px
- xl ≈ 18px
