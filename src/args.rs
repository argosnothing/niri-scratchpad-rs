use clap::{Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Subcommand, Debug, Serialize, Deserialize)]
pub enum Action {
    #[command(about = "Target a window by app id or title.")]
    Target {
        #[command(subcommand)]
        property: Property,
        #[arg(
            long,
            help = "Spawn the application if no target is found",
            name = "spawn command"
        )]
        spawn: Option<String>,
    },
    Create {
        register_number: i32,
        #[arg(short, long)]
        output: Option<Output>,
        #[arg(
            long,
            help = "Initial register create will toggle floating on the window"
        )]
        as_float: bool,
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

#[derive(Subcommand, Clone, Debug, Serialize, Deserialize)]
pub enum Property {
    #[command(name = "appid")]
    AppId {
        #[arg(name = "string")]
        value: String,
    },
    #[command(name = "title")]
    Title {
        #[arg(name = "string")]
        value: String,
    },
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
