use crate::bitcoin_node::{self, UTXO};
use crate::bitcoin_wallet::{BitcoinWallet as BitcoinWalletStruct, Error as BitcoinWalletError};
use crate::stacks_node::{self, PegOutRequestOp};
use crate::stacks_wallet::{Error as StacksWalletError, StacksWallet as StacksWalletStruct};
use bitcoin::Address as BitcoinAddress;
use blockstack_lib::{chainstate::stacks::StacksTransaction, types::chainstate::StacksAddress};
use std::fmt::Debug;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    #[error("Stacks Wallet Error: {0}")]
    StacksWalletError(#[from] StacksWalletError),
    #[error("Bitcoin Wallet Error: {0}")]
    BitcoinWalletError(#[from] BitcoinWalletError),
}

pub trait StacksWallet {
    /// Builds a verified signed transaction for a given peg-in operation
    fn build_mint_transaction(
        &self,
        op: &stacks_node::PegInOp,
        nonce: u64,
    ) -> Result<StacksTransaction, Error>;
    /// Builds a verified signed transaction for a given peg-out request operation
    fn build_burn_transaction(
        &self,
        op: &stacks_node::PegOutRequestOp,
        nonce: u64,
    ) -> Result<StacksTransaction, Error>;
    /// Builds a verified signed transaction for setting the sBTC wallet address
    fn build_set_btc_address_transaction(
        &self,
        address: &BitcoinAddress,
        nonce: u64,
    ) -> Result<StacksTransaction, Error>;
    /// Returns the sBTC address for the wallet
    fn address(&self) -> &StacksAddress;
}

pub trait BitcoinWallet {
    type Error: Debug;
    // Builds a fulfilled unsigned transaction using the provided utxos to cover the spend amount
    fn fulfill_peg_out(
        &self,
        op: &PegOutRequestOp,
        txouts: Vec<UTXO>,
    ) -> Result<bitcoin_node::BitcoinTransaction, Error>;
    /// Returns the BTC address for the wallet
    fn address(&self) -> &BitcoinAddress;
}

pub trait PegWallet {
    type StacksWallet: StacksWallet;
    type BitcoinWallet: BitcoinWallet;
    fn stacks(&self) -> &Self::StacksWallet;
    fn bitcoin(&self) -> &Self::BitcoinWallet;
}

pub type PegWalletAddress = bitcoin::Address;

pub struct WrapPegWallet {
    pub(crate) bitcoin_wallet: BitcoinWalletStruct,
    pub(crate) stacks_wallet: StacksWalletStruct,
}

impl PegWallet for WrapPegWallet {
    type StacksWallet = StacksWalletStruct;
    type BitcoinWallet = BitcoinWalletStruct;

    fn stacks(&self) -> &Self::StacksWallet {
        &self.stacks_wallet
    }

    fn bitcoin(&self) -> &Self::BitcoinWallet {
        &self.bitcoin_wallet
    }
}
