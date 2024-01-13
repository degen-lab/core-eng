use clap::Parser;
use hashbrown::HashMap;
use p256k1::{
    ecdsa::{self, KeyError},
    scalar::{Error as ScalarError, Scalar},
};

use serde::Deserialize;
use std::{fs, thread, time};
use std::io::Write;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use bincode::config;
use bitcoin::{KeyPair, XOnlyPublicKey};
use bitcoin::secp256k1::{Secp256k1, SecretKey};
use chrono::Local;
use stackslib::address::AddressHashMode;
use stackslib::burnchains::Address;
use stackslib::chainstate::stacks::{StacksPrivateKey, StacksTransaction, TransactionVersion};
use stackslib::types::chainstate::{StacksAddress, StacksPublicKey};
use stackslib::vm::ContractName;
use stackslib::vm::types::PrincipalData;
use p256k1::field::P;
use toml;
use tracing::info;
use url::Url;
use stackslib::burnchains::bitcoin::address::BitcoinAddress;
use crate::bitcoin_node::{BitcoinNode, LocalhostBitcoinNode};
use crate::bitcoin_wallet::BitcoinWallet;
use crate::peg_wallet::BitcoinWallet as BitcoinWalletTrait;
use crate::stacks_node::client::NodeClient;
use crate::stacks_node::StacksNode;
use crate::stacks_wallet::StacksWallet;
use crate::peg_wallet::StacksWallet as PegWallet;

// import type Bitcoin PrivateKey
// import type Bitcoin xOnlyPubKey and address
use crate::util_versioning::address_version;
use crate::util::parse_public_key;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    IO(#[from] std::io::Error),
    #[error("{0}")]
    Toml(#[from] toml::de::Error),
    #[error("Invalid Public Key: {0}")]
    InvalidPublicKey(KeyError),
    #[error("Failed to parse network_private_key: {0}")]
    InvalidPrivateKey(ScalarError),
    #[error("Invalid Key ID. Must specify Key IDs greater than 0.")]
    InvalidKeyID,
    #[error("Failed to parse stacks_private_key: {0}")]
    InvalidStacksPrivateKey(String),
    #[error("Failed to parse bitcoin_private_key: {0}")]
    InvalidBitcoinPrivateKey(String),
    #[error("Invalid config url. {0}")]
    InvalidConfigUrl(String),
    #[error("Invalid contract. {0}")]
    InvalidContract(String),
    #[error("Invalid amount. {0}")]
    AmountTooLow(String),
}

// status enum: "is-miner") || (ok "is-waiting") || (ok "is-pending") || (ok "is-none")))))
// 'Miner' | 'Pending' | 'Waiting' | 'NormalUser';
#[derive(Clone, Debug, PartialEq)]
pub enum MinerStatus {
    Miner,
    Pending,
    Waiting,
    NormalUser,
    NoStatus,
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    /// Config file path
    #[arg(short, long)]
    pub config: String,

    /// Start a signing round
    #[arg(short, long)]
    pub start: bool,

    /// ID associated with signer
    #[arg(short, long)]
    pub id: u32,
}

#[derive(Clone, Deserialize, Default, Debug)]
struct RawSigners {
    pub public_key: String,
    pub key_ids: Vec<u32>,
}

#[derive(Clone, Deserialize, Default, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Testnet,
    #[default]
    Regtest,
}
#[derive(Clone, Deserialize, Default, Debug)]
struct RawConfig {
    pub mining_contract: String,
    pub exchange_contract: String,
    pub stacks_private_key: String,
    pub stacks_node_rpc_url: String,
    pub bitcoin_private_key: String,
    pub bitcoin_node_rpc_url: String,
    /// The transaction fee in Satoshis used to broadcast transactions to the stacks node
    pub transaction_fee: u64,
    pub amount_to_script: u64,
    pub fee_to_script: u64,
    pub fee_to_pox: u64,
    pub network: Network,
    pub http_relay_url: String,
    pub keys_threshold: u32,
    pub network_private_key: String,
    signers: Vec<RawSigners>,
    coordinator_public_key: String,
}

