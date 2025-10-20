Fork of [raise](https://github.com/neg-serg/raise) with little change: classes are found using lower().contains() instead of exact match by default. Also add shorthand for title.

# Hyprrun

Run or raise / Jumpapp implemented for Hyprland. It will raise window if it exists,
or cycle to next window if current window matches class to focus. Otherwise
it will launch new window.

```
$ hyprrun
Usage: hyprrun -e <launch> -m <field[:method]=pattern>

Raise window if it exists, otherwise launch new window.

Options:
  -m, --match       additional matcher in the form field[:method]=pattern
  -e, --launch      command to launch
  -c, --class       class to focus (shorthand for `--match class:contains=...`)
  -c, --title       title to focus (shorthand for `--match title:contains=...`)
  --help            display usage information
```

### Matching

The `--match` flag allows choosing how a window should be selected. Each
matcher uses the format `field[:method]=pattern` and multiple matchers can be
combined; they all have to match for a window to qualify.

Supported fields:
- `class` — current window class reported by Hyprland
- `initial-class` — class when the window was first created
- `title` — current window title
- `initial-title` — original title assigned on window creation
- `tag` — window tag assigned via dynamic tags
- `xdgtag` — XDG surface tag (`xdgTag` in `hyprctl clients`)

Aliases: you can also use the short forms `c`, `initialClass`, `initialTitle`, and `xdg-tag`.

Supported methods (default is `contains`):
- `equals` / `eq`
- `contains` / `substr`
- `prefix` / `starts-with`
- `suffix` / `ends-with`
- `regex` / `re`

Examples:

```
raise --launch firefox --match class=firefox
raise --launch alacritty --match title:contains=notes
raise --launch slack --match class=Slack --match title:regex="(?i)daily"
```

## Install `hyprrun`

Add `github:hayshin/hyprrun` as a flake to your NixOS configuration

For NixOS, add hyprrun to your flake inputs:

```nix
inputs = {
  hyprrun.url = "github:hayshin/hyprrun";
};
```

Then add it to your system, for instance: `home.packages = [inputs.hyprrun.defaultPackage.x86_64-linux];`

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

## How to find window information and fields?

Run `hyprctl clients` while window is open, and look for `field: <field>`.
