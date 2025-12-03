use clap::{Parser, Subcommand, ValueEnum};

#[derive(Subcommand, Debug)]
pub enum Action {
    Create {
        scratchpad_number: i32,
        #[arg(short, long)]
        output: Option<Output>,
    },
    Delete {
        scratchpad_number: i32,
        #[arg(short, long)]
        output: Option<Output>,
    },
}

#[derive(ValueEnum, Clone, Debug)]
#[value(rename_all = "lowercase")]
pub enum Output {
    Title,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub action: Action,
}
