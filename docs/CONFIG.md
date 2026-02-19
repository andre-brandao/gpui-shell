# Configuration

GPUi Shell loads configuration from:

- `$XDG_CONFIG_HOME/gpuishell/config.toml`
- or `$HOME/.config/gpuishell/config.toml` (fallback)

If the file does not exist, it is created with defaults on startup.

Theme colors are loaded from:

- `$XDG_CONFIG_HOME/gpuishell/theme.toml`
- or `$HOME/.config/gpuishell/theme.toml` (fallback)

If `theme.toml` is missing or invalid, the default theme is used.

## `config.toml` example

```toml
[bar]
size = 32.0
position = "left"
start = ["LauncherBtn", "Workspaces", "SysInfo"]
center = ["ActiveWindow"]
end = ["Clock", "Systray", "KeyboardLayout", "Settings"]

[launcher]
width = 600.0
height = 450.0
margin_top = 100.0
margin_right = 0.0
margin_bottom = 0.0
margin_left = 0.0

[osd]
position = "right"

[control_center.power_actions]
sleep = "systemctl suspend"
reboot = "systemctl reboot"
poweroff = "systemctl poweroff"
```

## `theme.toml` format

`theme.toml` is written automatically when you apply a theme from the launcher.

Color values accept:

- `#RRGGBB`
- `#RRGGBBAA`

## `bar`

- `size` (`float`): bar thickness in pixels.
- `position` (`"left" | "right" | "top" | "bottom"`): screen edge.
- `start` (`string[]`): widgets in the start section.
- `center` (`string[]`): widgets in the center section.
- `end` (`string[]`): widgets in the end section.

Notes:

- Vertical layout is inferred from `position = "left" | "right"`.
- Horizontal layout is inferred from `position = "top" | "bottom"`.

## `launcher`

- `width` (`float`): launcher window width in pixels.
- `height` (`float`): launcher window height in pixels.
- `margin_top` (`float`): top margin for layer-shell placement.
- `margin_right` (`float`): right margin for layer-shell placement.
- `margin_bottom` (`float`): bottom margin for layer-shell placement.
- `margin_left` (`float`): left margin for layer-shell placement.

## `osd`

- `position` (`"top" | "bottom" | "left" | "right"`): screen edge where the
  volume/brightness OSD appears.

## `control_center`

### `power_actions`

- `sleep` (`string`): command to run for sleep/suspend.
- `reboot` (`string`): command to run for reboot.
- `poweroff` (`string`): command to run for power off.

Commands run via `sh -c`. Set a command to an empty string to disable it.

## Widget names

Known widget names:

- `LauncherBtn` (alias: `Launcher`)
- `Workspaces`
- `ActiveWindow` (alias: `WindowTitle`)
- `SysInfo`
- `Clock`
- `Systray` (alias: `Tray`)
- `KeyboardLayout`
- `Settings` (aliases: `Info`, `ControlCenter`)
- `Battery`

Unknown names are ignored and logged as warnings.
