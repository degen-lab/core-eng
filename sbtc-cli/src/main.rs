use clap::Parser;

use crate::commands::broadcast::{broadcast_tx, BroadcastArgs};
use crate::commands::deposit::{build_deposit_tx, DepositArgs};
use crate::commands::generate::{generate, GenerateArgs};
use crate::commands::withdraw::{build_withdrawal_tx, WithdrawalArgs};

mod commands;

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(clap::Subcommand, Debug, Clone)]
enum Command {
    Deposit(DepositArgs),
    Withdraw(WithdrawalArgs),
    Broadcast(BroadcastArgs),
    GenerateFrom(GenerateArgs),
}

fn main() -> Result<(), anyhow::Error> {
    let args = Cli::parse();

    match args.command {
        Command::Deposit(deposit_args) => build_deposit_tx(&deposit_args),
        Command::Withdraw(withdrawal_args) => build_withdrawal_tx(&withdrawal_args),
        Command::Broadcast(broadcast_args) => broadcast_tx(&broadcast_args),
        Command::GenerateFrom(generate_args) => generate(&generate_args),
    }
}
