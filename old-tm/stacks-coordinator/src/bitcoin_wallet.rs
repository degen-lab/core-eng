use std::iter::repeat;

use crate::bitcoin_node::UTXO;
use crate::peg_wallet::{BitcoinWallet as BitcoinWalletTrait, Error as PegWalletError};
use crate::stacks_node::PegOutRequestOp;
use bitcoin::blockdata::opcodes;
use bitcoin::TxOut;
use bitcoin::{
    blockdata::script, hashes::hex::FromHex, schnorr::TweakedPublicKey, Address, Network, OutPoint,
    Script, Transaction, TxIn, XOnlyPublicKey,
};
use tracing::{debug, warn};

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    #[error("Unable to fulfill peg-out request op due to insufficient funds.")]
    InsufficientFunds,
    #[error("Invalid unspent transaction id: {0}")]
    InvalidTransactionID(String),
    #[error("Invalid unspent transaction script pub key: {0}")]
    InvalidScriptPubKey(String),
    #[error("Missing peg-out fulfillment utxo.")]
    MissingFulfillmentUTXO,
    #[error("Fulfillment UTXO amount does not equal the fulfillment fee.")]
    MismatchedFulfillmentFee,
}

pub struct BitcoinWallet {
    address: Address,
    public_key: XOnlyPublicKey,
}

impl BitcoinWallet {
    pub fn new(public_key: XOnlyPublicKey, network: Network) -> Self {
        let tweaked_public_key = TweakedPublicKey::dangerous_assume_tweaked(public_key);
        let address = Address::p2tr_tweaked(tweaked_public_key, network);
        Self {
            address,
            public_key,
        }
    }
}

impl BitcoinWalletTrait for BitcoinWallet {
    type Error = Error;
    fn fulfill_peg_out(
        &self,
        op: &PegOutRequestOp,
        available_utxos: Vec<UTXO>,
    ) -> Result<(Transaction, Vec<TxOut>), PegWalletError> {
        // Create an empty transaction
        let mut tx = Transaction {
            version: 2,
            lock_time: bitcoin::PackedLockTime(0),
            input: vec![],
            output: vec![],
        };
        // Consume UTXOs until we have enough to cover the total spend (fulfillment fee and peg out amount)
        let mut total_consumed = 0;
        let mut prevouts = vec![];
        let mut found_fulfillment_utxo = false;
        for utxo in available_utxos.into_iter() {
            if utxo.txid == op.txid.to_string() && utxo.vout == 2 {
                // This is the fulfillment utxo.
                if utxo.amount != op.fulfillment_fee {
                    // Something is wrong. The fulfillment fee should match the fulfillment utxo amount.
                    // Malformed Peg Request Op
                    return Err(PegWalletError::from(Error::MismatchedFulfillmentFee));
                }
                tx.input.push(utxo_to_input(&utxo)?);
                prevouts.push(utxo_to_output(&utxo)?);
                found_fulfillment_utxo = true;
            } else if total_consumed < op.amount {
                total_consumed += utxo.amount;
                tx.input.push(utxo_to_input(&utxo)?);
                prevouts.push(utxo_to_output(&utxo)?);
            } else if found_fulfillment_utxo {
                // We have consumed enough to cover the total spend
                // i.e. have found the fulfillment utxo and covered the peg out amount
                break;
            }
        }
        // Sanity check all the things!
        // If we did not find the fulfillment utxo, something went wrong
        if !found_fulfillment_utxo {
            warn!("Failed to find fulfillment utxo.");
            return Err(PegWalletError::from(Error::MissingFulfillmentUTXO));
        }
        // Check that we have sufficient funds and didn't just run out of available utxos.
        if total_consumed < op.amount {
            warn!(
                "Consumed total {} is less than intended spend: {}",
                total_consumed, op.amount
            );
            return Err(PegWalletError::from(Error::InsufficientFunds));
        }

        // Get the transaction change amount
        let change_amount = total_consumed - op.amount;
        debug!(
            "change_amount: {:?}, total_consumed: {:?}, op.amount: {:?}",
            change_amount, total_consumed, op.amount
        );
        // Do not want to use Script::new_v1_p2tr because it will tweak our key when we don't want it to
        let public_key_tweaked = TweakedPublicKey::dangerous_assume_tweaked(self.public_key);
        let script_pubkey = Script::new_v1_p2tr_tweaked(public_key_tweaked);

        tx.output.push(withdrawal_data_output());

        let withdrawal_output = bitcoin::TxOut {
            value: op.amount,
            script_pubkey: script_pubkey.clone(),
        };
        tx.output.push(withdrawal_output);

        if change_amount >= script_pubkey.dust_value().to_sat() {
            let change_output = bitcoin::TxOut {
                value: change_amount,
                script_pubkey,
            };
            tx.output.push(change_output);
        } else {
            // Instead of leaving that change to the BTC miner, we could / should bump the sortition fee
            debug!("Not enough change to clear dust limit. Not adding change address.");
        }

        Ok((tx, prevouts))
    }

