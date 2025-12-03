# Niri Scratchpad

## This is currently a work in progress, but is technically functional for how I use scratchpads. 

Niri scratchpad simply uses the currently focused window to mark a window as a scratchpad when ran. This means this is only really functional when using the scratchpad as a keybind. 

You also will need to add this to your niri config: 

```kdl
    workspace "stash" {
        open-on-output "DP-1" // insert the output you actually have here. you can also probably just omit this as well. 
    }
    window-rule {
        open-on-workspace "stash"
        open-floating true
        default-floating-position x=16 y=0 relative-to="left"
        default-column-width  { fixed 920; }
        default-window-height { fixed 920; } // ~80% of 1080
    }
```

For binding to a keybind you would do: 
```kdl
    Mod+Q            { spawn "niri-scratchpad" "1"; }
```
This will take the currently focused window and bind it to niri-scratchpad index 1. Pressing this keybind again will move the scratchpad to the stash workspace. 


## Removing Scratchpads
Currently scratchpad data is saved to `XDG_RUNTIME_DIR` under `niri-scratchpad.json`. Currently the only way to remove a scratchpad would be to simply delete that file and readd scratches.

## Installation
It's just a rust binary, you'll need to build it with niri_ipc crate and serde. I provide a flake you can also consume as an input. 
```nix
    niri-scratchpad.url = "github:argosnothing/niri-scratchpad";
```

## Roadmap
Assuming niri doesn't implement scratchpads natively ( We all pray ), by priority: 

1. Flag arguments ( currently scratchpad simply parses the first argument as a scratchpad index. )
2. Scratchpad deletion 
  * Typing `niri-scratchpad -d 1` should unbind the window from that scratchpad index.
  * Windows tracked as a scratchpad should also be deleted from the tracked scratchpad list, freeing a spot. 
4. Spawn support
  This would require a diff mechanism than indexes, as spawn would need to have the command as part of the arg and i'd need to do matching off title or app_id
