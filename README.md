Fork of [raise](https://github.com/svelterust/raise) with little change: classes are found using lower().contains() instead of exact match.
Name changed to be not confused when installed.

# Hyprrun

Run or raise / Jumpapp implemented for Hyprland. It will raise window if it exists,
or cycle to next window if current window matches class to focus. Otherwise
it will launch new window.

```
$ hyprrun
Usage: hyprrun -c <class> -e <launch>

Raise window if it exists, otherwise launch new window.

Options:
  -c, --class       class to focus
  -e, --launch      command to launch
  --help            display usage information
```

## Install `hyprrun`

Add `github:hayshin/hyprrun` as a flake to your NixOS configuration

For NixOS, add hyprrun to your flake inputs:

```nix
inputs = {
  hyprrun.url = "github:hayshin/hyprrun";
};
```

Then add it to your system, for instance: `environment.systemPackages = [hyprrun.defaultPackage.x86_64-linux];`

## Example configuration

I like having <kbd>Super</kbd> + `<key>` bound to run or raise, and <kbd>Super</kbd> + <kbd>Shift</kbd> + `<key>` to launch application regularly.

```
bind = SUPER, V, exec, raise --class "Alacritty" --launch "alacritty"
bind = SUPER_SHIFT, V, exec, alacritty
bind = SUPER, C, exec, raise --class "firefox" --launch "firefox"
bind = SUPER_SHIFT, C, exec, firefox
bind = SUPER, F, exec, raise --class "emacs" --launch "emacsclient --create-frame"
bind = SUPER_SHIFT, F, exec, emacsclient --create-frame
```

## How to find class?

Run `hyprctl clients` while window is open, and look for `class: <class>`.
