use std::str::FromStr;
use bitcoin::blockdata::opcodes::all;
use bitcoin::blockdata::script::Builder;
use bitcoin::secp256k1::{All, Message, Secp256k1};
use bitcoin::{Address, KeyPair, Network, OutPoint, PackedLockTime, SchnorrSig, SchnorrSighashType, Script, Sequence, Transaction, Txid, TxIn, TxOut, Witness, XOnlyPublicKey};
use bitcoin::psbt::Prevouts;
use bitcoin::psbt::serialize::Serialize;
use bitcoin::schnorr::{TapTweak, TweakedPublicKey};
use bitcoin::util::sighash::{ScriptPath, SighashCache};
use bitcoin::util::taproot;
use bitcoin::util::taproot::{ControlBlock, LeafVersion, TaprootSpendInfo};
use crate::bitcoin_node::{LocalhostBitcoinNode, UTXO};
use crate::signing_round::UtxoError::InvalidUTXO;

pub fn create_script_refund(
    user_public_key: &XOnlyPublicKey,
    unlock_block: usize,
) -> Script {
    Builder::new()
        .push_int(unlock_block as i64)
        .push_opcode(all::OP_CLTV)
        .push_opcode(all::OP_DROP)
        .push_x_only_key(user_public_key)
        .push_opcode(all::OP_CHECKSIG)
        .into_script()
}

pub fn create_script_unspendable() -> Script {
    Builder::new().push_opcode(all::OP_RETURN).into_script()
}

pub fn create_tree(
    secp: &Secp256k1<All>,
    aggregate_x_only: XOnlyPublicKey,
    network: Network,
    script_1: &Script,
    script_2: &Script,
) -> (TaprootSpendInfo, Address) {
    let builder = taproot::TaprootBuilder::with_huffman_tree(vec![
        (1, script_1.clone()),
        (1, script_2.clone()),
    ]).unwrap();

    let tap_info = builder.finalize(secp, aggregate_x_only).unwrap();

    // let tweaked_public_key = TweakedPublicKey::dangerous_assume_tweaked(aggregate_x_only);
    // let address_tweaked = Address::p2tr_tweaked(tweaked_public_key, network);

    let address = Address::p2tr(
        secp,
        tap_info.internal_key(),
        tap_info.merkle_root(),
        network,
    );

    (tap_info, address)
}

pub fn get_current_block_height(client: &LocalhostBitcoinNode) -> u64 {
    client.get_block_count().unwrap()
}

pub fn create_tx_from_user_to_script(
    previous_outputs_vec: &Vec<UTXO>,
    user_address: &Address,
    script_address: &Address,
    amount: u64,
    fee_to_script: u64,
    fee_to_pox: u64,
) -> Transaction {
    let mut inputs = vec![];
    let mut total_utxo_amount: u64 = 0;

    for position in 0..previous_outputs_vec.len() {
        let outpoint = OutPoint::new(
            Txid::from_str(&previous_outputs_vec[position].txid.as_str()).unwrap(),
            previous_outputs_vec[position].vout.clone()
        );

        total_utxo_amount = total_utxo_amount + previous_outputs_vec[position].amount;

        inputs.push(
            TxIn {
                previous_output: outpoint,
                script_sig: Script::new(),
                sequence: Sequence(0x8030FFFF),
                witness: Witness::default(),
            }
        )
    }

    let amount_back_to_user = total_utxo_amount - amount - fee_to_script - fee_to_pox;

    if amount_back_to_user != 0 {
        Transaction {
            version: 2,
            lock_time: PackedLockTime(0),
            input: inputs,
            output: vec![
                TxOut {
                    value: amount + fee_to_pox,
                    script_pubkey: script_address.script_pubkey(),
                },
                TxOut {
                    value: amount_back_to_user,
                    script_pubkey: user_address.script_pubkey(),
                }
            ],
        }
    }
    else {
        Transaction {
            version: 2,
            lock_time: PackedLockTime(0),
            input: inputs,
            output: vec![
                TxOut {
                    value: amount + fee_to_pox,
                    script_pubkey: script_address.script_pubkey(),
                }
            ],
        }
    }
}

