# Dynamic Niri Scratchpad

Dynamically assign windows as scratchpads against numerical register.

```kdl
    workspace "stash" {
        open-on-output "DP-1" // Your output name, or just omit this property entirely
    }
```

For binding to a keybind you would do: 
```kdl
    Mod+Q            { spawn "niri-scratchpad" "create" "1"; }
```
This will take the currently focused window and bind it to niri-scratchpad register 1. Pressing this keybind again will move the scratchpad to the stash workspace. 

A separate command is available for removing a scratchpad at a particular register. 
```kdl
    Mod+Ctrl+Q            { spawn "niri-scratchpad" "delete" "2"; }
```
delete scratchpad at 1 register

## Installation
It's just a rust binary, you'll need to build it with niri_ipc crate and serde. I provide a flake you can also consume as an input. 
```nix
    niri-scratchpad.url = "github:argosnothing/niri-scratchpad";
```

## Roadmap
Assuming niri doesn't implement scratchpads natively ( We all pray ), by priority: 

1. Scratchpad deletion 
    * Windows tracked as a scratchpad should also be deleted from the tracked scratchpad list, freeing a spot. 
2. Spawn support
  This would require a diff mechanism than indexes, as spawn would need to have the command as part of the arg and i'd need to do matching off title or app_id
