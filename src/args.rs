use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(clap::Args, Debug, Clone, Serialize, Deserialize)]
pub struct ScratchpadOpts {
    #[arg(long, help = "Set window to floating")]
    pub as_float: bool,
    #[arg(long, help = "Animate window if floating")]
    pub animations: bool,
    #[arg(long, help = "Scratchpad follows across workspace changes")]
    pub follow: bool,
    #[arg(long, help = "Stash other scratchpads when summoned")]
    pub exclusive: bool,
}

#[derive(Subcommand, Debug, Serialize, Deserialize)]
pub enum Action {
    #[command(about = "Target a window by app id or title.")]
    Target {
        property: PropertyKind,
        value: String,
        #[arg(
            long,
            help = "Spawn the application if no target is found",
            name = "spawn command"
        )]
        spawn: Option<String>,
        #[command(flatten)]
        opts: ScratchpadOpts,
    },
    Create {
        register_number: i32,
        #[arg(short, long)]
        output: Option<Output>,
        #[command(flatten)]
        opts: ScratchpadOpts,
    },
    Delete {
        register_number: i32,
        #[arg(short, long)]
        output: Option<Output>,
    },
    Get {
        register_number: i32,
        output: Output,
    },
    Sync,
    Daemon,
}

#[derive(ValueEnum, Clone, Debug, PartialEq, Serialize, Deserialize)]
#[value(rename_all = "lowercase")]
pub enum PropertyKind {
    AppId,
    Title,
}

#[derive(ValueEnum, Clone, Debug, Serialize, Deserialize)]
#[value(rename_all = "lowercase")]
pub enum Output {
    Title,
    AppId,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub action: Action,
}
