pub mod coordinator;

use coordinator::{Coordinator, Error};
use degen_base_signer::{
    config::Config,
    net::{HttpNet, HttpNetListen},
};

pub const DEVNET_COORDINATOR_ID: u32 = 0;

pub fn create_coordinator_from_path(
    path: impl AsRef<std::path::Path>,
) -> Result<Coordinator<HttpNetListen>, Error> {
    let config = Config::from_path(path)?;
    create_coordinator(&config)
}

pub fn create_coordinator(config: &Config) -> Result<Coordinator<HttpNetListen>, Error> {
    let net: HttpNet = HttpNet::new(config.http_relay_url.clone());
    let net_listen: HttpNetListen = HttpNetListen::new(net, vec![]);
    let coordinator = Coordinator::new(DEVNET_COORDINATOR_ID, config, net_listen)?;
    Ok(coordinator)
}
