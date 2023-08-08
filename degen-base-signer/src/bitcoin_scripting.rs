use std::str::FromStr;
use bitcoin::blockdata::opcodes::all;
use bitcoin::blockdata::script::Builder;
use bitcoin::secp256k1::{All, Message, Secp256k1, SecretKey};
use bitcoin::{Address, EcdsaSig, EcdsaSighashType, KeyPair, Network, OutPoint, PackedLockTime, PrivateKey, PublicKey, SchnorrSig, SchnorrSighashType, Script, Sequence, Transaction, Txid, TxIn, TxOut, Witness, XOnlyPublicKey};
use bitcoin::psbt::{Input, Prevouts};
use bitcoin::psbt::serialize::Serialize;
use bitcoin::schnorr::TapTweak;
use bitcoin::util::sighash::SighashCache;
use bitcoin::util::taproot;
use bitcoin::util::taproot::TaprootSpendInfo;
use tracing::info;
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

pub fn sign_key_tx(
    secp: &Secp256k1<All>,
    tx_ref: &Transaction,
    prevouts: &Prevouts<TxOut>,
    key_pair_internal: &KeyPair,
    tap_info: &TaprootSpendInfo,
) -> Vec<u8> {
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

    info!("{schnorr_sig:#?}");

    schnorr_sig.serialize()
}

pub fn create_tx_from_user_to_script (
    outputs_vec: &Vec<UTXO>,
    user_address: &Address,
    script_address: &Address,
    amount: u64,
    fee: u64,
    tx_index: usize,
) -> Transaction {
    let prev_output_txid_string = &outputs_vec[tx_index].txid;
    let prev_output_txid = Txid::from_str(prev_output_txid_string.as_str()).unwrap();
    let prev_output_vout = outputs_vec[tx_index].vout.clone();
    let outpoint = OutPoint::new(prev_output_txid, prev_output_vout);

    let left_amount = &outputs_vec[tx_index].amount - amount - fee;

    Transaction {
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
    }
}

pub fn sign_user_to_script_tx (
    secp: &Secp256k1<All>,
    current_tx: &Transaction,
    amount: u64,
    fee: u64,
    input_index: usize,
    secret_key: SecretKey,
) -> Transaction {
    let public_key = PublicKey::from_private_key(secp, &PrivateKey::new(secret_key, Network::Regtest));

    let script = Builder::new()
        .push_opcode(all::OP_DUP)
        .push_opcode(all::OP_HASH160)
        .push_slice(&Script::new_v0_p2wpkh(&public_key.wpubkey_hash().unwrap())[2..])
        .push_opcode(all::OP_EQUALVERIFY)
        .push_opcode(all::OP_CHECKSIG)
        .into_script();

    let total_amount = amount + fee;

    let sig_hash = SighashCache::new(&mut current_tx.clone())
        .segwit_signature_hash(
            input_index,
            &script,
            total_amount,
            EcdsaSighashType::All,
        )
        .unwrap();

    let msg = Message::from_slice(&sig_hash).unwrap();
    let sig = EcdsaSig::sighash_all(secp.sign_ecdsa(&msg, &secret_key));

    let mut tx = current_tx.clone();

    tx.input[input_index].witness.push(sig.to_vec());

    tx
}