---
title: Control Center
description: Control center configuration reference.
---

The `[control_center]` section configures the control center panel.

## Power actions

The `[control_center.power_actions]` table sets the commands for power operations.

| Option     | Type     | Default                | Description                      |
| ---------- | -------- | ---------------------- | -------------------------------- |
| `sleep`    | `string` | `"systemctl suspend"`  | Command to run for sleep/suspend. |
| `reboot`   | `string` | `"systemctl reboot"`   | Command to run for reboot.       |
| `poweroff` | `string` | `"systemctl poweroff"` | Command to run for power off.    |

Commands run via `sh -c`. Set a command to an empty string to disable it.

## Example

```toml
[control_center.power_actions]
sleep = "systemctl suspend"
reboot = "systemctl reboot"
poweroff = "systemctl poweroff"
```
