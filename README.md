# Dynamic Niri Scratchpad
Dynamically assign windows as scratchpads against a numerical register. 

## Setup
You will need a static workspace called `stash` declared somewhere in you niri config. This will be where all stashed scratchpads live.
```kdl
    workspace "stash" {
        open-on-output "DP-1" // Your output name, or just omit this property entirely
    }
```

For binding to a keybind you would do: 
```kdl
    Mod+Q            { spawn "niri-scratchpad" "create" "1"; }
```
* This will take the currently focused window and bind it to niri-scratchpad register 1. 
* Pressing this keybind again will toggle stashing and unstashing the window when this command is reran. 


A separate command is available for removing a scratchpad at a particular register. 
```kdl
    Mod+Ctrl+Q            { spawn "niri-scratchpad" "delete" "1"; }
```
* delete scratchpad at register 1
* this register will now be available again for the `niri-scratchpad create 1` command

## Command interface: 
* `niri-scratchpad create <scratchpad_number>`
  * options
    * `-o, --output [title, appid]`
    * `--as-float`
* `niri-scratchpad delete <scratchpad_number>`
  * options
    * `-o, --output [title, appid]`
* `niri-scratchpad get <scratchpad_number> [title, appid]`
* `help`
  
`--output` provides the property to standard out after command execution. 

## Installation
It's just a rust binary, you'll need to build it with `niri_ipc` `serde` and `clap` crates. I provide a flake you can also consume as an input. 
```nix
inputs {
    niri-scratchpad.url = "github:argosnothing/niri-scratchpad";
}
```

To put it in your path on nix:
```nix

    environment.systemPackages = [
      inputs.niri-scratchpad.packages.${pkgs.system}.default
    ];
```

# Roadmap for 1.0 Release

1. `--retrieve` option for scratchpad deletion
    1. `niri-scratchpad delete 1 --retrieve` will retrieve the window from that scratchpad register, focusing it to the output.
    2. `remove-float` flag will remove floating on the window after retrieval on a deleted scratchpad window.
2. `niri-scratchpad spawn <scratchpad_number> <cmd_str>` action
    1. Instead of relying on current focused window for the scratchpads window id, this will spawn a window using the command, and then take that windows id as the scratchpad window id. This will be different than several other implementations that force the command to also keep track of title or appid. We will still be using the actual unique window id, so there will not be a need to type a scratchpad register to those properties.
    2. If an existing window is still in the register when spawn is called again, it will pull the scratchpad into focus similar to `niri-scratchpad create`
    3. `--as-float` that will work similar to the option on create.
3.  `niri-scratchpad sync`. Currently state is only updated on the create and delete scratchpad actions. While create will resync for the purpose of reattaching a new window to a register, for the simple purpose of updating state there is no action that does only this. `niri-scratchpad get [appid, title] exists`, but this is only for the purpose of querying the state.
    1. Main usecase will be for the purposes of services that would periodically run sync to make sure the scratchpad appid and statuses match the `get` actions for the purpose of UI display even if a window is deleted. 