/// uses script sign
pub fn sign_tx_script_refund(
    secp: &Secp256k1<All>,
    tx_ref: &Transaction,
    txout_vec: &Vec<TxOut>,
    script: &Script,
    key_pair_user: &KeyPair,
    tap_info: &TaprootSpendInfo,
) -> Transaction {
    let mut tx = tx_ref.clone();

    let prevouts = Prevouts::All(txout_vec);

    for position in 0..txout_vec.len() {
        let sighash_sig = SighashCache::new(&mut tx.clone())
            .taproot_script_spend_signature_hash(
                position,
                &prevouts,
                ScriptPath::with_defaults(script),
                SchnorrSighashType::AllPlusAnyoneCanPay,
            )
            .unwrap();
        // println!("sighash_sig: {}", sighash_sig);
        // println!("message: {}", Message::from_slice(&sighash_sig).unwrap());
        let msg = Message::from_slice(&sighash_sig).unwrap();
        let sig = secp.sign_schnorr(&msg, key_pair_user);
        // println!("sig: {}", sig);

        let actual_control = tap_info
            .control_block(&(script.clone(), LeafVersion::TapScript))
            .unwrap();

        // TODO: degens - verify_p2tr_commitment works with key_pair_internal but not with key_pair_from_script
        // we don't have private/secret key for aggregated key in refund path
        // modifiy it to work with it or remove it
        // verify_p2tr_commitment(secp, script, key_pair_user, tap_info, &actual_control);

        let schnorr_sig = SchnorrSig {
            sig,
            hash_ty: SchnorrSighashType::AllPlusAnyoneCanPay,
        };

        let wit = Witness::from_vec(vec![
            schnorr_sig.to_vec(),
            script.to_bytes(),
            actual_control.serialize(),
        ]);

        tx.input[position].witness = wit;
    }

    tx
}

pub fn create_refund_tx(
    utxos: &Vec<UTXO>,
    user_address: &Address,
    amount: u64,
) -> Transaction {
    let mut inputs = vec![];

    for utxo in utxos {
        let prev_output_txid_string = &utxo.txid;
        let prev_output_txid = Txid::from_str(prev_output_txid_string.as_str()).unwrap();
        let prev_output_vout = utxo.vout.clone();
        let outpoint = OutPoint::new(prev_output_txid, prev_output_vout);

        inputs.push(
            TxIn {
                previous_output: outpoint,
                script_sig: Script::new(),
                sequence: Sequence(0x8030FFFF),
                witness: Witness::default(),
            }
        )
    }

    Transaction {
        version: 2,
        lock_time: PackedLockTime(100),
        input: inputs,
        output: vec![
            TxOut {
                value: amount,
                script_pubkey: user_address.script_pubkey(),
            },
        ],
    }
}

pub fn sign_tx_user_to_script(
    secp: &Secp256k1<All>,
    tx_ref: &Transaction,
    prevouts: &Prevouts<TxOut>,
    key_pair_internal: &KeyPair,
) -> Transaction {
    let mut tx = tx_ref.clone();

    for position in 0..tx_ref.input.len() {
        let sighash_sig = SighashCache::new(&mut tx.clone())
            .taproot_key_spend_signature_hash(position, prevouts, SchnorrSighashType::AllPlusAnyoneCanPay) // or All
            .unwrap();

        let tweak_key_pair = key_pair_internal.tap_tweak(secp, None);
        // then sig
        let msg = Message::from_slice(&sighash_sig).unwrap();

        let sig = secp.sign_schnorr(&msg, &tweak_key_pair.to_inner());

        //verify sig
        secp.verify_schnorr(&sig, &msg, &tweak_key_pair.to_inner().x_only_public_key().0)
            .unwrap();

        // then witness
        let schnorr_sig = SchnorrSig {
            sig,
            hash_ty: SchnorrSighashType::AllPlusAnyoneCanPay, // or All
        };

        tx.input[position].witness.push(schnorr_sig.serialize());
    }

    tx
}