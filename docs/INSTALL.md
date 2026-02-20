# Installation (NixOS)

GPUi Shell is packaged as a Nix flake. Follow the steps below to add it to your NixOS configuration.

## 1. Add the flake input

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

## 2. Install the package

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

## 3. Compositor configuration

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
