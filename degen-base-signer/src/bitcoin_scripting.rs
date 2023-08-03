use bitcoin::blockdata::opcodes::all;
use bitcoin::blockdata::script::Builder;
use bitcoin::secp256k1::{All, Secp256k1};
use bitcoin::{Address, Network, Script, XOnlyPublicKey};
use bitcoin::util::taproot;
use crate::bitcoin_node::LocalhostBitcoinNode;

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
    internal_xonly_public_key: XOnlyPublicKey, // TODO: degens change to aggregate public key
    script_1: &Script,
    script_2: &Script,
) -> (taproot::TaprootSpendInfo, Address) {
    let builder = taproot::TaprootBuilder::with_huffman_tree(vec![
        (1, script_1.clone()),
        (1, script_2.clone()),
    ]).unwrap(); // TODO: degens - or use unwrap check it

    let tap_info = builder.finalize(secp, internal_xonly_public_key).unwrap();
    let address = Address::p2tr(
        secp,
        tap_info.internal_key(),
        tap_info.merkle_root(),
        Network::Regtest,
    );

    (tap_info, address)
}

// pub fn get_current_block_height(client: &LocalhostBitcoinNode) -> u64 {
//     client.get_blockcount().unwrap()
// }

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
