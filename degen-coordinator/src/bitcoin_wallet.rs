use bitcoin::{Address, blockdata::{opcodes::all, script::Builder}, hashes::hex::FromHex, KeyPair, Network, OutPoint, Script, secp256k1::{All, Secp256k1}, Transaction, TxIn, util::taproot, XOnlyPublicKey};
use serde_json::Number;
use tracing::{debug, warn};

use crate::bitcoin_node::{BitcoinNode, BitcoinTransaction, LocalhostBitcoinNode, UTXO};
use crate::coordinator::PublicKey;
use crate::peg_wallet::{BitcoinWallet as BitcoinWalletTrait, Error as PegWalletError};
use crate::stacks_node::PegOutRequestOp;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    #[error("Unable to fulfill peg-out request op due to insufficient funds.")]
    InsufficientFunds,
    #[error("Invalid unspent transaction id: {0}")]
    InvalidTransactionID(String),
    #[error("Missing peg-out fulfillment utxo.")]
    MissingFulfillmentUTXO,
    #[error("Fulfillment UTXO amount does not equal the fulfillment fee.")]
    MismatchedFulfillmentFee,
}

pub struct BitcoinWallet {
    address: Address,
    public_key: PublicKey,
}

impl BitcoinWallet {
    pub fn new(public_key: PublicKey, network: Network) -> Self {
        let secp = Secp256k1::verification_only();
        let address = bitcoin::Address::p2tr(&secp, public_key, None, network);
        Self {
            address,
            public_key,
        }
    }
}

/// Minimum dust required
const DUST_UTXO_LIMIT: u64 = 5500;

impl BitcoinWalletTrait for BitcoinWallet {
    type Error = Error;
    fn fulfill_peg_out(&self, op: &PegOutRequestOp, txouts: Vec<UTXO>) -> Result<Transaction, PegWalletError> {
        // todo!()
        Ok(Transaction {
            version: 2,
            lock_time: bitcoin::PackedLockTime(0),
            input: vec![],
            output: vec![],
        }
        )
    }


    fn address(&self) -> &Address {
        &self.address
    }

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
    ) -> Result<Transaction, PegWalletError> {
        // Create an empty transaction
        let mut tx = Transaction {
            version: 2,
            lock_time: bitcoin::PackedLockTime(0),
            input: vec![],
            output: vec![],
        };
        // Consume UTXOs until we have enough to cover the total spend
        // tx fee and spend to script amount
        let mut total_consumed = 0;
        let mut utxos = vec![];
        let mut fulfillment_utxo = None;
        for utxo in available_utxos.into_iter() {
            // TODO: check each output,
            if total_consumed < amount {
                if utxo.amount >= amount {
                    total_consumed = utxo.amount;
                    fulfillment_utxo = Some(utxo.clone());
                    utxos = vec![utxo.clone()];
                } else {
                    total_consumed += utxo.amount;
                    utxos.push(utxo);
                }
                if total_consumed >= amount {
                    // We have consumed enough to cover the total spend
                    // i.e. have found the fulfillment utxo and covered the peg out amount
                    break;
                }
            }
        }
        // Sanity check all the things!
        // If we did not find the fulfillment utxo, then we need to sign multiple inputs
        let fulfillment_utxo = fulfillment_utxo.ok_or_else(|| {
            warn!("No transaction had alone the amount required: {}.", amount);
        }).unwrap();
        // Check that we have sufficient funds and didn't just run out of available utxos.
        if total_consumed < amount {
            warn!(
                "Not enough sats in utxo {}. Less than intended spend: {}",
                total_consumed, amount
            );
            return Err(PegWalletError::from(Error::InsufficientFunds));
        }
        // Get the transaction change amount
        let change_amount = total_consumed - amount;
        debug!(
            "change_amount: {:?}, total_consumed: {:?}, amount: {:?}",
            change_amount, total_consumed, amount
        );
        // TODO: what is this?
        if change_amount >= DUST_UTXO_LIMIT {
            let secp = Secp256k1::verification_only();
            let script_pubkey = Script::new_v1_p2tr(&secp, self.public_key, None);
            let change_output = bitcoin::TxOut {
                value: change_amount,
                script_pubkey,
            };
            tx.output.push(change_output);
        } else {
            // Instead of leaving that change to the BTC miner, we could / should bump the sortition fee
            debug!("Not enough change to clear dust limit. Not adding change address.");
        }
        // If we have fulfillment_input, it would be the only utxo in utxos, and be the first
        for utxo in utxos {
            let input = utxo_to_input(utxo)?;
            tx.input.push(input);
        }
        Ok(tx)
    }
}

