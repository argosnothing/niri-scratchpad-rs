## Roadmap  
Instead of having an issue for everything I want to do, i'm just going to make a roadmap. 
1. Support for "traditional" static scratchpads https://github.com/argosnothing/niri-scratchpad-rs/issues/5#issue-3972464777  
   * Along with this would bring
     * Create or open
     * spawn on startup
2. `--focus-first`:  
    *  It was a design decision for scratchpad create to always toggle scratch outside of initial create. In the case of multiple layered scratchpads, the user needs to un-toggle the scratchpad, and re-toggle for it to be brought to the forefront. My my use-case this is an okay tradeoff as when i have multiple scratchpads open on my monitor, i won't need to care about what window I have highlighted if i want the toggle to immediately hide the scratchpad. 
    * In the case the user does not want this behavior, the `--focus-first` option will do these things
       1. If the scratchpad is not in the current workspace, it will be summoned normally.
       2. If the scratchpad is in the current workspace, and create is ran, it will toggle the window normally
       3. *In the case that the scratchpad window with <x> is in the current workspace* **AND** it's not the currently focused window when scratchpad create <x> --focus-first, it will instead be focused. This will allow the binding to be used as a way to *jump* to that scratchpad for focus. 
