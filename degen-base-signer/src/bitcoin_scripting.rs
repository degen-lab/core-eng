use std::str::FromStr;
use bitcoin::blockdata::opcodes::all;
use bitcoin::blockdata::script::Builder;
use bitcoin::secp256k1::{All, Message, Secp256k1};
use bitcoin::{Address, KeyPair, Network, OutPoint, PackedLockTime, SchnorrSig, SchnorrSighashType, Script, Sequence, Transaction, Txid, TxIn, TxOut, Witness, XOnlyPublicKey};
use bitcoin::psbt::Prevouts;
use bitcoin::psbt::serialize::Serialize;
use bitcoin::schnorr::TapTweak;
use bitcoin::util::sighash::{ScriptPath, SighashCache};
use bitcoin::util::taproot;
use bitcoin::util::taproot::{ControlBlock, LeafVersion, TaprootSpendInfo};
use crate::bitcoin_node::{LocalhostBitcoinNode, UTXO};

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
    internal: &KeyPair, // TODO: degens change to aggregate public key
    script_1: &Script,
    script_2: &Script,
) -> (TaprootSpendInfo, Address) {
    let builder = taproot::TaprootBuilder::with_huffman_tree(vec![
        (1, script_1.clone()),
        (1, script_2.clone()),
    ]).unwrap(); // TODO: degens - or use unwrap check it

    let (internal_public_key, _) = internal.x_only_public_key();

    let tap_info = builder.finalize(secp, internal_public_key).unwrap();
    let address = Address::p2tr(
        secp,
        tap_info.internal_key(),
        tap_info.merkle_root(),
        Network::Regtest,
    );

    (tap_info, address)
}

pub fn get_current_block_height(client: &LocalhostBitcoinNode) -> u64 {
    client.get_block_count().unwrap()
}

pub fn create_tx_from_user_to_script (
    outputs_vec: &Vec<UTXO>,
    user_address: &Address,
    script_address: &Address,
    amount: u64,
    fee: u64,
    tx_index: usize,
) -> (Transaction, u64) {
    let outpoint = OutPoint::new(
        Txid::from_str(&outputs_vec[tx_index].txid.as_str()).unwrap(), 
        outputs_vec[tx_index].vout.clone()
    );

    let left_amount = &outputs_vec[tx_index].amount - amount - fee;

    (Transaction {
        version: 2,
        lock_time: PackedLockTime(0),
        input: vec![TxIn {
            previous_output: outpoint,
            script_sig: Script::new(),
            sequence: Sequence(0x8030FFFF),
            witness: Witness::default(),
        }],
        output: vec![
            TxOut {
                value: amount,
                script_pubkey: user_address.script_pubkey(),
            },
            TxOut {
                value: left_amount,
                script_pubkey: script_address.script_pubkey(),
            }
        ],
    }, left_amount)
}

pub fn sign_tx_user_to_script(
    secp: &Secp256k1<All>,
    tx_ref: &Transaction,
    prevouts: &Prevouts<TxOut>,
    key_pair_internal: &KeyPair,
) -> Transaction {
    let mut tx = tx_ref.clone();
    let sighash_sig = SighashCache::new(&mut tx.clone())
        .taproot_key_spend_signature_hash(0, prevouts, SchnorrSighashType::AllPlusAnyoneCanPay) // or All
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

    tx.input[0].witness.push(schnorr_sig.serialize());

    tx
}

/// uses key sign
pub fn sign_tx_script_to_pox(
    secp: &Secp256k1<All>,
    tx_ref: &Transaction,
    prevouts: &Prevouts<TxOut>,
    key_pair_internal: &KeyPair,
    tap_info: &TaprootSpendInfo,
) -> Transaction {
    let mut tx = tx_ref.clone();
    let sighash_sig = SighashCache::new(&mut tx.clone())
        .taproot_key_spend_signature_hash(0, prevouts, SchnorrSighashType::AllPlusAnyoneCanPay) // or All
        .unwrap();

    let tweak_key_pair = key_pair_internal.tap_tweak(secp, tap_info.merkle_root());
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

    tx.input[0].witness.push(schnorr_sig.serialize());

    tx
}

/// uses script sign
/// TODO: how to sign multiple inputs using this?
pub fn sign_tx_script_refund(
    secp: &Secp256k1<All>,
    tx_ref: &Transaction,
    prevouts: &Prevouts<TxOut>,
    script: &Script,
    key_pair_user: &KeyPair,
    tap_info: &TaprootSpendInfo,
) -> Transaction {
    let mut tx = tx_ref.clone();
    let sighash_sig = SighashCache::new(&mut tx.clone())
        .taproot_script_spend_signature_hash(
            0,
            prevouts,
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
    // println!("actual_control: {:#?}", actual_control);

    // verify commitment
    // TODO: modify verify_p2tr_commitment to not use key_pair_internal
    // we don't have private/secret key for aggregated key in refund path
    verify_p2tr_commitment(secp, script, key_pair_user, tap_info, &actual_control);

    let schnorr_sig = SchnorrSig {
        sig,
        hash_ty: SchnorrSighashType::AllPlusAnyoneCanPay,
    };

    let wit = Witness::from_vec(vec![
        schnorr_sig.to_vec(),
        script.to_bytes(),
        actual_control.serialize(),
    ]);

    tx.input[0].witness = wit;

    tx
}

fn verify_p2tr_commitment(
    secp: &Secp256k1<All>,
    script: &Script,
    key_pair_user: &KeyPair,
    tap_info: &TaprootSpendInfo,
    actual_control: &ControlBlock,
) {
    let tweak_key_pair = key_pair_user
        .tap_tweak(&secp, tap_info.merkle_root())
        .to_inner();
    let (tweak_key_pair_public_key, _) = tweak_key_pair.x_only_public_key();
    assert!(actual_control.verify_taproot_commitment(secp, tweak_key_pair_public_key, script));
}

pub fn create_refund_tx(
    // outputs_vec: &Vec<UTXO>,
    outpoint: OutPoint,
    user_address: &Address,
    amount_left: u64,
    fee: u64,
) -> Transaction {
    // let prev_output_txid_string = &outputs_vec[tx_index].txid;
    // let prev_output_txid = Txid::from_str(prev_output_txid_string.as_str()).unwrap();
    // let prev_output_vout = outputs_vec[tx_index].vout.clone();
    // let outpoint = OutPoint::new(prev_output_txid, prev_output_vout);

    // let left_amount = &outputs_vec[tx_index].amount - fee;
    let amount = amount_left - fee;

    Transaction {
        version: 2,
        lock_time: PackedLockTime(100),
        input: vec![TxIn {
            previous_output: outpoint,
            script_sig: Script::new(),
            sequence: Sequence(0x8030FFFF),
            witness: Witness::default(),
        }],
        output: vec![
            TxOut {
                value: amount,
                script_pubkey: user_address.script_pubkey(),
            },
        ],
    }
}