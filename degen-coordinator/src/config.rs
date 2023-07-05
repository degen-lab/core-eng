// TODO: Set appropriate types
type ContractIdentifier = String;
type StacksPrivateKey = String;
type Url = String;

/// Errors associated with reading the Config file
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("Toml Error: {0}")]
    TomlError(#[from] toml::de::Error),
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Testnet,
}

#[derive(serde::Deserialize)]
pub struct Config {
    pub sbtc_contract: ContractIdentifier,
    pub stacks_private_key: StacksPrivateKey,
    pub stacks_node_rpc_url: Url,
    pub bitcoin_node_rpc_url: Url,
    pub frost_dkg_round_id: u64,
    pub signer_config_path: String,
    pub start_block_height: Option<u64>,
    pub rusqlite_path: Option<String>,
    /// The network version we are using ('mainnet' or 'testnet'). Default: 'mainnet'
    pub network: Option<Network>,
    /// The transaction fee in Satoshis used to broadcast transactions to the stacks node
    pub transaction_fee: u64,
}

impl Config {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Self, Error> {
        Ok(toml::from_str(&std::fs::read_to_string(path)?)?)
    }
}
