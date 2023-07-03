use std::str::FromStr;
use bitcoin::blockdata::opcodes::all;
use bitcoin::blockdata::script;
use bitcoin::{Address, KeyPair, Network, OutPoint, PackedLockTime, PrivateKey, PublicKey, SchnorrSig, SchnorrSighashType, Script, secp256k1, Transaction, TxOut, Witness, XOnlyPublicKey};
use bitcoin::psbt::Prevouts;
use bitcoin::psbt::serialize::Serialize;
use bitcoin::schnorr::TapTweak;
use bitcoin::secp256k1::{All, Secp256k1};
use bitcoin::util::{sighash, taproot};
use bitcoin::util::sighash::SighashCache;
use bitcoin::util::taproot::{ControlBlock, TaprootSpendInfo};
use rand_core::OsRng;
use tracing::debug;
use crate::bitcoin_node::{UTXO};
use crate::net::Message;

/// Minimum dust required
const DUST_UTXO_LIMIT: u64 = 5500;
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

// degen utils for function creation
fn create_script_refund(
    user_public_key: &XOnlyPublicKey,
    unlock_block: usize,
) -> bitcoin::Script {
    script::Builder::new()
        .push_int(unlock_block as i64)
        .push_opcode(all::OP_CLTV)
        .push_opcode(all::OP_DROP)
        .push_x_only_key(user_public_key)
        .push_opcode(all::OP_CHECKSIG)
        .into_script()
}

pub fn create_script_unspendable() -> bitcoin::Script {
    script::Builder::new().push_opcode(all::OP_RETURN).into_script()
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
    ]).unwrap();

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

fn verify_p2tr_commitment(
    secp: &Secp256k1<All>,
    script: &Script,
    key_pair_internal: &KeyPair,
    tap_info: &TaprootSpendInfo,
    actual_control: &ControlBlock,
) {
    let tweak_key_pair = key_pair_internal
        .tap_tweak(&secp, tap_info.merkle_root())
        .to_inner();
    let (tweak_key_pair_public_key, _) = tweak_key_pair.x_only_public_key();
    assert!(actual_control.verify_taproot_commitment(secp, tweak_key_pair_public_key, script));
}


// from
// amount
// spender
// utxos
// outputs
// consume utxos till the amount is met,
// if met directly by one utxo, consume only that
fn create_unsigned_tx(
    sender_address: &Address,
    receiver_address: &Address,
    amount: u64,
    fee: u64,
    available_utxos: &Vec<UTXO>,
) -> Result<Transaction, Error> {

    let mut tx = Transaction {
        version: 2,
        lock_time: PackedLockTime(0),
        input: vec![],
        output: vec![],
    };
    // Consume UTXOs until we have enough to cover the total spend
    // tx fee and spend to script amount
    let mut total_consumed = 0;
    let mut utxos = vec![];
    for utxo in available_utxos.into_iter() {
        // TODO: check each output,
        if total_consumed < amount {
            if utxo.amount >= amount {
                total_consumed = utxo.amount;
                utxos = vec![utxo];
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
    if total_consumed < amount {
        return Err(Error::InsufficientFunds);
    }

        // Get the transaction change amount
    let change_amount = total_consumed - amount - fee;
    debug!(
            "change_amount: {:?}, total_consumed: {:?}, amount: {:?}",
            change_amount, total_consumed, amount
        );
    let spend_output = bitcoin::TxOut {
        value: amount,
        script_pubkey: receiver_address.script_pubkey()
    };
    tx.output.push(spend_output);
    if (change_amount > DUST_UTXO_LIMIT) {
        let change_output = bitcoin::TxOut {
            value: change_amount,
            script_pubkey: spender_address.script_pubkey()
        };
        tx.output.push(change_output);
    }
    for utxo in utxos {
        let input = utxo_to_input(utxo)?;
        tx.input.push(input);
    }
    Ok(tx)
}

pub fn get_prevouts_utxos(available_utxos: &Vec<UTXO>) -> Vec<Prevouts<TxOut>> {
    let mut prevouts: Vec<Prevouts<TxOut>> = Vec::new();
    for (index, utxo) in available_utxos.into_iter() {
        let prevout = Prevouts::All(utxo);
        prevouts.push(prevout);
    }
    prevouts
}



// have general things that can be parsed directly here
// have specific things as arguments so that this function can remain outside the struct
/// 1 input case written
pub fn create_tx_signed_key_taproot_fund_script(
    public_key_sender: PublicKey,
    address_sender: &Address, // p2tr //bitcoin::Address::p2tr(&secp, public_key, None, network);
    address_receiver: &Address, // p2tr
    amount: u64,
    fee: u64,
    private_key_user: &str,
    block_height: u64,
    network: Network,
    available_utxos: Vec<UTXO>,
) -> Result<Transaction, Error>
{
    let secp = Secp256k1::new();
    let mut rng = OsRng;
    let blocks_refund = 10;

    // let sender_address = bitcoin::Address::p2tr(&secp, sender_public_key, None, network);

    // TODO: keypair from private key
    let private_key = Secp256k1PrivateKey::from_hex(private_key_user)
        .expect("Failed to construct a valid private key");
    let keypair = KeyPair::new(&secp, &mut rng);

    let script_refund = create_script_refund(
        XOnlyPublicKey::from_keypair(&keypair)[0],
        (block_height + blocks_refund) as usize
    );
    let script_unspendable = create_script_unspendable();

    let (tap_info, address) = create_tree(
        &secp,
        &keypair,
        &script_refund,
        &script_unspendable,
    );

    // then create tx with output  (script address) + change to user input
    let tx = create_unsigned_tx(spender_address, receiver_address, amount, fee, &available_utxos)?;
    // // script pubkey
    // let script_script_pubkey = receiver_address.script_pubkey();
    // let change_script_pubkey = sender_address.script_pubkey();


    // construct prevouts
    let prevouts = get_prevouts_utxos(&available_utxos);

    // sign tx
    let mut txclone = tx.clone();
    // then sign tx with key
    let sighash_sig = SighashCache::new(&mut txclone)
        .taproot_key_spend_signature_hash(
            0,
            &prevouts,
            SchnorrSighashType::All
        ).unwrap();

    let tweak_key_pair = keypair.tap_tweak(&secp, tap_info.merkle_root());

    let msg = Message::from_slice(&sighash_sig).unwrap();
    let sig = secp.sign_schnorr(&msg, &tweak_key_pair.to_inner());
    // verify sig
    secp.verify_schnorr(&sig, &msg, &tweak_key_pair.to_inner().x_only_public_key().0)
        .unwrap();
    let schnorr_sig = SchnorrSig {
        sig,
        hash_ty: SchnorrSighashType::All, // or All
    };
    tx.input[0].witness.push(schnorr_sig.serialize());

    Ok(tx)


    // for index in 0..tx.input.len() {
    //     let mut comp = SighashCache::new(&tx);
    //
    //     // user key signature
    //
    //     let wit = Witness::from_vec(vec![
    //         schnorr_sig.to_vec(),
    //         script.to_bytes(),
    //         actual_control.serialize(),
    //     ]);
    //
    //     tx.input[index].witness.push(wit);
    // }
    // Ok(tx)
}

