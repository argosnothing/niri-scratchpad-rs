# Dynamic Niri Scratchpad

Dynamically assign windows as scratchpads against numerical register. The scratchpad will also manage windows that have been deleted since running the command last. If you invoke `niri-scratchpad create 1` and 1 register has been deleted since last excution, it will bind the currently focused window to that register instead as a scratchpad. 

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
    Mod+Ctrl+Q            { spawn "niri-scratchpad" "delete" "1"; }
```
delete scratchpad at register 1

## Installation
It's just a rust binary, you'll need to build it with niri_ipc crate and serde. I provide a flake you can also consume as an input. 
```nix
    niri-scratchpad.url = "github:argosnothing/niri-scratchpad";
```