pub type SignerKeyIds = HashMap<u32, Vec<u32>>;

impl RawConfig {
    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<RawConfig, Error> {
        let content = fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    pub fn public_keys(&self) -> Result<PublicKeys, Error> {
        let mut public_keys = PublicKeys::default();
        for (i, s) in self.signers.iter().enumerate() {
            let signer_public_key =
                parse_public_key(&s.public_key).map_err(Error::InvalidPublicKey)?;
            for key_id in &s.key_ids {
                //We do not allow a key id of 0.
                if *key_id == 0 {
                    return Err(Error::InvalidKeyID);
                }
                public_keys.key_ids.insert(*key_id, signer_public_key);
            }
            //We start our signer and key IDs from 1 hence the + 1;
            let signer_key = u32::try_from(i).unwrap() + 1;
            public_keys.signers.insert(signer_key, signer_public_key);
        }
        Ok(public_keys)
    }

    pub fn signer_key_ids(&self) -> SignerKeyIds {
        let mut signer_key_ids = SignerKeyIds::default();
        for (i, s) in self.signers.iter().enumerate() {
            signer_key_ids.insert((i + 1).try_into().unwrap(), s.key_ids.clone());
        }
        signer_key_ids
    }

    pub fn coordinator_public_key(&self) -> Result<ecdsa::PublicKey, Error> {
        parse_public_key(&self.coordinator_public_key).map_err(Error::InvalidPublicKey)
    }

    pub fn network_private_key(&self) -> Result<Scalar, Error> {
        let network_private_key = Scalar::try_from(self.network_private_key.as_str())
            .map_err(Error::InvalidPrivateKey)?;
        Ok(network_private_key)
    }

    pub fn parse_stacks_private_key(&self) -> Result<(StacksPrivateKey, StacksAddress), Error> {
        let sender_key = StacksPrivateKey::from_hex(&self.stacks_private_key)
            .map_err(|e| Error::InvalidStacksPrivateKey(e.to_string()))?;
        let pk = StacksPublicKey::from_private(&sender_key);
        let address = StacksAddress::from_public_keys(
            address_version(&self.parse_version().0),
            &AddressHashMode::SerializeP2PKH,
            1,
            &vec![pk],
        )
            .ok_or(Error::InvalidStacksPrivateKey(
                "Failed to generate stacks address from private key".to_string(),
            ))?;
        Ok((sender_key, address))
    }
    pub fn parse_bitcoin_private_key(&self) -> Result<(SecretKey, XOnlyPublicKey), Error> {
        let secp = Secp256k1::new();
        let sender_key = SecretKey::from_str(&self.bitcoin_private_key)
            .map_err(|e| Error::InvalidBitcoinPrivateKey(e.to_string()))?;
        let key_pair_source = KeyPair::from_secret_key(&secp, &sender_key);
        let (xonly_public_key, _) = key_pair_source.x_only_public_key();
        Ok((sender_key, xonly_public_key))
    }
    pub fn parse_version(&self) -> (TransactionVersion, bitcoin::Network) {
        // Determine what network we are running on
        match self.network {
            Network::Mainnet => (TransactionVersion::Mainnet, bitcoin::Network::Bitcoin),
            // TODO: change the network back to Testnet
            Network::Testnet => (TransactionVersion::Testnet, bitcoin::Network::Regtest),
            Network::Regtest => (TransactionVersion::Testnet, bitcoin::Network::Regtest),
        }
    }
    pub fn parse_contract(&self) -> Result<((ContractName, StacksAddress), (ContractName, StacksAddress)), Error> {
        let mut split = self.mining_contract.split('.');
        let mut split2 = self.exchange_contract.split('.');
        let mining_address = split
            .next()
            .ok_or(Error::InvalidContract("Missing address".to_string()))?;
        let mining_name = split
            .next()
            .ok_or(Error::InvalidContract("Missing name.".to_string()))?
            .to_owned();
        let exchange_address = split2
            .next()
            .ok_or(Error::InvalidContract("Missing address".to_string()))?;
        let exchange_name = split2
            .next()
            .ok_or(Error::InvalidContract("Missing name.".to_string()))?
            .to_owned();
        let mining_address = StacksAddress::from_string(mining_address)
            .ok_or(Error::InvalidContract("Bad contract address.".to_string()))?;
        let mining_name = ContractName::try_from(mining_name)
            .map_err(|e| Error::InvalidContract(format!("Bad contract name: {}.", e)))?;
        let exchange_address = StacksAddress::from_string(exchange_address)
            .ok_or(Error::InvalidContract("Bad contract address.".to_string()))?;
        let exchange_name = ContractName::try_from(exchange_name)
            .map_err(|e| Error::InvalidContract(format!("Bad contract name: {}.", e)))?;
        Ok(((mining_name, mining_address), (exchange_name, exchange_address)))
    }

}

#[derive(Default, Clone, Debug)]
pub struct PublicKeys {
    pub signers: HashMap<u32, ecdsa::PublicKey>,
    pub key_ids: HashMap<u32, ecdsa::PublicKey>,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub contract_name: ContractName,
    pub contract_address: StacksAddress,
    pub stacks_private_key: StacksPrivateKey,
    pub stacks_address: StacksAddress,
    pub stacks_node_rpc_url: Url,
    pub local_stacks_node: NodeClient,
    pub stacks_wallet: StacksWallet,
    pub stacks_version: TransactionVersion,
    pub bitcoin_private_key: SecretKey,
    pub bitcoin_xonly_public_key: XOnlyPublicKey,
    pub bitcoin_node_rpc_url: Url,
    pub local_bitcoin_node: LocalhostBitcoinNode,
    pub bitcoin_wallet: BitcoinWallet,
    pub transaction_fee: u64,
    pub bitcoin_network: bitcoin::Network,
    pub http_relay_url: String,
    pub keys_threshold: u32,
    pub amount_to_script: u64,
    pub fee_to_script: u64,
    pub fee_to_pox: u64,
    pub network_private_key: Scalar,
    pub public_keys: PublicKeys,
    pub signer_key_ids: SignerKeyIds,
    pub coordinator_public_key: ecdsa::PublicKey,
    pub total_signers: u32,
    pub total_keys: u32,
    pub status: MinerStatus,
    pub pox_transactions_block_heights: Arc<Mutex<Vec<u64>>>,
    pub fund_each_block: bool,
}

impl Config {
    pub fn new(
        contract_name: ContractName,
        contract_address: StacksAddress,
        stacks_private_key: StacksPrivateKey,
        stacks_address: StacksAddress,
        stacks_node_rpc_url: Url,
        local_stacks_node: NodeClient,
        stacks_wallet: StacksWallet,
        stacks_version: TransactionVersion,
        bitcoin_private_key: SecretKey,
        bitcoin_xonly_public_key: XOnlyPublicKey,
        bitcoin_node_rpc_url: Url,
        local_bitcoin_node: LocalhostBitcoinNode,
        bitcoin_wallet: BitcoinWallet,
        transaction_fee: u64,
        bitcoin_network: bitcoin::Network,
        keys_threshold: u32,
        amount_to_script: u64,
        fee_to_script: u64,
        fee_to_pox: u64,
        coordinator_public_key: ecdsa::PublicKey,
        public_keys: PublicKeys,
        signer_key_ids: SignerKeyIds,
        network_private_key: Scalar,
        http_relay_url: String,
        status: MinerStatus,
        pox_transactions_block_heights: Arc<Mutex<Vec<u64>>>,
        fund_each_block: bool,
    ) -> Config {
        Self {
            contract_name,
            contract_address,
            stacks_private_key,
            stacks_address,
            stacks_node_rpc_url,
            local_stacks_node,
            stacks_wallet,
            stacks_version,
            bitcoin_private_key,
            bitcoin_xonly_public_key,
            bitcoin_node_rpc_url,
            local_bitcoin_node,
            bitcoin_wallet,
            transaction_fee,
            bitcoin_network,
            keys_threshold,
            coordinator_public_key,
            network_private_key,
            http_relay_url,
            amount_to_script,
            fee_to_script,
            fee_to_pox,
            total_signers: public_keys.signers.len().try_into().unwrap(),
            total_keys: public_keys.key_ids.len().try_into().unwrap(),
            public_keys,
            signer_key_ids,
            status,
            pox_transactions_block_heights,
            fund_each_block,
        }
    }

