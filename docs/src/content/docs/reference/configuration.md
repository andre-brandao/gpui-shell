---
title: Configuration
description: Overview of GPUi Shell configuration files.
---

GPUi Shell loads configuration from:

- `$XDG_CONFIG_HOME/gpuishell/config.toml`
- or `~/.config/gpuishell/config.toml` (fallback)

If the file does not exist, it is created with defaults on startup. Changes are hot-reloaded automatically.

## `config.toml` example

```toml
[bar]
size = 32.0
position = "left"
start = ["LauncherBtn", "Workspaces", "SysInfo"]
center = ["ActiveWindow"]
end = ["Clock", "Mpris", "Notifications", "Systray", "KeyboardLayout", "Settings"]

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

See the individual reference pages for details on each section:

- [Bar](/gpui-shell/reference/bar/)
- [Launcher](/gpui-shell/reference/launcher/)
- [OSD](/gpui-shell/reference/osd/)
- [Control Center](/gpui-shell/reference/control-center/)
- [Theme](/gpui-shell/reference/theme/)