fn create_script_refund(
    user_public_key: &XOnlyPublicKey,
    unlock_block: usize,
) -> bitcoin::Script {
    Builder::new()
        .push_int(unlock_block as i64)
        .push_opcode(all::OP_CLTV)
        .push_opcode(all::OP_DROP)
        .push_x_only_key(user_public_key)
        .push_opcode(all::OP_CHECKSIG)
        .into_script()
}

pub fn create_script_unspendable() -> bitcoin::Script {
    Builder::new().push_opcode(all::OP_RETURN).into_script()
}


fn create_tree(
    secp: &Secp256k1<All>,
    key_pair_internal: &KeyPair,
    script_1: &Script,
    script_2: &Script,
) -> (taproot::TaprootSpendInfo, bitcoin::Address) {
    let builder = taproot::TaprootBuilder::with_huffman_tree(vec![
        (1, script_1.clone()),
        (1, script_2.clone()),
    ]).unwrap(); // TODO: degens - or use unwrap check it

    let (internal_public_ley, _) = key_pair_internal.x_only_public_key();
    let tap_info = builder.finalize(secp, internal_public_ley).unwrap();
    let address = Address::p2tr(
        secp,
        tap_info.internal_key(),
        tap_info.merkle_root(),
        Network::Testnet,
    );

    (tap_info, address)
}

fn get_current_block_height(client: &LocalhostBitcoinNode) -> u64 {
    client.get_blockcount().unwrap()
}

// fn get_prev_txs(
//     client: &LocalhostBitcoinNode,
//     address: &Address,
// ) -> (
//     Vec<bitcoin::TxIn>,
//     Vec<bitcoin::Transaction>
// ) {
//     let vec_tx_in: Vec<TxIn> = client.list_unspent(address)
//         .unwrap()
// }


