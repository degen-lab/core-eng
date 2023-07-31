use clap::Parser;

///Command line interface for stacks coordinator
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Config file path
    /// TODO: pull this info from sBTC
    #[arg(short, long)]
    pub config: String,

    /// Optional starting block height to use.
    /// Will override any listed value within the config file
    /// Must be greater than 0.
    #[arg(short = 'b', long)]
    pub start_block_height: Option<u64>,

    /// Signer Config file path
    /// TODO: this should not be a seperate option really
    #[arg(short, long)]
    pub signer_config: String,

    /// Subcommand to perform
    #[clap(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand, Debug)]
pub enum Command {
    // Listen for incoming peg in and peg out requests.
    Run,
    // Run distributed key generation round
    Dkg,
    // Run distributed key generation round then sign a message
    DkgSign,
}
