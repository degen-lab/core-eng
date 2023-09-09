use bitcoin::{
    psbt::Prevouts,
    util::{
        base58,
        sighash::{Error as SighashError, SighashCache},
    },
    SchnorrSighashType, XOnlyPublicKey,
};
use blockstack_lib::{types::chainstate::StacksAddress, util::secp256k1::Secp256k1PublicKey};
use frost_coordinator::{
    coordinator::Error as FrostCoordinatorError, create_coordinator, create_coordinator_from_path,
};
use frost_signer::{
    config::Config as SignerConfig,
    net::{Error as HttpNetError, HttpNetListen},
    signing_round::DkgPublicShare,
};
use std::{
    collections::BTreeMap,
    fs::File,
    path::{Path, PathBuf},
    sync::mpsc::RecvError,
    thread::sleep,
    time::Duration,
};
use tracing::{debug, info, warn};
use wsts::{common::Signature, field::Element, taproot::SchnorrProof, Point, Scalar};

use crate::bitcoin_wallet::BitcoinWallet;
use crate::stacks_node::{self, Error as StacksNodeError};
use crate::stacks_wallet::StacksWallet;
use crate::{config::Config, stacks_node::client::BroadcastError};
use crate::{
    peg_wallet::{
        BitcoinWallet as BitcoinWalletTrait, Error as PegWalletError, PegWallet,
        StacksWallet as StacksWalletTrait, WrapPegWallet,
    },
    stacks_wallet::BuildStacksTransaction,
};

// Traits in scope
use crate::bitcoin_node::{
    BitcoinNode, BitcoinTransaction, Error as BitcoinNodeError, LocalhostBitcoinNode,
};
use crate::peg_queue::{
    Error as PegQueueError, PegQueue, SbtcOp, SqlitePegQueue, SqlitePegQueueError,
};
use crate::stacks_node::{client::NodeClient, StacksNode};

type FrostCoordinator = frost_coordinator::coordinator::Coordinator<HttpNetListen>;

/// Helper that uses this module's error type
pub type Result<T> = std::result::Result<T, Error>;

// The max number of nonce retries we should attempt before erroring out
const MAX_NONCE_RETRIES: u64 = 10;

// The max number of retries for invalid fee's we should attempt before erroring out
const MAX_FEE_RETRIES: u64 = 2;