    pub fn from_path(path: impl AsRef<std::path::Path>) -> Result<Config, Error> {
        let raw_config = RawConfig::from_path(path)?;
        Config::try_from(&raw_config)
    }
}

fn operate_address_status_non_miner(
    stacks_wallet: &StacksWallet,
    stacks_node: &mut NodeClient,
    stacks_address: &StacksAddress,
    bitcoin_pubkey: &XOnlyPublicKey,
) -> MinerStatus {
    let mut current_status = stacks_node.get_status(stacks_address).unwrap_or(MinerStatus::NoStatus);

    // TODO: degens - delete these after querying for mempool done
    let mut not_in_waiting_mempool = true;
    let mut not_in_pending_mempool = true;
    let mut not_in_mempool_to_be_miner = true;

    let mut nonce = stacks_node.next_nonce(stacks_address).unwrap_or(0);

    // hashbytes for the bitcoin p2pkh address
    // types.tuple({
    // version - link // https://github.com/stacksgov/sips/blob/feat/sip-015/sips/sip-015/sip-015-network-upgrade.md#new-method-get-burn-block-info
    //     version: types.buff(hash160(buffer_from('00'))),
    //     hashbytes: types.buff(hash160(buffer_from(publicKeyHex))),
    // })
    while current_status != MinerStatus::Miner {
        match current_status {
            MinerStatus::NormalUser => {
                // TODO: degens - query the mempool
                if not_in_waiting_mempool {
                    match stacks_wallet.ask_to_join(nonce, bitcoin_pubkey.serialize().to_vec().clone())
                    {
                        Ok(tx) => {
                            match stacks_node.broadcast_transaction(&tx) {
                                Ok(()) => {
                                    info!("The tx for ask-to-join: {:#?}", tx);
                                    not_in_waiting_mempool = false;
                                    nonce += 1;
                                }
                                Err(e) => {
                                    info!("Failed to broadcast ask-to-jon transaction: {:?}", e);
                                }
                            }
                        }
                        Err(e) => {
                            info!("Failed to constuct ask-to-join transaction: {:?}", e);
                        }
                    }
                }
            }
            MinerStatus::Waiting => {
                let enough_voted = stacks_node.is_enough_voted_to_enter(stacks_address).unwrap_or(false);
                if enough_voted {
                    if not_in_pending_mempool {
                        match stacks_wallet.call_try_enter(nonce) {
                            Ok(tx) => {
                                match stacks_node.broadcast_transaction(&tx) {
                                    Ok(()) => {
                                        info!("The tx for try-enter: {:#?}", tx);
                                        not_in_pending_mempool = false;
                                        nonce += 1;
                                    }
                                    Err(e) => {
                                        info!("Failed to broadcast try-enter transaction: {:?}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                info!("Failed to constuct try-enter transaction: {:?}", e);
                            }
                        }
                    }
                }
            }
            MinerStatus::Pending => {
                let enough_blocks_passed = stacks_node.is_enough_blocks_passed_for_pending_miners(stacks_address).unwrap_or(false);
                if enough_blocks_passed {
                    if not_in_mempool_to_be_miner {
                        match stacks_wallet.add_pending_miners_to_pool(nonce) {
                            Ok(tx) => {
                                match stacks_node.broadcast_transaction(&tx) {
                                    Ok(()) => {
                                        info!("The tx for add-pending-miners-to-pool: {:#?}", tx);
                                        not_in_mempool_to_be_miner = false;
                                        nonce += 1;
                                    }
                                    Err(e) => {
                                        info!("Failed to broadcast add-pending-miners-to-pool transaction: {:?}", e);
                                    }
                                }
                            }
                            Err(e) => {
                                info!("Failed to constuct add-pending-miners-to-pool transaction: {:?}", e);
                            }
                        }
                    }
                }
            }
            MinerStatus::Miner => {
                break
            }
            MinerStatus::NoStatus => {}
        }
        sleep(time::Duration::from_secs(60));
        current_status = stacks_node.get_status(stacks_address).unwrap_or(MinerStatus::NoStatus);
    }
    current_status
}


impl TryFrom<&RawConfig> for Config {
    type Error = Error;
    fn try_from(raw_config: &RawConfig) -> Result<Self, Error> {
        let (stacks_private_key, stacks_address) = raw_config.parse_stacks_private_key()?;
        let (bitcoin_private_key, bitcoin_xonly_public_key) = raw_config.parse_bitcoin_private_key()?;
        let (stacks_version, bitcoin_network) = raw_config.parse_version();
        let ((mining_name, mining_address), (exchange_name, exchange_address)) = raw_config.parse_contract().unwrap();
        // TODO: degens - use exchange contract as well
        let stacks_node_rpc_url = Url::parse(raw_config.stacks_node_rpc_url.as_str())
            .map_err(|e|
                Error::InvalidConfigUrl(format!("Invalid stacks_node_rpc_url: {}", e))
            )?;
        let bitcoin_node_rpc_url = Url::parse(raw_config.bitcoin_node_rpc_url.as_str())
            .map_err(|e|
                Error::InvalidConfigUrl(format!("Invalid bitcoin_node_rpc_url: {}", e))
            )?;
        let mut local_stacks_node = NodeClient::new(
            stacks_node_rpc_url.clone(),
            mining_name.clone(),
            mining_address,
        );
        let stacks_wallet = StacksWallet::new(
            mining_name.clone(),
            mining_address,
            stacks_private_key,
            stacks_address,
            stacks_version,
            raw_config.transaction_fee.clone(),
        );
        
        let status = operate_address_status_non_miner(&stacks_wallet, &mut local_stacks_node, &stacks_address, &bitcoin_xonly_public_key);

        let mut stacks_node_clone = local_stacks_node.clone();
        let mut stacks_wallet_clone = stacks_wallet.clone();
        let mut nonce = local_stacks_node.next_nonce(&stacks_address).unwrap_or(0);

        thread::spawn(move || {
            loop {
                match stacks_node_clone.is_auto_exchange(&stacks_address) {
                    Ok(auto_exchange) => {
                        if auto_exchange == true && stacks_node_clone.get_user_balance(&stacks_address).unwrap_or(0) > 1000 * 1000000 {
                            // TODO: degens - build the auto-exchange flow
                        }
                    }
                    Err(e) => {
                        info!("Couldn't get auto exchange value: {:#?}", e);
                    }
                }

                let waiting_list = stacks_node_clone.get_waiting_list(&stacks_address).unwrap_or(vec![]);

                for waiting_miner in waiting_list {
                    match stacks_node_clone.is_blacklisted(&stacks_address, &waiting_miner) {
                        Ok(is_blacklisted) => {
                            if is_blacklisted {
                                info!("{:#?} is blacklisted, skipping.", &waiting_miner.to_string())
                            }
                            else {
                                match stacks_wallet_clone.vote_positive_join_request(nonce, waiting_miner) {
                                    Ok(tx) => {
                                        match stacks_node_clone.broadcast_transaction(&tx) {
                                            Ok(()) => {
                                                info!("Voted to accept {:#?} in pool.", &waiting_miner.to_string());
                                                nonce += 1;
                                            }
                                            Err(e) => {
                                                info!("Couldn't accept miner in pool: {:#?}", e);
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        info!("Error creating vote-positive-join-request transaction: {:?}", e);
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            info!("Error retreiving value for is-blacklisted: {:?}", e);
                        }
                    }
                }

                sleep(time::Duration::from_secs(600));
            }
        });

        let bitcoin_wallet = BitcoinWallet::new(bitcoin_xonly_public_key, bitcoin_network);
        let local_bitcoin_node = LocalhostBitcoinNode::new(bitcoin_node_rpc_url.clone());
        if let Err(e) = local_bitcoin_node.load_wallet(bitcoin_wallet.address()) {
            info!("Couldn't load script address: {:?}", e)
        }

        let number_of_signers = local_stacks_node.get_miners_list(stacks_wallet.address()).unwrap_or(vec![stacks_wallet.address().clone()]).len() as u64 - 1;
        let amount_to_pox = local_stacks_node.get_pool_total_spend_per_block(stacks_wallet.address()).unwrap_or(0) as u64 / number_of_signers;
        let fee_to_script = raw_config.fee_to_script;
        let fee_to_pox = raw_config.fee_to_pox / number_of_signers;
        let total_fees = fee_to_script + fee_to_pox;
        let mut fund_each_block = false;

        if raw_config.amount_to_script == 0 {
            fund_each_block = true;
        } else if raw_config.amount_to_script < amount_to_pox + total_fees {
            return Err(Error::AmountTooLow(format!("The amount you specified is too low in order to send to PoX and cover the fees: {} < {}", raw_config.amount_to_script, amount_to_pox + total_fees)));
        }

        let pox_transactions_block_heights = Arc::new(Mutex::new(Vec::<u64>::new()));
        let pox_tx_block_heights_thread = Arc::clone(&pox_transactions_block_heights);

        thread::spawn(move || {
            loop {
                if let Ok(mut block_heights) = pox_tx_block_heights_thread.lock() {
                    while block_heights.len() > 1 {
                        if block_heights[1] - block_heights[0] > 1 {
                            info!("The coordinator didn't fund the scripts for {} block(s)!", block_heights[1] - block_heights[0] - 1);

                            let mut missing_block_heights = String::new();

                            for i in block_heights[0] + 1..block_heights[1] - 1 {
                                missing_block_heights.push_str(&format!("{}, ", i))
                            };

                            missing_block_heights.push_str((block_heights[1] - 1).to_string().as_str());

                            let log_directory = std::path::Path::new("../degen-base-signer/logs/");

                            if !log_directory.exists() {
                                if let Err(e) = fs::create_dir_all(&log_directory) {
                                    info!("Failed to create directory: {:?}", e);
                                }
                            }

                            let file_path = log_directory.join(format!("malicious_coordinator_no_pox_transaction.txt", ));

                            let formatted_date_time = Local::now().format("%d-%m-%Y - %H:%M:%S").to_string();

                            match fs::OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open(&file_path) {
                                Ok(mut file) => {
                                    let log_message = format!("\
                                    Date and time: {:?}\n\
                                    Cause: The coordinator didn't fund the scripts for {} block(s)!\n\
                                    Block heights: {:#?}\n\n\
                                    ============================================\n\n", formatted_date_time, block_heights[1] - block_heights[0] - 1, missing_block_heights);

                                    if let Err(e) = file.write_all(log_message.as_bytes()) {
                                        info!("Couldn't write to file: {:?}", e)
                                    }

                                    if let Err(e) = file.flush() {
                                        info!("Couldn't flush the file: {:?}", e)
                                    }
                                }
                                Err(e) => {
                                    info!("Couldn't create log file: {:?}", e);
                                }
                            }
                        }
                        block_heights.remove(0);
                    }
                }
                sleep(time::Duration::from_secs(60));
            }
        });

        Ok(Config::new(
            mining_name,
            mining_address,
            stacks_private_key,
            stacks_address,
            stacks_node_rpc_url,
            local_stacks_node,
            stacks_wallet,
            stacks_version,
            bitcoin_private_key,
            bitcoin_xonly_public_key,
            bitcoin_node_rpc_url,
            local_bitcoin_node,
            bitcoin_wallet,
            raw_config.transaction_fee,
            bitcoin_network,
            raw_config.keys_threshold,
            raw_config.amount_to_script,
            raw_config.fee_to_script,
            raw_config.fee_to_pox,
            raw_config.coordinator_public_key()?,
            raw_config.public_keys()?,
            raw_config.signer_key_ids(),
            raw_config.network_private_key()?,
            raw_config.http_relay_url.clone(),
            status,
            pox_transactions_block_heights,
            fund_each_block,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::{Config, Error, RawConfig, RawSigners};

    #[test]
    fn try_from_raw_config_test() {
        let mut raw_config = RawConfig::default();

        // Should fail with the default config (require valid private and public keys...)
        assert!(matches!(
            Config::try_from(&raw_config),
            Err(Error::InvalidPublicKey(_))
        ));

        raw_config.coordinator_public_key =
            "22Rm48xUdpuTuva5gz9S7yDaaw9f8sjMcPSTHYVzPLNcj".to_string();
        assert!(matches!(
            Config::try_from(&raw_config),
            Err(Error::InvalidPrivateKey(_))
        ));

        raw_config.network_private_key = "9aSCCR6eirt1NAHwJtSz4HMwBHTyMo62SyPMvVDt5DQn".to_string();
        assert!(Config::try_from(&raw_config).is_ok());
    }

    #[test]
    fn coordinator_public_key_test() {
        let mut config = RawConfig::default();
        // Should fail with an empty public key
        assert!(matches!(
            config.coordinator_public_key(),
            Err(Error::InvalidPublicKey(_))
        ));
        // Should fail with an invalid public key
        config.coordinator_public_key = "Invalid Public Key".to_string();
        assert!(matches!(
            config.coordinator_public_key(),
            Err(Error::InvalidPublicKey(_))
        ));
        // Should succeed with a valid public key
        config.coordinator_public_key = "22Rm48xUdpuTuva5gz9S7yDaaw9f8sjMcPSTHYVzPLNcj".to_string();
        assert!(config.coordinator_public_key().is_ok());
    }

    #[test]
    fn public_keys_test() {
        let mut config = RawConfig::default();
        let public_key = "22Rm48xUdpuTuva5gz9S7yDaaw9f8sjMcPSTHYVzPLNcj".to_string();
        // Should succeed with an empty vector
        let public_keys = config.public_keys().unwrap();
        assert!(public_keys.key_ids.is_empty());
        assert!(public_keys.signers.is_empty());

        // Should fail with an empty public key
        let raw_signer_keys = RawSigners {
            key_ids: vec![1, 2],
            public_key: "".to_string(),
        };
        config.signers = vec![raw_signer_keys];
        assert!(matches!(
            config.public_keys(),
            Err(Error::InvalidPublicKey(_))
        ));

        // Should fail with an invalid public key
        let raw_signer_keys = RawSigners {
            key_ids: vec![1, 2],
            public_key: "Invalid public key".to_string(),
        };
        config.signers = vec![raw_signer_keys];
        assert!(matches!(
            config.public_keys(),
            Err(Error::InvalidPublicKey(_))
        ));

        // Should fail with an invalid key ID
        let raw_signer_keys = RawSigners {
            key_ids: vec![0, 1],
            public_key: public_key.clone(),
        };
        config.signers = vec![raw_signer_keys];
        assert!(matches!(config.public_keys(), Err(Error::InvalidKeyID)));

        // Should succeed with a valid public keys
        let raw_signer_keys1 = RawSigners {
            key_ids: vec![1, 2],
            public_key: public_key.clone(),
        };
        let raw_signer_keys2 = RawSigners {
            key_ids: vec![3, 4],
            public_key,
        };
        config.signers = vec![raw_signer_keys1, raw_signer_keys2];
        let public_keys = config.public_keys().unwrap();
        assert_eq!(public_keys.signers.len(), 2);
        assert_eq!(public_keys.key_ids.len(), 4);
    }

    // test private keys stacks and bitcoin
    #[test]
    fn parse_stacks_private_key_test() {
        let mut config = RawConfig::default();
        // An empty private key should fail
        assert!(matches!(
            config.parse_stacks_private_key(),
            Err(Error::InvalidStacksPrivateKey(_))
        ));
        // An invalid key shoudl fail
        config.stacks_private_key = "This is an invalid private key...".to_string();
        assert!(matches!(
            config.parse_stacks_private_key(),
            Err(Error::InvalidStacksPrivateKey(_))
        ));
        // A valid key should succeed
        config.stacks_private_key =
            "d655b2523bcd65e34889725c73064feb17ceb796831c0e111ba1a552b0f31b3901".to_string();
        assert_eq!(
            config.parse_stacks_private_key().unwrap().0.to_hex(),
            config.stacks_private_key
        );
    }

}
