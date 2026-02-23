---
title: Installation
description: How to install GPUi Shell on NixOS.
---

GPUi Shell is packaged as a Nix flake.

## Try it out

You can try GPUi Shell without installing it:

```bash
nix run github:andre-brandao/gpui-shell
```

To run with arguments (e.g., open launcher with pre-filled text):

```bash
nix run github:andre-brandao/gpui-shell -- --input "search term"
```

## Permanent Installation

### 1. Add the flake input

In your `flake.nix`:

```nix
{
  inputs = {
    # ...
    shell = {
      url = "github:andre-brandao/gpui-shell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
}
```

### 2. Install the package

Add it to your Home Manager packages or `environment.systemPackages`:

```nix
# Home Manager
home.packages = [
  inputs.shell.packages.${pkgs.system}.default
];

# Or system-wide
environment.systemPackages = [
  inputs.shell.packages.${pkgs.system}.default
];
```

### 3. Compositor configuration

### Niri

In your Niri config:

```kdl
spawn-at-startup "gpuishell"

binds {
    Mod+Return hotkey-overlay-title="Open Launcher" { spawn "gpuishell"; }
}
```

### Hyprland

In your Hyprland config:

```ini
exec-once = gpuishell
bind = $mainMod, Return, exec, gpuishell
```

### 4. Configuration

On first launch, a default config file is created at `~/.config/gpuishell/config.toml`. Edit it to customize the shell. See the [Configuration Reference](/gpui-shell/reference/configuration/) for details.

For example, to change bar widget positions:

```toml
[bar]
position = "top"
start = ["LauncherBtn", "Workspaces"]
center = ["Clock"]
end = ["Systray", "Battery", "Settings"]
```
