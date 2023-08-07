use bitcoin::blockdata::opcodes::all;
use bitcoin::blockdata::script::Builder;
use bitcoin::secp256k1::{All, Message, Secp256k1};
use bitcoin::{Address, KeyPair, Network, SchnorrSig, SchnorrSighashType, Script, Transaction, TxOut, XOnlyPublicKey};
use bitcoin::psbt::Prevouts;
use bitcoin::psbt::serialize::Serialize;
use bitcoin::schnorr::TapTweak;
use bitcoin::util::sighash::SighashCache;
use bitcoin::util::taproot;
use bitcoin::util::taproot::TaprootSpendInfo;
use tracing::info;
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

pub fn sign_tx(
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

// pub fn sign_message(&mut self, msg: &[u8]) -> Result<SchnorrProof, Error> {
//     //Continually compute a new aggregate nonce until we have a valid even R
//     loop {
//         let R = self.compute_aggregate_nonce(msg)?;
//         if R.has_even_y() {
//             break;
//         }
//     }
//
//     // Collect commitments from DKG public share polys for SignatureAggregator
//     let polys: Vec<PolyCommitment> = self
//         .dkg_public_shares
//         .values()
//         .map(|ps| ps.public_share.clone())
//         .collect();
//
//     let mut aggregator = v1::SignatureAggregator::new(self.total_keys, self.threshold, polys)?;
//
//     let nonce_responses: Vec<NonceResponse> = self.public_nonces.values().cloned().collect();
//
//     // Request signature shares
//     self.request_signature_shares(&nonce_responses, msg)?;
//     self.collect_signature_shares()?;
//
//     let nonces = nonce_responses
//         .iter()
//         .flat_map(|nr| nr.nonces.clone())
//         .collect::<Vec<PublicNonce>>();
//     let shares = &self
//         .public_nonces
//         .iter()
//         .flat_map(|(i, _)| self.signature_shares[i].clone())
//         .collect::<Vec<SignatureShare>>();
//
//     // Sign the message using the aggregator
//     let sig = aggregator.sign(msg, &nonces, shares)?;
//
//     // Generate Schnorr proof
//     let proof = SchnorrProof::new(&sig).map_err(Error::Bip340)?;
//
//     // Verify the proof
//     if !proof.verify(&self.aggregate_public_key.x(), msg) {
//         return Err(Error::SchnorrProofFailed);
//     }
//
//     Ok(proof)
//}

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
