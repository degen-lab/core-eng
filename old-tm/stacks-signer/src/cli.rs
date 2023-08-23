use crate::secp256k1::Secp256k1;
use clap::{Parser, Subcommand};

///Command line interface for stacks signer
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Subcommand action to take
    #[clap(subcommand)]
    pub command: Command,
}

/// Possible actions that stacks signer can perform
#[derive(Subcommand)]
pub enum Command {
    /// Join the p2p network as specified in the config file
    Run {
        /// Associated signer id
        #[arg(short, long)]
        id: u32,
        /// Config file path
        #[arg(short, long)]
        config: String,
    },
    /// Generate Secp256k1 Private Key
    PrivateKey(Secp256k1),
    /// Generate Secp256k1 Public Key
    PublicKey {
        /// Config file path
        #[arg(short, long)]
        config: String,
    },
}
