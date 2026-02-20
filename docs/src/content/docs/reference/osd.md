---
title: OSD
description: On-Screen Display configuration reference.
---

The `[osd]` section controls the volume and brightness on-screen display.

## Options

| Option     | Type     | Default   | Description                                                                     |
| ---------- | -------- | --------- | ------------------------------------------------------------------------------- |
| `position` | `string` | `"right"` | Screen edge for the OSD indicator: `top`, `bottom`, `left`, or `right`. |

## Example

```toml
[osd]
position = "bottom"
```
