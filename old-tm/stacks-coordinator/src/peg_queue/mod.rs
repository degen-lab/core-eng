use blockstack_lib::burnchains::Txid;
use blockstack_lib::types::chainstate::BurnchainHeaderHash;

use crate::stacks_node;
use crate::stacks_node::Error as StacksNodeError;
mod sqlite_peg_queue;

pub use sqlite_peg_queue::{Error as SqlitePegQueueError, SqlitePegQueue};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Sqlite Peg Queue Error: {0}")]
    SqlitePegQueueError(#[from] SqlitePegQueueError),
    #[error("Stacks Node Error: {0}")]
    StacksNodeError(#[from] StacksNodeError),
}

pub trait PegQueue {
    fn sbtc_op(&self) -> Result<Option<SbtcOp>, Error>;
    fn poll<N: stacks_node::StacksNode>(&self, stacks_node: &N) -> Result<(), Error>;

    fn acknowledge(&self, txid: &Txid, burn_header_hash: &BurnchainHeaderHash)
        -> Result<(), Error>;
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub enum SbtcOp {
    PegIn(stacks_node::PegInOp),
    PegOutRequest(stacks_node::PegOutRequestOp),
}

impl SbtcOp {
    pub fn as_peg_in(&self) -> Option<&stacks_node::PegInOp> {
        match self {
            Self::PegIn(op) => Some(op),
            _ => None,
        }
    }

    pub fn as_peg_out_request(&self) -> Option<&stacks_node::PegOutRequestOp> {
        match self {
            Self::PegOutRequest(op) => Some(op),
            _ => None,
        }
    }
}
