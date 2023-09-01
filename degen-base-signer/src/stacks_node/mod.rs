pub mod client;

use bitcoin::XOnlyPublicKey;
use blockstack_lib::{
    chainstate::{burn::operations as burn_ops, stacks::StacksTransaction},
    codec::Error as CodecError,
    types::chainstate::StacksAddress,
    vm::{types::serialization::SerializationError, Value as ClarityValue},
};
use blockstack_lib::vm::types::{PrincipalData, SequenceData};
use blockstack_lib::vm::{ClarityName, Value};
use tracing::info;
use crate::config::{MinerStatus, PublicKeys, SignerKeyIds};
use wsts::ecdsa::PublicKey;

use self::client::BroadcastError;

/// Kinds of common errors used by stacks coordinator
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Invalid JSON entry: {0}")]
    InvalidJsonEntry(String),
    #[error("Failed to find burn block height: {0}")]
    UnknownBlockHeight(u64),
    #[error("Failed to find account: {0}")]
    UnknownAddress(String),
    #[error("{0}")]
    JsonError(#[from] serde_json::Error),
    #[error("{0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Failed to serialize transaction. {0}")]
    CodecError(#[from] CodecError),
    #[error("Failed to connect to stacks node.")]
    Timeout,
    #[error("Failed to load Stacks chain tip.")]
    BehindChainTip,
    #[error("Broadcast error: {0}")]
    BroadcastError(#[from] BroadcastError),
    #[error("Failed to call function {0}")]
    ReadOnlyFailure(String),
    #[error("Clarity Deserialization Error: {0}")]
    SerializationError(#[from] SerializationError),
    #[error("No coordinator found in sBTC contract.")]
    NoCoordinatorData,
    #[error("No signer data found for signer ID {0}")]
    NoSignerData(u128),
    #[error("Recieved a malformed clarity value from {0} contract call: {1}")]
    MalformedClarityValue(String, ClarityValue),
    #[error("Error occurred deserializing clarity value: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
    #[error("URL Parse Error: {0}")]
    UrlParseError(#[from] url::ParseError),
}

#[cfg_attr(test, mockall::automock)]
pub trait StacksNode {
    fn get_peg_in_ops(&self, block_height: u64) -> Result<Vec<PegInOp>, Error>;
    fn get_peg_out_request_ops(&self, block_height: u64) -> Result<Vec<PegOutRequestOp>, Error>;
    fn burn_block_height(&self) -> Result<u64, Error>;
    fn next_nonce(&mut self, addr: &StacksAddress) -> Result<u64, Error>;
    fn broadcast_transaction(&self, tx: &StacksTransaction) -> Result<(), Error>;
    fn keys_threshold(&self, sender: &StacksAddress) -> Result<u128, Error>;
    fn public_keys(&self, sender: &StacksAddress) -> Result<PublicKeys, Error>;
    fn signer_key_ids(&self, sender: &StacksAddress) -> Result<SignerKeyIds, Error>;
    fn coordinator_public_key(&self, sender: &StacksAddress) -> Result<Option<PublicKey>, Error>;
    fn bitcoin_wallet_public_key(&self, sender: &StacksAddress) -> Result<Option<XOnlyPublicKey>, Error>;
    fn get_status(&self, sender: &StacksAddress) -> Result<MinerStatus, Error>;
    fn get_warn_number_user(&self, sender: &StacksAddress, warned_address: PrincipalData) -> Result<u128, Error>;
    fn get_notifier(&self, sender: &StacksAddress) -> Result<PrincipalData, Error>;
    fn is_blacklisted(&self, sender: &StacksAddress, address: PrincipalData) -> Result<bool, Error>;
    fn is_block_claimed(&self, sender: &StacksAddress, block_height: u128) -> Result<bool, Error>;
    fn is_enough_voted_to_enter(&self, sender: &StacksAddress) -> Result<bool, Error>;
    fn is_auto_exchange(&self, sender: &StacksAddress) -> Result<bool, Error>;
    fn get_reward_info_for_block_height(&self, sender: &StacksAddress, block_height: u128) -> Result<(u128, PrincipalData), Error>;
    fn get_miners_list(&self, sender: &StacksAddress) -> Result<Vec<StacksAddress>, Error>;
}

pub type PegInOp = burn_ops::PegInOp;
pub type PegOutRequestOp = burn_ops::PegOutRequestOp;