/// Kinds of common errors used by stacks coordinator
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Error occurred with the HTTP Relay
    #[error("Http Network Error: {0}")]
    HttpNetError(#[from] HttpNetError),
    /// Error occurred in the Peg Queue
    #[error("Peg Queue Error: {0}")]
    PegQueueError(#[from] PegQueueError),
    // Error occurred in the Peg Wallet
    #[error("Peg Wallet Error: {0}")]
    PegWalletError(#[from] PegWalletError),
    /// Error occurred in the Frost Coordinator
    #[error("Frost Coordinator Error: {0}")]
    FrostCoordinatorError(#[from] FrostCoordinatorError),
    /// Error occurred in the Sqlite Peg Queue
    #[error("Sqlite Peg Queue Error: {0}")]
    SqlitePegQueueError(#[from] SqlitePegQueueError),
    #[error("Command sender disconnected unexpectedly: {0}")]
    UnexpectedSenderDisconnect(#[from] RecvError),
    #[error("Stacks Node Error: {0}")]
    StacksNodeError(#[from] StacksNodeError),
    #[error("Bitcoin Node Error: {0}")]
    BitcoinNodeError(#[from] BitcoinNodeError),
    #[error("{0}")]
    ConfigError(String),
    #[error("Invalid bitcoin wallet public key: {0}")]
    InvalidPublicKey(String),
    #[error("Error occured during signing: {0}")]
    SigningError(#[from] SighashError),
    #[error("No coordinator set.")]
    NoCoordinator,
    #[error("Max fee retries exceeded.")]
    MaxFeeRetriesExceeded,
    #[error("Max nonce retries exceeded.")]
    MaxNonceRetriesExceeded,
    #[error("Point error: {0}")]
    PointError(String),
}

pub trait Coordinator: Sized {
    type PegQueue: PegQueue;
    type FeeWallet: PegWallet;
    type StacksNode: StacksNode;
    type BitcoinNode: BitcoinNode;

    // Required methods
    fn peg_queue(&self) -> &Self::PegQueue;
    fn fee_wallet_mut(&mut self) -> &mut Self::FeeWallet;
    fn fee_wallet(&self) -> &Self::FeeWallet;
    fn frost_coordinator(&self) -> &FrostCoordinator;
    fn frost_coordinator_mut(&mut self) -> &mut FrostCoordinator;
    fn stacks_node(&self) -> &Self::StacksNode;
    fn stacks_node_mut(&mut self) -> &mut Self::StacksNode;
    fn bitcoin_node(&self) -> &Self::BitcoinNode;

    // Provided methods
    fn run(mut self, polling_interval: u64) -> Result<()> {
        loop {
            info!("Polling for withdrawal and deposit requests to process...");
            self.peg_queue().poll(self.stacks_node())?;
            self.process_queue()?;

            sleep(Duration::from_secs(polling_interval));
        }
    }

    fn process_queue(&mut self) -> Result<()> {
        loop {
            match self.peg_queue().sbtc_op()? {
                Some(SbtcOp::PegIn(op)) => {
                    debug!("Processing peg in request: {:?}", op);
                    self.peg_in(op)?;
                }
                Some(SbtcOp::PegOutRequest(op)) => {
                    debug!("Processing peg out request: {:?}", op);
                    self.peg_out(op)?;
                }
                None => return Ok(()),
            }
        }
    }
}

// Private helper functions
trait CoordinatorHelpers: Coordinator {
    fn peg_in(&mut self, op: stacks_node::PegInOp) -> Result<()> {
        // Build a transaction from the peg in op and broadcast it to the node with reattempts
        self.try_broadcast_transaction(&op)
    }

    fn peg_out(&mut self, op: stacks_node::PegOutRequestOp) -> Result<()> {
        // First build both the sBTC and BTC transactions before attempting to broadcast either of them
        // This ensures that if either of the transactions fail to build, neither of them will be broadcast

        // Build and sign a fulfilled bitcoin transaction
        let fulfill_tx = self.fulfill_peg_out(&op)?;

        // Build a transaction from the peg out request op and broadcast it to the node with reattempts
        self.try_broadcast_transaction(&op)?;

        // Broadcast the BTC transaction to the Bitcoin node
        self.bitcoin_node().broadcast_transaction(&fulfill_tx)?;
        info!(
            "Broadcasted fulfilled BTC transaction: {}",
            fulfill_tx.txid()
        );
        Ok(())
    }

    fn fulfill_peg_out(&mut self, op: &stacks_node::PegOutRequestOp) -> Result<BitcoinTransaction> {
        // Retreive the utxos
        let utxos = self
            .bitcoin_node()
            .list_unspent(self.fee_wallet().bitcoin().address())?;

        // Build unsigned fulfilled peg out transaction
        let (mut tx, prevouts) = self.fee_wallet().bitcoin().fulfill_peg_out(op, utxos)?;
        let sighash_tx = tx.clone();
        let mut sighash_cache = SighashCache::new(&sighash_tx);
        // Sign the transaction
        for index in 0..tx.input.len() {
            let taproot_sighash = sighash_cache
                .taproot_key_spend_signature_hash(
                    index,
                    &Prevouts::All(&prevouts),
                    SchnorrSighashType::Default,
                )
                .map_err(Error::SigningError)?;
            let (_frost_sig, schnorr_proof) = self
                .frost_coordinator_mut()
                .sign_message(&taproot_sighash.as_hash())?;

            debug!(
                "Fulfill Tx {:?} SchnorrProof ({},{})",
                &tx, schnorr_proof.r, schnorr_proof.s
            );

            let finalized = schnorr_proof.to_bytes();
            let finalized_b58 = base58::encode_slice(&finalized);
            debug!("CALC SIG ({}) {}", finalized.len(), finalized_b58);

            tx.input[index].witness.push(finalized);
        }
        //Return the signed transaction
        Ok(tx)
    }

    /// Broadcast a transaction to the stacks node, retrying if the nonce is rejected or the fee set too low until a retry limit is reached
    fn try_broadcast_transaction<T: BuildStacksTransaction>(&mut self, op: &T) -> Result<()> {
        // Retrieve the nonce from the stacks node using the sBTC wallet address
        let address = *self.fee_wallet().stacks().address();
        let mut nonce = self.stacks_node_mut().next_nonce(&address)?;
        let mut nonce_retries = 0;
        let mut fee_retries = 0;
        loop {
            // Build a transaction using the peg in op and calculated nonce
            let tx = self.fee_wallet().stacks().build_transaction(op, nonce)?;

            // Broadcast the resulting sBTC transaction to the stacks node
            match self.stacks_node().broadcast_transaction(&tx) {
                Err(StacksNodeError::BroadcastError(BroadcastError::ConflictingNonceInMempool)) => {
                    warn!("Transaction rejected by stacks node due to conflicting nonce in mempool. Stacks node may be falling behind!");
                    nonce_retries += 1;
                    if nonce_retries > MAX_NONCE_RETRIES {
                        return Err(Error::MaxNonceRetriesExceeded);
                    }
                    warn!("Incrementing nonce and retrying...");
                    nonce = self.stacks_node_mut().next_nonce(&address)?;
                }
                Err(StacksNodeError::BroadcastError(BroadcastError::FeeTooLow(
                    expected,
                    actual,
                ))) => {
                    fee_retries += 1;
                    if fee_retries > MAX_FEE_RETRIES {
                        return Err(Error::MaxFeeRetriesExceeded);
                    }
                    warn!(
                        "Transaction rejected by stacks node due to provided fee being too low: {}",
                        actual
                    );
                    warn!("Incrementing fee to {} and retrying...", expected);
                    self.fee_wallet_mut().stacks_mut().set_fee(expected);
                }
                Err(e) => return Err(e.into()),
                Ok(_) => {
                    info!("Broadcasted sBTC transaction: {}", tx.txid());
                    return Ok(());
                }
            }
        }
    }
}

impl<T: Coordinator> CoordinatorHelpers for T {}

pub enum Command {
    Stop,
    Timeout,
}

pub struct StacksCoordinator {
    frost_coordinator: FrostCoordinator,
    local_peg_queue: SqlitePegQueue,
    local_stacks_node: NodeClient,
    local_bitcoin_node: LocalhostBitcoinNode,
    pub local_fee_wallet: WrapPegWallet,
}

impl StacksCoordinator {
    pub fn run_dkg_round(&mut self) -> Result<XOnlyPublicKey> {
        let p = self.frost_coordinator.run_distributed_key_generation()?;
        XOnlyPublicKey::from_slice(&p.x().to_bytes())
            .map_err(|e| Error::InvalidPublicKey(e.to_string()))
    }

    pub fn sign_message(&mut self, message: &str) -> Result<(Signature, SchnorrProof)> {
        Ok(self.frost_coordinator.sign_message(message.as_bytes())?)
    }
}

fn create_frost_coordinator_from_path(
    signer_config_path: &str,
    config: &Config,
    stacks_node: &mut NodeClient,
    stacks_wallet: &StacksWallet,
) -> Result<FrostCoordinator> {
    debug!("Creating frost coordinator from signer config path...");
    let coordinator = create_coordinator_from_path(signer_config_path).map_err(|e| {
        Error::ConfigError(format!(
            "Invalid signer_config_path {:?}: {}",
            signer_config_path, e
        ))
    })?;

    // Make sure this coordinator data was loaded into the sbtc contract correctly
    let coordinator_data_loaded =
        if let Some(public_key) = stacks_node.coordinator_public_key(&config.stacks_address)? {
            public_key.to_bytes() == coordinator.public_key().to_bytes()
        } else {
            false
        };
    if !coordinator_data_loaded {
        // Load the coordinator data into the sbtc contract
        // TODO: load all contract info into the contract from a file, not just the coordinator data
        // so that subsequent runs of the coordinator don't need to load the data from a file again
        // until a stacking cyle has finished and a new signing set and coordinator are generated.
        debug!("loading coordinator data into sBTC contract...");
        let nonce = stacks_node.next_nonce(&config.stacks_address)?;
        let coordinator_public_key =
            Secp256k1PublicKey::from_slice(&coordinator.public_key().to_bytes())
                .map_err(|e| Error::InvalidPublicKey(e.to_string()))?;
        let coordinator_tx = stacks_wallet.build_set_coordinator_data_transaction(
            &config.stacks_address,
            &coordinator_public_key,
            nonce,
        )?;
        stacks_node.broadcast_transaction(&coordinator_tx)?;
    }
    Ok(coordinator)
}

fn create_frost_coordinator_from_contract(
    config: &Config,
    stacks_node: &mut NodeClient,
) -> Result<FrostCoordinator> {
    debug!("Creating frost coordinator from stacks node...");
    let keys_threshold = stacks_node.keys_threshold(&config.stacks_address)?;
    let coordinator_public_key = stacks_node
        .coordinator_public_key(&config.stacks_address)?
        .ok_or_else(|| Error::NoCoordinator)?;
    let public_keys = stacks_node.public_keys(&config.stacks_address)?;
    let signer_key_ids = stacks_node.signer_key_ids(&config.stacks_address)?;
    let network_private_key = Scalar::try_from(
        config
            .network_private_key
            .clone()
            .unwrap_or(String::new())
            .as_bytes(),
    )
    .map_err(|_| Error::ConfigError("Invalid network_private_key.".to_string()))?;
    let http_relay_url = config.http_relay_url.clone().unwrap_or(String::new());
    create_coordinator(&SignerConfig::new(
        keys_threshold.try_into().unwrap(),
        coordinator_public_key,
        public_keys,
        signer_key_ids,
        network_private_key,
        http_relay_url,
    ))
    .map_err(|e| Error::ConfigError(e.to_string()))
}

fn create_frost_coordinator(
    config: &Config,
    stacks_node: &mut NodeClient,
    stacks_wallet: &StacksWallet,
) -> Result<FrostCoordinator> {
    debug!("Initializing frost coordinator...");
    // Create the frost coordinator and use it to generate the aggregate public key and corresponding bitcoin wallet address
    // Note: all errors returned from create_coordinator relate to configuration issues and should convert to this error type.
    if let Some(signer_config_path) = &config.signer_config_path {
        create_frost_coordinator_from_path(signer_config_path, config, stacks_node, stacks_wallet)
    } else {
        create_frost_coordinator_from_contract(config, stacks_node)
    }
}

fn read_dkg_public_shares(path: impl AsRef<Path>) -> Result<BTreeMap<u32, DkgPublicShare>> {
    let dkg_public_shares_path = path.as_ref().join("dkg_public_shares.json");

    serde_json::from_reader(File::open(&dkg_public_shares_path).map_err(|err| {
        Error::ConfigError(format!(
            "Unable to open DKG public shares file {}: {}",
            dkg_public_shares_path.to_str().unwrap_or("Invalid path"),
            err
        ))
    })?)
    .map_err(|err| Error::ConfigError(format!("Unable to parse DKG public shares JSON: {}", err)))
}

fn write_dkg_public_shares(
    path: impl AsRef<Path>,
    dkg_public_shares: &BTreeMap<u32, DkgPublicShare>,
) -> Result<()> {
    let dkg_public_shares_path = path.as_ref().join("dkg_public_shares.json");

    let dkg_public_shares_file = File::options()
        .create(true)
        .write(true)
        .open(&dkg_public_shares_path)
        .map_err(|err| {
            Error::ConfigError(format!(
                "Unable to open DKG public shares file {}: {}",
                dkg_public_shares_path.to_str().unwrap_or("Invalid path"),
                err
            ))
        })?;

    serde_json::to_writer_pretty(dkg_public_shares_file, dkg_public_shares).map_err(|err| {
        Error::ConfigError(format!(
            "Unable to write DKG public shares to file {}: {}",
            dkg_public_shares_path.to_str().unwrap_or("Invalid path"),
            err
        ))
    })?;

    Ok(())
}

fn load_dkg_data(
    data_directory: Option<&str>,
    frost_coordinator: &mut FrostCoordinator,
    stacks_node: &mut NodeClient,
    stacks_wallet: &StacksWallet,
    address: &StacksAddress,
) -> Result<XOnlyPublicKey> {
    debug!("Retrieving bitcoin wallet public key from sBTC contract...");
    if let Some(xonly_pubkey) = stacks_node.bitcoin_wallet_public_key(address)? {
        if let Some(data_directory) = data_directory {
            frost_coordinator.set_dkg_public_shares(read_dkg_public_shares(data_directory)?);
        }
        // We have to set the frost_coordinator aggregate key
        frost_coordinator.set_aggregate_public_key(
            Point::lift_x(&Element::from(xonly_pubkey.serialize()))
                .map_err(|e| Error::PointError(format!("{:?}", e)))?,
        );
        Ok(xonly_pubkey)
    } else {
        // If we don't get one stored in the contract...run the DKG round and get the resulting public key and use that
        let point = frost_coordinator.run_distributed_key_generation()?;

        if let Some(data_directory) = data_directory {
            write_dkg_public_shares(data_directory, frost_coordinator.get_dkg_public_shares())?;
        }

        let xonly_pubkey = XOnlyPublicKey::from_slice(&point.x().to_bytes())
            .map_err(|e| Error::InvalidPublicKey(e.to_string()))?;

        // Set the bitcoin address using the sbtc contract
        let nonce = stacks_node.next_nonce(address)?;
        let tx =
            stacks_wallet.build_set_bitcoin_wallet_public_key_transaction(&xonly_pubkey, nonce)?;
        stacks_node.broadcast_transaction(&tx)?;
        Ok(xonly_pubkey)
    }
}

impl TryFrom<&Config> for StacksCoordinator {
    type Error = Error;
    fn try_from(config: &Config) -> Result<Self> {
        info!("Initializing stacks coordinator...");
        let mut local_stacks_node = NodeClient::new(
            config.stacks_node_rpc_url.clone(),
            config.contract_name.clone(),
            config.contract_address,
        );

        let stacks_wallet = StacksWallet::new(
            config.contract_name.clone(),
            config.contract_address,
            config.stacks_private_key,
            config.stacks_address,
            config.stacks_version,
            config.transaction_fee,
        );

        let mut frost_coordinator =
            create_frost_coordinator(config, &mut local_stacks_node, &stacks_wallet)?;

        // Load the public key from either the frost_coordinator or the sBTC contract
        let xonly_pubkey = load_dkg_data(
            config.data_directory.as_deref(),
            &mut frost_coordinator,
            &mut local_stacks_node,
            &stacks_wallet,
            &config.stacks_address,
        )?;
        let bitcoin_wallet = BitcoinWallet::new(xonly_pubkey, config.bitcoin_network);

        // Load the bitcoin wallet
        let local_bitcoin_node = LocalhostBitcoinNode::new(config.bitcoin_node_rpc_url.clone());
        local_bitcoin_node.load_wallet(bitcoin_wallet.address())?;

        // If a user has not specified a start block height, begin from the current burn block height by default
        let start_block_height = config.start_block_height;
        let current_block_height = local_stacks_node.burn_block_height()?;
        let local_peg_queue = if let Some(path) = &config.data_directory {
            let db_path = PathBuf::from(path).join("peg_queue.sqlite");
            SqlitePegQueue::new(db_path, start_block_height, current_block_height)
        } else {
            SqlitePegQueue::in_memory(start_block_height, current_block_height)
        }?;

        Ok(Self {
            local_peg_queue,
            local_stacks_node,
            local_bitcoin_node,
            frost_coordinator,
            local_fee_wallet: WrapPegWallet {
                bitcoin_wallet,
                stacks_wallet,
            },
        })
    }
}

impl Coordinator for StacksCoordinator {
    type PegQueue = SqlitePegQueue;
    type FeeWallet = WrapPegWallet;
    type StacksNode = NodeClient;
    type BitcoinNode = LocalhostBitcoinNode;

    fn peg_queue(&self) -> &Self::PegQueue {
        &self.local_peg_queue
    }

    fn fee_wallet_mut(&mut self) -> &mut Self::FeeWallet {
        &mut self.local_fee_wallet
    }

    fn fee_wallet(&self) -> &Self::FeeWallet {
        &self.local_fee_wallet
    }

    fn frost_coordinator(&self) -> &FrostCoordinator {
        &self.frost_coordinator
    }

    fn frost_coordinator_mut(&mut self) -> &mut FrostCoordinator {
        &mut self.frost_coordinator
    }

    fn stacks_node(&self) -> &Self::StacksNode {
        &self.local_stacks_node
    }

    fn stacks_node_mut(&mut self) -> &mut Self::StacksNode {
        &mut self.local_stacks_node
    }

    fn bitcoin_node(&self) -> &Self::BitcoinNode {
        &self.local_bitcoin_node
    }
}

#[cfg(test)]
mod tests {
    use crate::config::{Config, RawConfig};
    use crate::coordinator::{CoordinatorHelpers, StacksCoordinator};
    use crate::stacks_node::PegOutRequestOp;
    use bitcoin::consensus::Encodable;
    use blockstack_lib::burnchains::Txid;
    use blockstack_lib::chainstate::stacks::address::{PoxAddress, PoxAddressType20};
    use blockstack_lib::types::chainstate::BurnchainHeaderHash;

    #[ignore]
    #[test]
    fn btc_fulfill_peg_out() {
        let raw_config = RawConfig {
            signer_config_path: Some("conf/signer.toml".to_string()),
            transaction_fee: 10,
            ..Default::default()
        };
        let config = Config::try_from(raw_config).unwrap();
        // todo: make StacksCoordinator with mock FrostCoordinator to locally generate PublicKey and Signature for unit test
        let mut sc = StacksCoordinator::try_from(&config).unwrap();
        let recipient = PoxAddress::Addr20(false, PoxAddressType20::P2WPKH, [0; 20]);
        let peg_wallet_address = PoxAddress::Addr20(false, PoxAddressType20::P2WPKH, [0; 20]);
        let op = PegOutRequestOp {
            amount: 0,
            recipient: recipient,
            signature: blockstack_lib::util::secp256k1::MessageSignature([0; 65]),
            peg_wallet_address: peg_wallet_address,
            fulfillment_fee: 0,
            memo: vec![],
            txid: Txid([0; 32]),
            vtxindex: 0,
            block_height: 0,
            burn_header_hash: BurnchainHeaderHash([0; 32]),
        };
        let btc_tx_result = sc.fulfill_peg_out(&op);
        assert!(btc_tx_result.is_ok());
        let btc_tx = btc_tx_result.unwrap();
        let mut btc_tx_encoded: Vec<u8> = vec![];
        btc_tx.consensus_encode(&mut btc_tx_encoded).unwrap();
        let verify_result = bitcoin::bitcoinconsensus::verify(&[], 100, &btc_tx_encoded, 0);
        assert!(verify_result.is_ok())
    }
}
