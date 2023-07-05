pub mod client;

use blockstack_lib::{
    chainstate::{burn::operations as burn_ops, stacks::StacksTransaction},
    codec::Error as CodecError,
    types::chainstate::StacksAddress,
};

/// Kinds of common errors used by stacks coordinator
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Invalid JSON entry: {0}")]
    InvalidJsonEntry(String),
    #[error("Failed to find burn block height: {0}")]
    UnknownBlockHeight(u64),
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
    #[error("Broadcast failure: {0}")]
    BroadcastFailure(String),
}

#[cfg_attr(test, mockall::automock)]
pub trait StacksNode {
    fn get_peg_in_ops(&self, block_height: u64) -> Result<Vec<PegInOp>, Error>;
    fn get_peg_out_request_ops(&self, block_height: u64) -> Result<Vec<PegOutRequestOp>, Error>;
    fn burn_block_height(&self) -> Result<u64, Error>;
    fn next_nonce(&self, addr: &StacksAddress) -> Result<u64, Error>;
    fn broadcast_transaction(&self, tx: &StacksTransaction) -> Result<(), Error>;
}

pub type PegInOp = burn_ops::PegInOp;
pub type PegOutRequestOp = burn_ops::PegOutRequestOp;
