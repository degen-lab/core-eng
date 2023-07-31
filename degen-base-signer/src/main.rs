use clap::Parser;
use tracing::{error, info, warn};

use degen_base_signer::config::{Cli, Config};
use degen_base_signer::logging;
use degen_base_signer::signer::Signer;

fn main() {
    logging::initiate_tracing_subscriber();

    let cli = Cli::parse();

    match Config::from_path(&cli.config) {
        Ok(config) => {
            let mut signer = Signer::new(config, cli.id);
            info!(
                "{} signer id #{}",
                degen_base_signer::version(),
                signer.signer_id
            ); // sign-on message

            //Start listening for p2p messages
            if let Err(e) = signer.start_p2p_sync() {
                warn!("An error occurred in the P2P Network: {}", e);
            }
        }
        Err(e) => {
            error!("An error occrred reading config file {}: {}", cli.config, e);
        }
    }
}
