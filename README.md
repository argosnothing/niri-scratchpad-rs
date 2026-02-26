# Niri Scratchpad 2.0
No config files required.

![tmp XVc2CNDYYc](https://github.com/user-attachments/assets/a1cf8329-61da-423a-a362-17a6a06274d2)

You must define a static workspace named `stash` in your Niri configuration.  
All stashed scratchpads are moved here.

```kdl
workspace "stash" { }
```

---

## Static Scratchpads

Static scratchpads show and hide windows based on properties such as **appid** or **title**.  
If multiple windows match, all matches are selected.

No background process is required when using only static scratchpads.

### Example Niri bindings

Toggle Firefox by app id:

```kdl
binds {
    Mod+F { spawn "niri-scratchpad" "target" "appid" "firefox"; }
}
```

Toggle by window title:

```kdl
binds {
    Mod+S { spawn "niri-scratchpad" "target" "title" "Spotify"; }
}
```

Spawn if not running:

```kdl
binds {
    Mod+Return {
        spawn "niri-scratchpad" "target" "appid" "alacritty" "--spawn" "alacritty";
    }
}
```

---

## Dynamic Scratchpads

Dynamic scratchpads assign a window to a numbered register.  
You can toggle the window with a keybind using that register.

This mode requires the daemon to track register state.

Start the daemon on login:

```kdl
spawn-sh-at-startup "niri-scratchpad daemon"
```

### Example Niri bindings

Create / toggle register 1:

```kdl
binds {
    Mod+Q { spawn "niri-scratchpad" "create" "1"; }
}
```

Delete register 1:

```kdl
binds {
    Mod+Ctrl+Q { spawn "niri-scratchpad" "delete" "1"; }
}
```

Create as floating:

```kdl
binds {
    Mod+Shift+Q { spawn "niri-scratchpad" "create" "1" "--as-float"; }
}
```

---

## Command Interface

### Static Targeting

| Command | Description |
|--------|-------------|
| `target appid <app_id>` | Match window(s) by app id |
| `target title <window_title>` | Match window(s) by title |

#### Options

| Option | Description |
|--------|-------------|
| `--spawn <command>` | Spawn application if no window matches |
| `--as-float` | Set matched windows to floating |

---

### Dynamic Registers

| Command | Description |
|--------|-------------|
| `create <number>` | Create or toggle scratchpad |
| `delete <number>` | Remove scratchpad and restore window |
| `get <number>` | Query scratchpad information |
| `daemon` | Start background daemon |

#### Options

| Option | Description |
|--------|-------------|
| `-o, --output [title\|appid]` | Print selected property to stdout |
| `--as-float` | Set window to floating when registering |

---

## Installation

This is a single Rust binary.

Prebuilt `x86_64` binaries are provided.  
Download, place in your PATH, and make executable:

```bash
chmod +x niri-scratchpad
```

---

## Building

Dependencies:

- rust
- cargo
- niri-ipc
- serde
- clap

Build:

```bash
cargo build --release
```

---

## Nix (Flakes)

Add to inputs:

```nix
inputs {
  niri-scratchpad.url = "github:argosnothing/niri-scratchpad";
}
```

Add to system packages:

```nix
environment.systemPackages = [
  inputs.niri-scratchpad.packages.${pkgs.system}.default
];
```

---

## Related Resources

- Static scratchpads via spawn command:  
  https://github.com/gvolpe/niri-scratchpad

- Niri scratchpad discussion:  
  https://github.com/YaLTeR/niri/discussions/329

---

## Hidden Workspace Experimental Branch

Experimental branch:

https://github.com/argosnothing/niri-scratchpad-rs/tree/hidden-workspaces

Draft Niri PR:

https://github.com/YaLTeR/niri/pull/2997

This approach hides the stash workspace while maintaining full scratchpad functionality.

To use:

- set `hidden true` on the stash workspace
- use `niri msg workspaces-with-hidden` instead of `workspaces`

Hidden workspaces are not advertised to the event stream, so bars and workspace widgets remain unaffected.

Feedback is welcome if you try this branch.
