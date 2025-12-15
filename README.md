# Dynamic Niri Scratchpad
Dynamically assign windows as scratchpads against a numerical register. 
![tmp XVc2CNDYYc](https://github.com/user-attachments/assets/a1cf8329-61da-423a-a362-17a6a06274d2)


## Setup

niri-scratchpad ( as of version 1.0 ) is a running process that lives in memory.  
```kdl
spawn-sh-at-startup "niri-scratchpad daemon"
```

You will need a static workspace called `stash` declared somewhere in you niri config. This will be where all stashed scratchpads live.  
```kdl
workspace "stash" {
    open-on-output "DP-1" // Your output name, or just omit this property entirely
}
```

For binding to a keybind you would do: 
```kdl
binds {
    Mod+Q            { spawn "niri-scratchpad" "create" "1"; }
}
```
* This will take the currently focused window and bind it to niri-scratchpad register 1. 
* Pressing this keybind again will toggle stashing and unstashing the window when this command is reran. 


A separate command is available for removing a scratchpad at a particular register. 
```kdl
binds {
    Mod+Ctrl+Q            { spawn "niri-scratchpad" "delete" "1"; }
}
```
* Delete scratchpad at register 1
* This register will now be available again for the `niri-scratchpad create 1` command

## Command interface:
* `niri-scratchpad daemon` Start the niri-scratchpad daemon ( I advise to have niri run this command on startup ) 
* `niri-scratchpad create <scratchpad_number>` 
  * *Info*: create **OR** summon a scratchpad window at `<scratchpad_number>`
  * options
    * `-o, --output [title, appid]`
    * `--as-float`
* `niri-scratchpad delete <scratchpad_number>`  
  * *Info*: delete a scratchpad at `<scratchpad_number>`.   
       this deleted scratchpad will have its window summoned to curent workspace
  * options
    * `-o, --output [title, appid]`
* `niri-scratchpad get <scratchpad_number> [title, appid]`
* `help`

`--output` provides the property to standard out after command execution.  
`--as-float` during new scratchpad registration to a window, also put that window into floating mode.

## Installation
It's just a rust binary:  
for `x86_64` I provide the executable directly. Download it, put it somewhere at `chmod +x niri-scratchpad`. Then run it with the options to use it.

## Building
Dependencies:
* `rust`
* `cargo`
* `niri_ipc`
* `serde`
* `clap`

## Nix (flakes)
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

## Extra Resources for scratchpads in niri
* Static scratchpads you can bind with a spawn command [gvolpe](https://github.com/gvolpe/niri-scratchpad)
* [niri scratchpad discussion](https://github.com/YaLTeR/niri/discussions/329)

## Secret Bonus
I have an [experimental branch](https://github.com/argosnothing/niri-scratchpad-rs/tree/hidden-workspaces) that uses a [draft niri PR](https://github.com/YaLTeR/niri/pull/2997) i've been working on.  

I'm daily driving this as it gives me a full scratchpad implementation that hides the workspace where my stashed scratchpads live. This PR should also work with other implementations as long as you make sure you stashed workspace also has `hidden true` in it and you do `niri msg workspaces-with-hidden` instead of just `niri msg workspaces`. This implementation does not advertise hidden workspaces to event stream, and only updates state with hidden workspaces for that specific ipc action, so your bars and widgets that show workspaces will not show those hidden workspaces.  

If you plan on using this please let me know if you run into any bugs, any feedback is welcome! 