    fn address(&self) -> &Address {
        &self.address
    }

    fn x_only_pub_key(&self) -> &XOnlyPublicKey {
        &self.public_key
    }
}

fn withdrawal_data_output() -> TxOut {
    let data: Vec<u8> = [b'T', b'2', b'!']
        .into_iter()
        .chain(repeat(b'.'))
        .take(35)
        .collect();

    let script_pubkey = script::Builder::new()
        .push_opcode(opcodes::all::OP_RETURN)
        .push_slice(&data)
        .into_script();

    bitcoin::TxOut {
        value: 0,
        script_pubkey,
    }
}

// Helper function to convert a utxo to an unsigned input
fn utxo_to_input(utxo: &UTXO) -> Result<TxIn, Error> {
    let input = TxIn {
        previous_output: OutPoint {
            txid: bitcoin::Txid::from_hex(&utxo.txid)
                .map_err(|_| Error::InvalidTransactionID(utxo.txid.clone()))?,
            vout: utxo.vout,
        },
        script_sig: Default::default(),
        sequence: bitcoin::Sequence(0xFFFFFFFD), // allow RBF
        witness: Default::default(),
    };
    Ok(input)
}

// Helper function to convert a utxo to an output
fn utxo_to_output(utxo: &UTXO) -> Result<TxOut, Error> {
    let output = TxOut {
        value: utxo.amount,
        script_pubkey: Script::from(
            hex::decode(&utxo.scriptPubKey)
                .map_err(|_| Error::InvalidScriptPubKey(utxo.scriptPubKey.clone()))?,
        ),
    };
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::{BitcoinWallet, Error};
    use crate::bitcoin_node::UTXO;
    use crate::peg_wallet::{BitcoinWallet as BitcoinWalletTrait, Error as PegWalletError};
    use crate::util::test::{build_peg_out_request_op, PRIVATE_KEY_HEX};
    use bitcoin::XOnlyPublicKey;
    use hex::encode;
    use rand::Rng;
    use std::str::FromStr;

    /// Helper function to build a valid bitcoin wallet
    fn bitcoin_wallet() -> BitcoinWallet {
        let public_key = XOnlyPublicKey::from_str(
            "cc8a4bc64d897bddc5fbc2f670f7a8ba0b386779106cf1223c6fc5d7cd6fc115",
        )
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

    #[test]
    fn fulfill_peg_out_insufficient_funds() {
        let wallet = bitcoin_wallet();
        let amount = 200000;

        // (1+2+3+4+5)*10000 = 1500000 < 200000. Insufficient funds.
        let mut txouts = build_utxos(5);

        let op = build_peg_out_request_op(PRIVATE_KEY_HEX, amount, 1, 1);
        // Build a fulfillment utxo that matches the generated op
        let fulfillment_utxo = build_utxo(op.txid.to_string(), 2, 1);
        txouts.push(fulfillment_utxo);

        let result = wallet.fulfill_peg_out(&op, txouts);
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            PegWalletError::BitcoinWalletError(Error::InsufficientFunds)
        );
    }

    #[test]
    fn fulfill_peg_out_change() {
        let wallet = bitcoin_wallet();
        let amount = 200000;

        // (1+2+3+4+5)*10000 = 210000 > 200000. We have change of 10000
        let mut txouts = build_utxos(6); // (1+2+3+4+5+6)*10000 = 210000

        let op = build_peg_out_request_op(PRIVATE_KEY_HEX, amount, 1, 1);
        // Build a fulfillment utxo that matches the generated op
        let fulfillment_utxo = build_utxo(op.txid.to_string(), 2, 1);
        txouts.push(fulfillment_utxo);

        let (btc_tx, _) = wallet.fulfill_peg_out(&op, txouts).unwrap();
        assert_eq!(btc_tx.input.len(), 7);
        assert_eq!(btc_tx.output.len(), 3); // We have change!
        assert_eq!(btc_tx.output[0].value, 0);
        assert_eq!(btc_tx.output[1].value, amount);
        assert_eq!(btc_tx.output[2].value, 10000);
    }

    #[test]
    fn fulfill_peg_out_no_change() {
        let wallet = bitcoin_wallet();
        let amount = 9999;

        // 1*10000 = 10000 > 9999. We only have change of 1...not enough to cover dust
        let mut txouts = build_utxos(1); // 1*10000 = 10000

        let op = build_peg_out_request_op(PRIVATE_KEY_HEX, amount, 1, 1);
        // Build a fulfillment utxo that matches the generated op
        let fulfillment_utxo = build_utxo(op.txid.to_string(), 2, 1);
        txouts.push(fulfillment_utxo);

        let (btc_tx, _) = wallet.fulfill_peg_out(&op, txouts).unwrap();
        assert_eq!(btc_tx.input.len(), 2);
        assert_eq!(btc_tx.output.len(), 2); // No change!
    }

    #[test]
    fn fulfill_peg_out_missing_fulfillment_utxo() {
        let wallet = bitcoin_wallet();
        let amount = 9999;

        let mut txouts = vec![];

        let op = build_peg_out_request_op(PRIVATE_KEY_HEX, amount, 1, 1);
        // Build a fulfillment utxo that matches the generated op, but with an invalid vout (i.e. incorrect vout)
        let fulfillment_utxo_invalid_vout = build_utxo(op.txid.to_string(), 1, 1);
        // Build a fulfillment utxo that does not match the generated op (i.e. mismatched txid)
        let fulfillment_utxo_invalid_txid = build_utxo(generate_txid(), 2, 1);
        txouts.push(fulfillment_utxo_invalid_vout);
        txouts.push(fulfillment_utxo_invalid_txid);

        let result = wallet.fulfill_peg_out(&op, txouts);

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            PegWalletError::BitcoinWalletError(Error::MissingFulfillmentUTXO)
        );
    }

    #[test]
    fn fulfill_peg_out_mismatched_fulfillment_utxo() {
        let wallet = bitcoin_wallet();
        let amount = 9999;

        let mut txouts = vec![];

        let op = build_peg_out_request_op(PRIVATE_KEY_HEX, amount, 1, 10);
        // Build a fulfillment utxo that matches the generated op, but has an invalid amount (does not cover the fulfillment fee)
        let fulfillment_utxo_invalid_amount = build_utxo(op.txid.to_string(), 2, 1);
        txouts.push(fulfillment_utxo_invalid_amount);

        let result = wallet.fulfill_peg_out(&op, txouts);

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            PegWalletError::BitcoinWalletError(Error::MismatchedFulfillmentFee)
        );
    }
}