// Helper function to convert a utxo to an unsigned input
fn utxo_to_input(utxo: UTXO) -> Result<TxIn, Error> {
    let input = TxIn {
        previous_output: OutPoint {
            txid: bitcoin::Txid::from_hex(&utxo.txid)
                .map_err(|_| Error::InvalidTransactionID(utxo.txid))?,
            vout: utxo.vout,
        },
        script_sig: Default::default(),
        sequence: bitcoin::Sequence(0xFFFFFFFD), // allow RBF
        witness: Default::default(),
    };
    Ok(input)
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use hex::encode;
    use rand::Rng;

    use crate::bitcoin_node::UTXO;
    use crate::coordinator::PublicKey;
    use crate::peg_wallet::{BitcoinWallet as BitcoinWalletTrait, Error as PegWalletError};
    use crate::util::test::{build_peg_out_request_op, PRIVATE_KEY_HEX};

    use super::{BitcoinWallet, Error};

    /// Helper function to build a valid bitcoin wallet
    fn bitcoin_wallet() -> BitcoinWallet {
        let public_key =
            PublicKey::from_str("cc8a4bc64d897bddc5fbc2f670f7a8ba0b386779106cf1223c6fc5d7cd6fc115")
                .expect("Failed to construct a valid public key for the bitcoin wallet");
        BitcoinWallet::new(public_key, bitcoin::Network::Testnet)
    }

    /// Helper function for building a random txid (32 byte hex string)
    fn generate_txid() -> String {
        let data: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();
        encode(data)
    }

    /// Helper function for building a utxo with the given txid, vout, and amount
    fn build_utxo(txid: String, vout: u32, amount: u64) -> UTXO {
        UTXO {
            txid,
            vout,
            amount,
            ..Default::default()
        }
    }

    /// Helper function for building a vector of nmb utxos with amounts increasing by 10000
    fn build_utxos(nmb: u32) -> Vec<UTXO> {
        (1..=nmb)
            .map(|i| build_utxo(generate_txid(), i, i as u64 * 10000))
            .collect()
    }

    // #[test]
    // fn fulfill_peg_out_insufficient_funds() {
    //     let wallet = bitcoin_wallet();
    //     let amount = 200000;
    //
    //     // (1+2+3+4+5)*10000 = 1500000 < 200000. Insufficient funds.
    //     let mut txouts = build_utxos(5);
    //
    //     let op = build_peg_out_request_op(PRIVATE_KEY_HEX, amount, 1, 1);
    //     // Build a fulfillment utxo that matches the generated op
    //     let fulfillment_utxo = build_utxo(op.txid.to_string(), 2, 1);
    //     txouts.push(fulfillment_utxo);
    //
    //     let result = wallet.fulfill_peg_out(&op, txouts);
    //     assert!(result.is_err());
    //     assert_eq!(
    //         result.err().unwrap(),
    //         PegWalletError::BitcoinWalletError(Error::InsufficientFunds)
    //     );
    // }
    //
    // #[test]
    // fn fulfill_peg_out_change() {
    //     let wallet = bitcoin_wallet();
    //     let amount = 200000;
    //
    //     // (1+2+3+4+5)*10000 = 210000 > 200000. We have change of 10000
    //     let mut txouts = build_utxos(6); // (1+2+3+4+5+6)*10000 = 210000
    //
    //     let op = build_peg_out_request_op(PRIVATE_KEY_HEX, amount, 1, 1);
    //     // Build a fulfillment utxo that matches the generated op
    //     let fulfillment_utxo = build_utxo(op.txid.to_string(), 2, 1);
    //     txouts.push(fulfillment_utxo);
    //
    //     let btc_tx = wallet.fulfill_peg_out(&op, txouts).unwrap();
    //     assert_eq!(btc_tx.input.len(), 7);
    //     assert_eq!(btc_tx.output.len(), 1); // We have change!
    //     assert_eq!(btc_tx.output[0].value, 10000);
    // }
    //
    #[test]
    fn fund_script() {
        let wallet = bitcoin_wallet();
        let amount = 5000;

        println!("address: {}", &wallet.address);
        let mut txouts = build_utxos(3);
        println!("txouts length: {:?}", txouts.len());
        let btc_tx = wallet.create_tx_fund_script(amount, &wallet.address, txouts);
        println!("btc tx: {:?}", btc_tx);
    }


    // #[test]
    // fn fulfill_peg_out_no_change() {
    //     let wallet = bitcoin_wallet();
    //     let amount = 9999;
    //
    //     // 1*10000 = 10000 > 9999. We only have change of 1...not enough to cover dust
    //     let mut txouts = build_utxos(1); // 1*10000 = 10000
    //
    //     let op = build_peg_out_request_op(PRIVATE_KEY_HEX, amount, 1, 1);
    //     // Build a fulfillment utxo that matches the generated op
    //     let fulfillment_utxo = build_utxo(op.txid.to_string(), 2, 1);
    //     txouts.push(fulfillment_utxo);
    //
    //     let btc_tx = wallet.fulfill_peg_out(&op, txouts).unwrap();
    //     assert_eq!(btc_tx.input.len(), 2);
    //     assert_eq!(btc_tx.output.len(), 0); // No change!
    // }
    //
    // #[test]
    // fn fulfill_peg_out_missing_fulfillment_utxo() {
    //     let wallet = bitcoin_wallet();
    //     let amount = 9999;
    //
    //     let mut txouts = vec![];
    //
    //     let op = build_peg_out_request_op(PRIVATE_KEY_HEX, amount, 1, 1);
    //     // Build a fulfillment utxo that matches the generated op, but with an invalid vout (i.e. incorrect vout)
    //     let fulfillment_utxo_invalid_vout = build_utxo(op.txid.to_string(), 1, 1);
    //     // Build a fulfillment utxo that does not match the generated op (i.e. mismatched txid)
    //     let fulfillment_utxo_invalid_txid = build_utxo(generate_txid(), 2, 1);
    //     txouts.push(fulfillment_utxo_invalid_vout);
    //     txouts.push(fulfillment_utxo_invalid_txid);
    //
    //     let result = wallet.fulfill_peg_out(&op, txouts);
    //
    //     assert!(result.is_err());
    //     assert_eq!(
    //         result.err().unwrap(),
    //         PegWalletError::BitcoinWalletError(Error::MissingFulfillmentUTXO)
    //     );
    // }
    //
    // #[test]
    // fn fulfill_peg_out_mismatched_fulfillment_utxo() {
    //     let wallet = bitcoin_wallet();
    //     let amount = 9999;
    //
    //     let mut txouts = vec![];
    //
    //     let op = build_peg_out_request_op(PRIVATE_KEY_HEX, amount, 1, 10);
    //     // Build a fulfillment utxo that matches the generated op, but has an invalid amount (does not cover the fulfillment fee)
    //     let fulfillment_utxo_invalid_amount = build_utxo(op.txid.to_string(), 2, 1);
    //     txouts.push(fulfillment_utxo_invalid_amount);
    //
    //     let result = wallet.fulfill_peg_out(&op, txouts);
    //
    //     assert!(result.is_err());
    //     assert_eq!(
    //         result.err().unwrap(),
    //         PegWalletError::BitcoinWalletError(Error::MismatchedFulfillmentFee)
    //     );
    // }
}
