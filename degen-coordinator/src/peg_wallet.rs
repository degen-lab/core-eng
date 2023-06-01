use crate::bitcoin_node::{self, UTXO};
use crate::bitcoin_wallet::{BitcoinWallet as BitcoinWalletStruct, Error as BitcoinWalletError};
use crate::stacks_node::{self, PegOutRequestOp};
use crate::stacks_wallet::{Error as StacksWalletError, StacksWallet as StacksWalletStruct};
use bitcoin::{Address as BitcoinAddress, Script, Transaction};
use blockstack_lib::{chainstate::stacks::StacksTransaction, types::chainstate::StacksAddress};
use std::fmt::Debug;
use std::vec;
use bitcoin::secp256k1::Secp256k1;
use tracing::{debug, warn};

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
    // Builds a degenerate transaction
    // fn fulfill_degen(
    //     &self,
    //     txouts: Vec<UTXO>,
    // ) -> Result<bitcoin_node::BitcoinTransaction, Error>;
    // Builds a fulfilled unsigned transaction using the provided utxos to cover the spend amount
    fn fulfill_peg_out(
        &self,
        op: &PegOutRequestOp,
        txouts: Vec<UTXO>,
    ) -> Result<bitcoin_node::BitcoinTransaction, Error>;
    /// Returns the BTC address for the wallet
    fn address(&self) -> &BitcoinAddress;
    // fn fulfill_degen(
    //     &self,
    //     available_utxos: Vec<UTXO>,
    // ) -> Result<Transaction, PegWalletError> {
    //     // Create an empty transaction
    //     let mut tx = Transaction {
    //         version: 2,
    //         lock_time: bitcoin::PackedLockTime(0),
    //         input: vec![],
    //         output: vec![],
    //     };
    //     // Consume UTXOs until we have enough to cover the total spend (fulfillment fee and peg out amount)
    //     let mut total_consumed = 0;
    //     let mut utxos = vec![];
    //     let mut fulfillment_utxo = None;
    //     for utxo in available_utxos.into_iter() {
    //         if utxo.txid == op.txid.to_string() && utxo.vout == 2 {
    //             // This is the fulfillment utxo.
    //             if utxo.amount != op.fulfillment_fee {
    //                 // Something is wrong. The fulfillment fee should match the fulfillment utxo amount.
    //                 // Malformed Peg Request Op
    //                 return Err(PegWalletError::from(Error::MismatchedFulfillmentFee));
    //             }
    //             fulfillment_utxo = Some(utxo);
    //         } else if total_consumed < op.amount {
    //             total_consumed += utxo.amount;
    //             utxos.push(utxo);
    //         } else if fulfillment_utxo.is_some() {
    //             // We have consumed enough to cover the total spend
    //             // i.e. have found the fulfillment utxo and covered the peg out amount
    //             break;
    //         }
    //     }
    //     // Sanity check all the things!
    //     // If we did not find the fulfillment utxo, something went wrong
    //     let fulfillment_utxo = fulfillment_utxo.ok_or_else(|| {
    //         warn!("Failed to find fulfillment utxo.");
    //         Error::MissingFulfillmentUTXO
    //     })?;
    //     // Check that we have sufficient funds and didn't just run out of available utxos.
    //     if total_consumed < op.amount {
    //         warn!(
    //             "Consumed total {} is less than intended spend: {}",
    //             total_consumed, op.amount
    //         );
    //         return Err(PegWalletError::from(Error::InsufficientFunds));
    //     }
    //     // Get the transaction change amount
    //     let change_amount = total_consumed - op.amount;
    //     debug!(
    //         "change_amount: {:?}, total_consumed: {:?}, op.amount: {:?}",
    //         change_amount, total_consumed, op.amount
    //     );
    //     if change_amount >= DUST_UTXO_LIMIT {
    //         let secp = Secp256k1::verification_only();
    //         let script_pubkey = Script::new_v1_p2tr(&secp, self.public_key, None);
    //         let change_output = bitcoin::TxOut {
    //             value: change_amount,
    //             script_pubkey,
    //         };
    //         tx.output.push(change_output);
    //     } else {
    //         // Instead of leaving that change to the BTC miner, we could / should bump the sortition fee
    //         debug!("Not enough change to clear dust limit. Not adding change address.");
    //     }
    //     // Convert the utxos to inputs for the transaction, ensuring the fulfillment utxo is the first input
    //     let fulfillment_input = utxo_to_input(fulfillment_utxo)?;
    //     tx.input.push(fulfillment_input);
    //     for utxo in utxos {
    //         let input = utxo_to_input(utxo)?;
    //         tx.input.push(input);
    //     }
    //     Ok(tx)
    // }
    //
    //
    fn create_tx_fund_script(
        &self,
        amount: u64,
        address: &bitcoin::Address,
        available_utxos: Vec<UTXO>,
    ) -> Result<Transaction, Error>;
    // fn fulfill_degen(
    //     &self,
    //     available_utxos: Vec<UTXO>,
    // ) -> Result<Transaction, PegWalletError> {
    //     // Create an empty transaction
    //     let mut tx = Transaction {
    //         version: 2,
    //         lock_time: bitcoin::PackedLockTime(0),
    //         input: vec![],
    //         output: vec![],
    //     };
    //     // Consume UTXOs until we have enough to cover the total spend (fulfillment fee and peg out amount)
    //     let mut total_consumed = 0;
    //     let mut utxos = vec![];
    //     let mut fulfillment_utxo = None;
    //     for utxo in available_utxos.into_iter() {
    //         if utxo.txid == op.txid.to_string() && utxo.vout == 2 {
    //             // This is the fulfillment utxo.
    //             if utxo.amount != op.fulfillment_fee {
    //                 // Something is wrong. The fulfillment fee should match the fulfillment utxo amount.
    //                 // Malformed Peg Request Op
    //                 return Err(PegWalletError::from(Error::MismatchedFulfillmentFee));
    //             }
    //             fulfillment_utxo = Some(utxo);
    //         } else if total_consumed < op.amount {
    //             total_consumed += utxo.amount;
    //             utxos.push(utxo);
    //         } else if fulfillment_utxo.is_some() {
    //             // We have consumed enough to cover the total spend
    //             // i.e. have found the fulfillment utxo and covered the peg out amount
    //             break;
    //         }
    //     }
    //     // Sanity check all the things!
    //     // If we did not find the fulfillment utxo, something went wrong
    //     let fulfillment_utxo = fulfillment_utxo.ok_or_else(|| {
    //         warn!("Failed to find fulfillment utxo.");
    //         Error::MissingFulfillmentUTXO
    //     })?;
    //     // Check that we have sufficient funds and didn't just run out of available utxos.
    //     if total_consumed < op.amount {
    //         warn!(
    //             "Consumed total {} is less than intended spend: {}",
    //             total_consumed, op.amount
    //         );
    //         return Err(PegWalletError::from(Error::InsufficientFunds));
    //     }
    //     // Get the transaction change amount
    //     let change_amount = total_consumed - op.amount;
    //     debug!(
    //         "change_amount: {:?}, total_consumed: {:?}, op.amount: {:?}",
    //         change_amount, total_consumed, op.amount
    //     );
    //     if change_amount >= DUST_UTXO_LIMIT {
    //         let secp = Secp256k1::verification_only();
    //         let script_pubkey = Script::new_v1_p2tr(&secp, self.public_key, None);
    //         let change_output = bitcoin::TxOut {
    //             value: change_amount,
    //             script_pubkey,
    //         };
    //         tx.output.push(change_output);
    //     } else {
    //         // Instead of leaving that change to the BTC miner, we could / should bump the sortition fee
    //         debug!("Not enough change to clear dust limit. Not adding change address.");
    //     }
    //     // Convert the utxos to inputs for the transaction, ensuring the fulfillment utxo is the first input
    //     let fulfillment_input = utxo_to_input(fulfillment_utxo)?;
    //     tx.input.push(fulfillment_input);
    //     for utxo in utxos {
    //         let input = utxo_to_input(utxo)?;
    //         tx.input.push(input);
    //     }
    //     Ok(tx)
    // }
    //
    //

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
