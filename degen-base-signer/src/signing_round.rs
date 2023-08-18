use hashbrown::{HashMap, HashSet};
use p256k1::{
    ecdsa,
    point::{Compressed, Point},
    scalar::Scalar,
};
use rand_core::{CryptoRng, OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::str::FromStr;
use bdk::miniscript::psbt::SighashError;
use bitcoin::{EcdsaSighashType, KeyPair, Network, OutPoint, PrivateKey, PublicKey, SchnorrSighashType, Script, TxOut, Witness, XOnlyPublicKey};
use bitcoin::blockdata::opcodes::all;
use bitcoin::blockdata::script::Builder;
use bitcoin::consensus::serialize;
use bitcoin::hashes::Hash;
use bitcoin::psbt::{PartiallySignedTransaction, Prevouts};
use bitcoin::secp256k1::{Secp256k1, SecretKey};
use bitcoin::util::sighash::SighashCache;
use bitcoin::util::{base58, taproot};
use blockstack_lib::burnchains::{Address, BurnchainBlockHeader, BurnchainTransaction, PrivateKey as PrivateKeyTrait, Txid};
use blockstack_lib::burnchains::bitcoin::{BitcoinNetworkType, BitcoinTransaction, BitcoinTxOutput};
use blockstack_lib::burnchains::bitcoin::address::{BitcoinAddress, SegwitBitcoinAddress};
use blockstack_lib::chainstate::burn::Opcodes;
use blockstack_lib::chainstate::burn::operations::PegOutRequestOp;
use blockstack_lib::chainstate::stacks::{StacksPrivateKey, TransactionVersion};
use blockstack_lib::chainstate::stacks::address::PoxAddress;
use blockstack_lib::types::chainstate::{BurnchainHeaderHash, StacksAddress};
use blockstack_lib::util::hash::{Hash160, Sha256Sum};
use blockstack_lib::vm::ContractName;
use rand::Rng;
use tracing::{debug, info, warn};
use url::Url;
pub use wsts;
use wsts::{
    common::{PolyCommitment, PublicNonce, SignatureShare},
    traits::Signer as SignerTrait,
    v1,
};
use wsts::bip340::test_helpers::sign;

use crate::{
    config::PublicKeys,
    signer::Signer as FrostSigner,
    state_machine::{Error as StateMachineError, StateMachine, States},
    util::{decrypt, encrypt, make_shared_secret},
};
use crate::bitcoin_node::{BitcoinNode, LocalhostBitcoinNode, UTXO};
use crate::bitcoin_scripting::{create_refund_tx, create_script_refund, create_script_unspendable, create_tree, create_tx_from_user_to_script, get_current_block_height, sign_tx_script_refund, sign_tx_user_to_script};
use crate::bitcoin_wallet::BitcoinWallet;
use crate::peg_wallet::BitcoinWallet as BitcoinWalletTrait;
use crate::stacks_node::client::NodeClient;
use crate::stacks_wallet::StacksWallet;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("InvalidPartyID")]
    InvalidPartyID,
    #[error("InvalidDkgPublicShare")]
    InvalidDkgPublicShare,
    #[error("InvalidDkgPrivateShares")]
    InvalidDkgPrivateShares(Vec<u32>),
    #[error("InvalidNonceResponse")]
    InvalidNonceResponse,
    #[error("InvalidSignatureShare")]
    InvalidSignatureShare,
    #[error("State Machine Error: {0}")]
    StateMachineError(#[from] StateMachineError),
    #[error("Error occured during signing: {0}")]
    SigningError(#[from] SighashError),
}

pub trait Signable {
    fn hash(&self, hasher: &mut Sha256);

    fn sign(&self, private_key: &Scalar) -> Result<Vec<u8>, ecdsa::Error> {
        let mut hasher = Sha256::new();

        self.hash(&mut hasher);

        let hash = hasher.finalize();
        match ecdsa::Signature::new(hash.as_slice(), private_key) {
            Ok(sig) => Ok(sig.to_bytes().to_vec()),
            Err(e) => Err(e),
        }
    }

    fn verify(&self, signature: &[u8], public_key: &ecdsa::PublicKey) -> bool {
        let mut hasher = Sha256::new();

        self.hash(&mut hasher);

        let hash = hasher.finalize();
        let sig = match ecdsa::Signature::try_from(signature) {
            Ok(sig) => sig,
            Err(_) => return false,
        };

        sig.verify(hash.as_slice(), public_key)
    }
}

pub struct SigningRound {
    pub dkg_id: u64,
    pub dkg_public_id: u64,
    pub sign_id: u64,
    pub sign_nonce_id: u64,
    pub threshold: u32,
    pub total_signers: u32,
    pub total_keys: u32,
    pub signer: Signer,
    pub state: States,
    pub commitments: BTreeMap<u32, PolyCommitment>,
    pub shares: HashMap<u32, HashMap<u32, Vec<u8>>>,
    pub public_nonces: Vec<PublicNonce>,
    pub network_private_key: Scalar,
    pub public_keys: PublicKeys,
    // TODO: should be encrypted, i guess?
    pub contract_name: ContractName,
    pub contract_address: StacksAddress,
    pub stacks_private_key: StacksPrivateKey,
    pub stacks_address: StacksAddress,
    pub stacks_node_rpc_url: Url,
    pub local_stacks_node: NodeClient,
    pub stacks_wallet: StacksWallet,
    pub stacks_version: TransactionVersion,
    pub bitcoin_private_key: SecretKey,
    pub bitcoin_xonly_public_key: XOnlyPublicKey,
    pub bitcoin_node_rpc_url: Url,
    pub local_bitcoin_node: LocalhostBitcoinNode,
    pub bitcoin_wallet: BitcoinWallet,
    pub transaction_fee: u64,
    pub bitcoin_network: Network,

    pub script_addresses: BTreeMap<PublicKey, BitcoinAddress>,

}

pub struct Signer {
    pub frost_signer: v1::Signer,
    pub signer_id: u32,
}

impl StateMachine for SigningRound {
    fn move_to(&mut self, state: States) -> Result<(), StateMachineError> {
        self.can_move_to(&state)?;
        self.state = state;
        Ok(())
    }

    fn can_move_to(&self, state: &States) -> Result<(), StateMachineError> {
        let prev_state = &self.state;
        let accepted = match state {
            States::Idle => true,
            States::DkgPublicDistribute => {
                prev_state == &States::Idle
                    || prev_state == &States::DkgPublicGather
                    || prev_state == &States::DkgPrivateDistribute
            }
            States::DkgPublicGather => prev_state == &States::DkgPublicDistribute,
            States::DkgPrivateDistribute => prev_state == &States::DkgPublicGather,
            States::DkgPrivateGather => prev_state == &States::DkgPrivateDistribute,
            States::SignGather => prev_state == &States::Idle,
            States::Signed => prev_state == &States::SignGather,
            States::DegensScriptDistribute => prev_state == &States::Idle,
            States::DegensScriptGather => prev_state == &States::DegensScriptDistribute,
            // TODO degens: add states for scripts
        };
        if accepted {
            info!("state change from {:?} to {:?}", prev_state, state);
            Ok(())
        } else {
            Err(StateMachineError::BadStateChange(format!(
                "{:?} to {:?}",
                prev_state, state
            )))
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum DkgStatus {
    Success,
    Failure(String),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum MessageTypes {
    DkgBegin(DkgBegin),
    DkgPrivateBegin(DkgBegin),
    DkgEnd(DkgEnd),
    DkgPublicEnd(DkgEnd),
    DkgPublicShare(DkgPublicShare),
    DkgPrivateShares(DkgPrivateShares),
    NonceRequest(NonceRequest),
    NonceResponse(NonceResponse),
    SignShareRequest(SignatureShareRequest),
    SignShareResponse(SignatureShareResponse),
    DegensCreateScriptsRequest(DegensScriptRequest),
    DegensCreateScriptsResponse(DegensScriptResponse),
    DegensSpendScripts(DegensSpendScript),
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DkgPublicShare {
    pub dkg_id: u64,
    pub dkg_public_id: u64,
    pub party_id: u32,
    pub public_share: PolyCommitment,
}

impl Signable for DkgPublicShare {
    fn hash(&self, hasher: &mut Sha256) {
        hasher.update("DKG_PUBLIC_SHARE".as_bytes());
        hasher.update(self.dkg_id.to_be_bytes());
        hasher.update(self.dkg_public_id.to_be_bytes());
        hasher.update(self.party_id.to_be_bytes());
        for a in &self.public_share.A {
            hasher.update(a.compress().as_bytes());
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DkgPrivateShares {
    pub dkg_id: u64,
    pub key_id: u32,
    /// Encrypt the shares using AES-GCM with a key derived from ECDH
    pub private_shares: HashMap<u32, Vec<u8>>,
}

impl Signable for DkgPrivateShares {
    fn hash(&self, hasher: &mut Sha256) {
        hasher.update("DKG_PRIVATE_SHARES".as_bytes());
        hasher.update(self.dkg_id.to_be_bytes());
        hasher.update(self.key_id.to_be_bytes());
        // make sure we iterate sequentially
        // TODO: change this once WSTS goes to 1 based indexing for key_ids, or change to BTreeMap
        for id in 0..self.private_shares.len() as u32 {
            hasher.update(id.to_be_bytes());
            hasher.update(&self.private_shares[&id]);
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DkgBegin {
    pub dkg_id: u64, //TODO: Strong typing for this, alternatively introduce a type alias
}

impl Signable for DkgBegin {
    fn hash(&self, hasher: &mut Sha256) {
        hasher.update("DKG_BEGIN".as_bytes());
        hasher.update(self.dkg_id.to_be_bytes());
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DkgEnd {
    pub dkg_id: u64,
    pub signer_id: u32,
    pub status: DkgStatus,
}

impl Signable for DkgEnd {
    fn hash(&self, hasher: &mut Sha256) {
        hasher.update("DKG_END".as_bytes());
        hasher.update(self.dkg_id.to_be_bytes());
        hasher.update(self.signer_id.to_be_bytes());
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct NonceRequest {
    pub dkg_id: u64,
    pub sign_id: u64,
    pub sign_nonce_id: u64,
}

impl Signable for NonceRequest {
    fn hash(&self, hasher: &mut Sha256) {
        hasher.update("NONCE_REQUEST".as_bytes());
        hasher.update(self.dkg_id.to_be_bytes());
        hasher.update(self.sign_id.to_be_bytes());
        hasher.update(self.sign_nonce_id.to_be_bytes());
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct NonceResponse {
    pub dkg_id: u64,
    pub sign_id: u64,
    pub sign_nonce_id: u64,
    pub signer_id: u32,
    pub key_ids: Vec<u32>,
    pub nonces: Vec<PublicNonce>,
}

impl Signable for NonceResponse {
    fn hash(&self, hasher: &mut Sha256) {
        hasher.update("NONCE_RESPONSE".as_bytes());
        hasher.update(self.dkg_id.to_be_bytes());
        hasher.update(self.sign_id.to_be_bytes());
        hasher.update(self.sign_nonce_id.to_be_bytes());
        hasher.update(self.signer_id.to_be_bytes());

        for key_id in &self.key_ids {
            hasher.update(key_id.to_be_bytes());
        }

        for nonce in &self.nonces {
            hasher.update(nonce.D.compress().as_bytes());
            hasher.update(nonce.E.compress().as_bytes());
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SignatureShareRequest {
    pub dkg_id: u64,
    pub sign_id: u64,
    pub correlation_id: u64,
    pub nonce_responses: Vec<NonceResponse>,
    pub message: Vec<u8>,
}

impl Signable for SignatureShareRequest {
    fn hash(&self, hasher: &mut Sha256) {
        hasher.update("SIGNATURE_SHARE_REQUEST".as_bytes());
        hasher.update(self.dkg_id.to_be_bytes());
        hasher.update(self.sign_id.to_be_bytes());
        hasher.update(self.correlation_id.to_be_bytes());

        for nonce_response in &self.nonce_responses {
            nonce_response.hash(hasher);
        }

        hasher.update(self.message.as_slice());
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct SignatureShareResponse {
    pub dkg_id: u64,
    pub sign_id: u64,
    pub correlation_id: u64,
    pub signer_id: u32,
    pub signature_shares: Vec<SignatureShare>,
}

impl Signable for SignatureShareResponse {
    fn hash(&self, hasher: &mut Sha256) {
        hasher.update("SIGNATURE_SHARE_RESPONSE".as_bytes());
        hasher.update(self.dkg_id.to_be_bytes());
        hasher.update(self.sign_id.to_be_bytes());
        hasher.update(self.correlation_id.to_be_bytes());
        hasher.update(self.signer_id.to_be_bytes());

        for signature_share in &self.signature_shares {
            hasher.update(signature_share.id.to_be_bytes());
            hasher.update(signature_share.z_i.to_bytes());
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DegensScriptRequest {
    pub dkg_id: u64,
}

impl Signable for DegensScriptRequest {
    fn hash(&self, hasher: &mut Sha256) {
        hasher.update("DEGENS_CREATE_SCRIPT_REQUEST".as_bytes());
        hasher.update(self.dkg_id.to_be_bytes());
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DegensScriptResponse {
    pub dkg_id: u64,
    pub public_key: XOnlyPublicKey,
    pub script: BitcoinAddress,
    pub block_height: u64,
}

impl Signable for DegensScriptResponse {
    fn hash(&self, hasher: &mut Sha256) {
        hasher.update("DEGENS_CREATE_SCRIPT_RESPONSE".as_bytes());
        hasher.update(self.dkg_id.to_be_bytes());
        hasher.update(self.public_key.to_string());
        hasher.update(self.script.to_string()); // TODO: check if alright
        hasher.update(self.block_height.to_be_bytes());
    }
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct DegensSpendScript {
    pub dkg_id: u64,
    pub addresses: Vec<String>, // TODO degens: update to address type/alias
}

impl Signable for DegensSpendScript {
    fn hash(&self, hasher: &mut Sha256) {
        hasher.update("DEGENS_SPEND_SCRIPT".as_bytes());
        hasher.update(self.dkg_id.to_be_bytes());
        for address in &self.addresses {
            hasher.update(address.as_bytes());
        }
    }
}

impl SigningRound {
    pub fn new(
        threshold: u32,
        total_signers: u32,
        total_keys: u32,
        signer_id: u32,
        key_ids: Vec<u32>,
        network_private_key: Scalar,
        public_keys: PublicKeys,
    ) -> SigningRound {
        assert!(threshold <= total_keys);
        let mut rng = OsRng;
        let frost_signer = v1::Signer::new(signer_id, &key_ids, total_keys, threshold, &mut rng);
        let signer = Signer {
            frost_signer,
            signer_id,
        };

        SigningRound {
            dkg_id: 0,
            dkg_public_id: 0,
            sign_id: 1,
            sign_nonce_id: 1,
            threshold,
            total_signers,
            total_keys,
            signer,
            state: States::Idle,
            commitments: BTreeMap::new(),
            shares: HashMap::new(),
            public_nonces: vec![],
            network_private_key,
            public_keys,
            contract_name: ContractName::from(""),
            contract_address: StacksAddress::new(26, Hash160([0; 20])),
            stacks_private_key: StacksPrivateKey::new(),
            stacks_address: StacksAddress::new(26, Hash160([0; 20])),
            stacks_node_rpc_url: Url::from_str("").unwrap(),
            local_stacks_node: NodeClient::new(
                Url::from_str("").unwrap(),
                ContractName::from(""),
                StacksAddress::new(26, Hash160([0; 20]))
            ),
            stacks_wallet: StacksWallet::new(
                ContractName::from(""),
                StacksAddress::from_string("").unwrap(),
                StacksPrivateKey::new(),
                StacksAddress::new(26, Hash160([0; 20])),
                TransactionVersion::Testnet,
                0,
            ),
            stacks_version: TransactionVersion::Testnet,
            bitcoin_private_key: SecretKey::new(&mut rng),
            bitcoin_xonly_public_key: SecretKey::new(&mut rng).x_only_public_key(&Secp256k1::new()).0,
            bitcoin_node_rpc_url: Url::from_str("").unwrap(),
            local_bitcoin_node: LocalhostBitcoinNode::new(Url::from_str("").unwrap()),
            bitcoin_wallet: BitcoinWallet::new(
                XOnlyPublicKey::from_str("cc8a4bc64d897bddc5fbc2f670f7a8ba0b386779106cf1223c6fc5d7cd6fc115").unwrap(),
                Network::Regtest,
            ),
            transaction_fee: 0,
            bitcoin_network: Network::Regtest,
            script_addresses: BTreeMap::new(),
        }
    }

    fn reset<T: RngCore + CryptoRng>(&mut self, dkg_id: u64, rng: &mut T) {
        self.dkg_id = dkg_id;
        self.dkg_public_id = 0;
        self.commitments.clear();
        self.shares.clear();
        self.public_nonces.clear();
        self.signer.frost_signer.reset_polys(rng);
    }

    pub fn process(&mut self, message: MessageTypes) -> Result<Vec<MessageTypes>, Error> {
        let out_msgs = match message {
            MessageTypes::DkgBegin(dkg_begin) => self.dkg_begin(dkg_begin),
            MessageTypes::DkgPrivateBegin(_) => self.dkg_private_begin(),
            MessageTypes::DkgPublicShare(dkg_public_shares) => {
                self.dkg_public_share(dkg_public_shares)
            }
            MessageTypes::DkgPrivateShares(dkg_private_shares) => {
                self.dkg_private_shares(dkg_private_shares)
            }
            MessageTypes::SignShareRequest(sign_share_request) => {
                self.sign_share_request(sign_share_request)
            }
            MessageTypes::NonceRequest(nonce_request) => {
                self.nonce_request(nonce_request)
            }
            MessageTypes::DegensCreateScriptsRequest(degens_create_script) => {
                self.degen_create_script(degens_create_script)
            }

            _ => Ok(vec![]), // TODO
        };

        match out_msgs {
            Ok(mut out) => {
                if self.public_shares_done() {
                    debug!(
                        "public_shares_done==true. commitments {}",
                        self.commitments.len()
                    );
                    let dkg_end_msgs = self.dkg_public_ended()?;
                    out.push(dkg_end_msgs);
                    self.move_to(States::DkgPrivateDistribute)?;
                } else if self.can_dkg_end() {
                    debug!(
                        "can_dkg_end==true. shares {} commitments {}",
                        self.shares.len(),
                        self.commitments.len()
                    );
                    let dkg_end_msgs = self.dkg_ended()?;
                    out.push(dkg_end_msgs);
                    self.move_to(States::Idle)?;
                }
                Ok(out)
            }
            Err(e) => Err(e),
        }
    }

    fn dkg_public_ended(&mut self) -> Result<MessageTypes, Error> {
        let dkg_end = DkgEnd {
            dkg_id: self.dkg_id,
            signer_id: self.signer.signer_id,
            status: DkgStatus::Success,
        };
        let dkg_end = MessageTypes::DkgPublicEnd(dkg_end);
        info!(
            "DKG_END round #{} signer_id {}",
            self.dkg_id, self.signer.signer_id
        );
        Ok(dkg_end)
    }

    fn dkg_ended(&mut self) -> Result<MessageTypes, Error> {
        let polys: Vec<PolyCommitment> = self.commitments.clone().into_values().collect();

        let mut decrypted_shares = HashMap::new();

        // go through private shares, and decrypt any for owned keys, leaving the rest as zero scalars
        let key_ids: HashSet<u32> = self.signer.frost_signer.get_key_ids().into_iter().collect();
        let mut invalid_dkg_private_shares = Vec::new();

        for (src_key_id, encrypted_shares) in &self.shares {
            let mut decrypted_key_shares = HashMap::new();

            for (dst_key_id, private_share) in encrypted_shares {
                if key_ids.contains(dst_key_id) {
                    debug!(
                        "decrypting dkg private share for key_id #{}",
                        dst_key_id + 1
                    );
                    let compressed =
                        Compressed::from(self.public_keys.key_ids[&(src_key_id + 1)].to_bytes());
                    let src_public_key = Point::try_from(&compressed).unwrap();
                    let shared_secret =
                        make_shared_secret(&self.network_private_key, &src_public_key);

                    match decrypt(&shared_secret, private_share) {
                        Ok(plain) => match Scalar::try_from(&plain[..]) {
                            Ok(s) => {
                                decrypted_key_shares.insert(*dst_key_id, s);
                            }
                            Err(e) => {
                                warn!("Failed to parse Scalar for dkg private share from key_id {} to key_id {}: {:?}", src_key_id, dst_key_id, e);
                                invalid_dkg_private_shares.push(*src_key_id);
                            }
                        },
                        Err(e) => {
                            warn!("Failed to decrypt dkg private share from key_id {} to key_id {}: {:?}", src_key_id, dst_key_id, e);
                            invalid_dkg_private_shares.push(*src_key_id);
                        }
                    }
                } else {
                    decrypted_key_shares.insert(*dst_key_id, Scalar::new());
                }
            }

            decrypted_shares.insert(*src_key_id, decrypted_key_shares);
        }

        let dkg_end = if invalid_dkg_private_shares.is_empty() {
            match self
                .signer
                .frost_signer
                .compute_secrets(&decrypted_shares, &polys)
            {
                Ok(()) => DkgEnd {
                    dkg_id: self.dkg_id,
                    signer_id: self.signer.signer_id,
                    status: DkgStatus::Success,
                },
                Err(dkg_error_map) => DkgEnd {
                    dkg_id: self.dkg_id,
                    signer_id: self.signer.signer_id,
                    status: DkgStatus::Failure(format!("{:?}", dkg_error_map)),
                },
            }
        } else {
            DkgEnd {
                dkg_id: self.dkg_id,
                signer_id: self.signer.signer_id,
                status: DkgStatus::Failure(format!("{:?}", invalid_dkg_private_shares)),
            }
        };

        let dkg_end = MessageTypes::DkgEnd(dkg_end);
        info!(
            "DKG_END round #{} signer_id {}",
            self.dkg_id, self.signer.signer_id
        );
        Ok(dkg_end)
    }

    fn public_shares_done(&self) -> bool {
        debug!(
            "public_shares_done state {:?} commitments {}",
            self.state,
            self.commitments.len(),
        );
        self.state == States::DkgPublicGather
            && self.commitments.len() == usize::try_from(self.total_keys).unwrap()
    }

    fn can_dkg_end(&self) -> bool {
        debug!(
            "can_dkg_end state {:?} commitments {} shares {}",
            self.state,
            self.commitments.len(),
            self.shares.len()
        );
        self.state == States::DkgPrivateGather
            && self.commitments.len() == usize::try_from(self.total_keys).unwrap()
            && self.shares.len() == usize::try_from(self.total_keys).unwrap()
    }

    fn nonce_request(&mut self, nonce_request: NonceRequest) -> Result<Vec<MessageTypes>, Error> {
        let mut rng = OsRng;
        let mut msgs = vec![];
        let signer_id = self.signer.signer_id;
        let key_ids = self.signer.frost_signer.get_key_ids();
        let nonces = self.signer.frost_signer.gen_nonces(&mut rng);

        let response = NonceResponse {
            dkg_id: nonce_request.dkg_id,
            sign_id: nonce_request.sign_id,
            sign_nonce_id: nonce_request.sign_nonce_id,
            signer_id,
            key_ids,
            nonces,
        };

        let response = MessageTypes::NonceResponse(response);

        info!(
            "nonce request with dkg_id {:?}. response sent from signer_id {}",
            nonce_request.dkg_id, signer_id
        );
        msgs.push(response);

        Ok(msgs)
    }

    fn sign_share_request(
        &mut self,
        sign_request: SignatureShareRequest,
    ) -> Result<Vec<MessageTypes>, Error> {
        let mut msgs = vec![];

        let signer_ids = sign_request
            .nonce_responses
            .iter()
            .map(|nr| nr.signer_id)
            .collect::<Vec<u32>>();

        info!("Got SignatureShareRequest for signer_ids {:?}", signer_ids);

        for signer_id in &signer_ids {
            if *signer_id == self.signer.signer_id {
                let key_ids: Vec<u32> = sign_request
                    .nonce_responses
                    .iter()
                    .flat_map(|nr| nr.key_ids.iter().copied())
                    .collect::<Vec<u32>>();
                let nonces = sign_request
                    .nonce_responses
                    .iter()
                    .flat_map(|nr| nr.nonces.clone())
                    .collect::<Vec<PublicNonce>>();
                let signature_shares = self.signer.frost_signer.sign(
                    &sign_request.message,
                    &signer_ids,
                    &key_ids,
                    &nonces,
                );

                let response = SignatureShareResponse {
                    dkg_id: sign_request.dkg_id,
                    sign_id: sign_request.sign_id,
                    correlation_id: sign_request.correlation_id,
                    signer_id: *signer_id,
                    signature_shares,
                };

                info!(
                    "Sending SignatureShareResponse for signer_id {:?}",
                    signer_id
                );

                let response = MessageTypes::SignShareResponse(response);

                msgs.push(response);
            } else {
                debug!("SignShareRequest for {} dropped.", signer_id);
            }
        }
        Ok(msgs)
    }

    fn dkg_begin(&mut self, dkg_begin: DkgBegin) -> Result<Vec<MessageTypes>, Error> {
        let mut rng = OsRng;

        self.reset(dkg_begin.dkg_id, &mut rng);
        self.move_to(States::DkgPublicDistribute)?;

        let _party_state = self.signer.frost_signer.save();

        self.dkg_public_begin()
    }

    fn dkg_public_begin(&mut self) -> Result<Vec<MessageTypes>, Error> {
        let mut rng = OsRng;
        let mut msgs = vec![];
        let polys = self.signer.frost_signer.get_poly_commitments(&mut rng);

        info!(
            "sending DkgPublicShares for round #{}, {} poly commitments for signer #{}",
            self.dkg_id,
            polys.len(),
            self.signer.frost_signer.get_id(),
        );

        for poly in &polys {
            let public_share = DkgPublicShare {
                dkg_id: self.dkg_id,
                dkg_public_id: self.dkg_public_id,
                party_id: poly.id.id.get_u32(),
                public_share: poly.clone(),
            };

            let public_share = MessageTypes::DkgPublicShare(public_share);
            msgs.push(public_share);
        }

        self.move_to(States::DkgPublicGather)?;
        Ok(msgs)
    }

    fn dkg_private_begin(&mut self) -> Result<Vec<MessageTypes>, Error> {
        let mut rng = OsRng;
        let mut msgs = vec![];
        for (key_id, private_shares) in &self.signer.frost_signer.get_shares() {
            info!(
                "signer {} sending dkg private share for key_id #{}",
                self.signer.signer_id, key_id
            );
            // encrypt each share for the recipient
            let mut encrypted_shares = HashMap::new();

            for (dst_key_id, private_share) in private_shares {
                debug!(
                    "encrypting dkg private share for key_id #{}",
                    dst_key_id + 1
                );
                let compressed =
                    Compressed::from(self.public_keys.key_ids[&(dst_key_id + 1)].to_bytes());
                let dst_public_key = Point::try_from(&compressed).unwrap();
                let shared_secret = make_shared_secret(&self.network_private_key, &dst_public_key);
                let encrypted_share =
                    encrypt(&shared_secret, &private_share.to_bytes(), &mut rng).unwrap();

                encrypted_shares.insert(*dst_key_id, encrypted_share);
            }

            let private_shares = DkgPrivateShares {
                dkg_id: self.dkg_id,
                key_id: *key_id,
                private_shares: encrypted_shares,
            };

            let private_shares = MessageTypes::DkgPrivateShares(private_shares);
            msgs.push(private_shares);
        }

        self.move_to(States::DkgPrivateGather)?;
        Ok(msgs)
    }

    fn dkg_public_share(
        &mut self,
        dkg_public_share: DkgPublicShare,
    ) -> Result<Vec<MessageTypes>, Error> {
        self.commitments
            .insert(dkg_public_share.party_id, dkg_public_share.public_share);
        info!(
            "received DkgPublicShare from key #{} {}/{}",
            dkg_public_share.party_id,
            self.commitments.len(),
            self.total_keys
        );
        Ok(vec![])
    }

    fn dkg_private_shares(
        &mut self,
        dkg_private_shares: DkgPrivateShares,
    ) -> Result<Vec<MessageTypes>, Error> {
        let shares_clone = dkg_private_shares.private_shares.clone();
        self.shares
            .insert(dkg_private_shares.key_id, dkg_private_shares.private_shares);
        info!(
            "received DkgPrivateShares from key #{} {}/{} {:?}",
            dkg_private_shares.key_id,
            self.shares.len(),
            self.total_keys,
            shares_clone.keys(),
        );
        Ok(vec![])
    }

    fn degen_create_script(&mut self, degens_create_script: DegensScriptRequest) -> Result<Vec<MessageTypes>, Error> {
        // let current_block_height = get_current_block_height(&self.local_bitcoin_node);
        let secp = Secp256k1::new();
        let keypair = KeyPair::from_secret_key(&secp, &self.bitcoin_private_key);

        let script_1 = create_script_refund(
            &self.bitcoin_xonly_public_key,
            100,
        );
        let script_2 = create_script_unspendable();

        let (tap_info, script_address) = create_tree(
            &secp,
            &keypair,
            &script_1,
            &script_2,
        );

        let unspent_list_signer = self.local_bitcoin_node.list_unspent(&self.bitcoin_wallet.address()).expect("Failed to get unspent list for signer.");

        let mut unspent_list_txout: Vec<TxOut> = vec![];
        unspent_list_signer.iter().for_each(|utxo| {
            unspent_list_txout.push(TxOut {
                value: utxo.amount,
                script_pubkey: Script::from_str(utxo.scriptPubKey.as_str()).unwrap(),
            });
        });

        let prevouts_signer = Prevouts::One(0, unspent_list_txout[0].clone());

        let amount_to_script: u64 = 1000;
        let fee: u64 = 300;

        let (user_to_script_unsigned, amount_left) = create_tx_from_user_to_script(
            &unspent_list_signer,
            &self.bitcoin_wallet.address(),
            &script_address,
            amount_to_script,
            fee,
            0);

        let user_to_script_signed = sign_tx_user_to_script(
            &secp,
            &user_to_script_unsigned,
            &prevouts_signer,
            &keypair,
        );

        let txid = self.local_bitcoin_node.broadcast_transaction(&user_to_script_signed).unwrap();

        info!("{txid:#?}");

        let outpoint = OutPoint::new(txid, 1);

        let refund_tx = create_refund_tx(outpoint, self.bitcoin_wallet.address(), amount_left, fee);

        let prevout = Prevouts::One(0, TxOut {value: amount_left, script_pubkey: script_address.script_pubkey()});

        let signed_tx = sign_tx_script_refund(&secp, &refund_tx, &prevout, &script_1, &keypair, &tap_info);

        let signed_txid = self.local_bitcoin_node.broadcast_transaction(&signed_tx).unwrap();
        info!("{signed_txid:#?}");



        // let user_to_script_txid = self.local_bitcoin_node.broadcast_transaction(&user_to_script_unsigned);
        //
        // info!("{:#?}", user_to_script_txid);

        // let unspent_list = self.local_bitcoin_node.list_unspent(&script_address).unwrap();
        // info!("script_address: {script_address:#?}");
        // info!("unspent list: {unspent_list:#?}");
        //
        // let amount = 1000;
        //
        // let pox_addr_1 = bitcoin::Address::from_str("BCRT1P8M4KHK8A06CUCAWGPPQ3GDKEXTH7MZF6DGW54KZ67TKFQ3RCUU5QNKHUDG").unwrap();
        // let pox_addr_2 = bitcoin::Address::from_str("BCRT1P6HNYZU0USW758H2F04GMWDGYWXAK9PAMA9S7AQ7NZEMXVGG0JTHSN9DNZN").unwrap();
        // let addresses_list = vec![pox_addr_1, pox_addr_2, script_address];
        //
        // let script_unsigned_tx = create_tx_from_user_to_script(&unspent_list, &addresses_list, amount, 0);
        //
        // info!("{script_unsigned_tx:#?}");

        // let script_txid = self.local_bitcoin_node.broadcast_transaction(&script_signed_tx);

        // info!("{script_txid:#?}");

        // // create new op using from_tx()
        // // create tx with recipient script address and block header
        // // TODO: degens change amount to readonly sc call
        // let amount: u64 = 1000;

        // let script_address_pubkey = &script_address.script_pubkey();
        // let script_address_bytes = script_address_pubkey.as_bytes();
        //
        // let mut script_pubkey = vec![81, 32]; // OP_1 OP_PUSHBYTES_32
        // script_pubkey.extend_from_slice(&script_address_bytes);
        //
        // let mut msg = amount.to_be_bytes().to_vec();
        // msg.extend_from_slice(&script_pubkey);
        //
        // let signature = self.stacks_private_key
        //     .sign(Sha256Sum::from_data(&msg).as_bytes())
        //     .expect("Failed to sign amount and recipient fields.");
        //
        // let mut data = vec![];
        // data.extend_from_slice(&amount.to_be_bytes());
        // data.extend_from_slice(signature.as_bytes());
        //
        // let output_script_address = BitcoinAddress::from_scriptpubkey(
        //     BitcoinNetworkType::Regtest,
        //     script_address_bytes
        // ).unwrap();
        //
        // let output = BitcoinTxOutput {
        //     address: output_script_address,
        //     units: amount,
        // };
        //
        // let mut rng = rand::thread_rng();
        //
        // let peg_wallet_address = rng.gen::<[u8; 32]>();
        // let output2 = BitcoinTxOutput {
        //     units: amount,
        //     address: BitcoinAddress::Segwit(SegwitBitcoinAddress::P2TR(true, peg_wallet_address)),
        // };
        //
        // let header = BurnchainBlockHeader {
        //     block_height: 0,
        //     block_hash: [0; 32].into(),
        //     parent_block_hash: [0; 32].into(),
        //     num_txs: 0,
        //     timestamp: 0,
        // };
        //
        // let burnchain_tx: BurnchainTransaction = BurnchainTransaction::Bitcoin(BitcoinTransaction {
        //     txid: Txid([0; 32]),
        //     vtxindex: 0,
        //     opcode: Opcodes::PegOutRequest as u8,
        //     data,
        //     data_amt: 0,
        //     inputs: vec![],
        //     outputs: vec![output, output2],
        // });
        //
        // let op = PegOutRequestOp::from_tx(&header, &burnchain_tx).expect("Failed to construct peg-out request op");

        // let (mut tx, prevouts_vec) = self.bitcoin_wallet.script_peg_out(&op, unspent_list).expect("Failed to construct transaction");
        //
        // // TODO: is the tx signed? how to sign it if not?
        // info!("unsigned_tx: {:#?}", tx);
        //
        // // TODO: try here same format as on vs-code
        //
        // let prevout = Prevouts::One(
        //     0,
        //     prevouts_vec[0].clone(),
        // );
        // let sig = sign_key_tx(&secp, &tx, &prevout, &keypair, &tap_info);
        // tx.input[0].witness.push(sig);
        //
        //
        // // for index in 0..tx.input.len() {
        // //     let prevout = Prevouts::One(
        // //         0,
        // //         prevouts_vec[index].clone(),
        // //     );
        // //     let sig = sign_key_tx(&secp, &tx, &prevout, &keypair, &tap_info);
        // //     tx.input[index].witness.push(sig);
        // // }
        //
        // info!("signed_tx: {:#?}", tx);
        //
        // // TODO: broadcast transaction (see how the tx was signed, if the current one isn't)
        //
        // let script_txid = self.local_bitcoin_node.broadcast_transaction(&tx);
        // info!("broadcast_txid: {:#?}", script_txid);

        // TODO: create another pegout with input the script address and output 2 user addresses in bitcoin_wallet

        // TODO: send the message that the script was created and money sent to it
        // TODO: the message contains signer's stacks address and script address (to keep track of the people who sent money), make a list for coordinator

        // send funds to script
        // my private key to spend through it
        // my address


        // retrieve script address/public key


        // how do we want to return the addresses? change type? all functions have this return type
        Ok(vec![])
    }

}

impl From<&FrostSigner> for SigningRound {
    fn from(signer: &FrostSigner) -> Self {
        let signer_id = signer.signer_id;
        let signer_private_key = signer.config.stacks_private_key;
        assert!(signer_id > 0 && signer_id <= signer.config.total_signers);
        let key_ids = signer.config.signer_key_ids[&signer_id]
            .iter()
            .map(|i| i - 1)
            .collect::<Vec<u32>>();

        assert!(signer.config.keys_threshold <= signer.config.total_keys);
        let mut rng = OsRng;
        let frost_signer = v1::Signer::new(
            signer_id,
            &key_ids,
            signer.config.total_keys,
            signer.config.keys_threshold,
            &mut rng,
        );

        let network_private_key = signer.config.network_private_key;
        let public_keys = signer.config.public_keys.clone();

        SigningRound {
            dkg_id: 1,
            dkg_public_id: 1,
            sign_id: 1,
            sign_nonce_id: 1,
            threshold: signer.config.keys_threshold,
            total_keys: signer.config.total_keys,
            total_signers: signer.config.total_signers,
            signer: Signer {
                frost_signer,
                signer_id,
            },
            state: States::Idle,
            commitments: BTreeMap::new(),
            shares: HashMap::new(),
            public_nonces: vec![],
            network_private_key,
            public_keys,
            contract_name: signer.config.contract_name.clone(),
            contract_address: signer.config.contract_address,
            stacks_private_key: signer.config.stacks_private_key,
            stacks_address: signer.config.stacks_address,
            stacks_node_rpc_url: signer.config.stacks_node_rpc_url.clone(),
            local_stacks_node: signer.config.local_stacks_node.clone(),
            stacks_wallet: signer.config.stacks_wallet.clone(),
            stacks_version: signer.config.stacks_version,
            bitcoin_private_key: signer.config.bitcoin_private_key,
            bitcoin_xonly_public_key: signer.config.bitcoin_xonly_public_key,
            bitcoin_node_rpc_url: signer.config.bitcoin_node_rpc_url.clone(),
            local_bitcoin_node: signer.config.local_bitcoin_node.clone(),
            bitcoin_wallet: signer.config.bitcoin_wallet.clone(),
            transaction_fee: signer.config.transaction_fee,
            bitcoin_network: signer.config.bitcoin_network,
            script_addresses: BTreeMap::new(),
        }
    }
}

#[cfg(test)]
mod test {
    use hashbrown::HashMap;
    use rand_core::{CryptoRng, OsRng, RngCore};
    use wsts::{common::PolyCommitment, schnorr::ID, Scalar};

    use crate::signing_round::{
        DkgPrivateShares, DkgPublicShare, DkgStatus, MessageTypes, SigningRound,
    };
    use crate::state_machine::States;

    fn get_rng() -> impl RngCore + CryptoRng {
        let rnd = OsRng;
        //rand::rngs::StdRng::seed_from_u64(rnd.next_u64()) // todo: fix trait `rand_core::RngCore` is not implemented for `StdRng`
        rnd
    }

    #[test]
    fn dkg_public_share() {
        let mut rnd = get_rng();
        let mut signing_round =
            SigningRound::new(1, 1, 1, 1, vec![1], Default::default(), Default::default());
        let public_share = DkgPublicShare {
            dkg_id: 0,
            party_id: 0,
            public_share: PolyCommitment {
                id: ID::new(&Scalar::new(), &Scalar::new(), &mut rnd),
                A: vec![],
            },
            dkg_public_id: 0,
        };
        signing_round.dkg_public_share(public_share).unwrap();
        assert_eq!(1, signing_round.commitments.len())
    }

    #[test]
    fn dkg_private_shares() {
        let mut signing_round =
            SigningRound::new(1, 1, 1, 1, vec![1], Default::default(), Default::default());
        let mut private_shares = DkgPrivateShares {
            dkg_id: 0,
            key_id: 0,
            private_shares: HashMap::new(),
        };
        private_shares.private_shares.insert(1, Vec::new());
        signing_round.dkg_private_shares(private_shares).unwrap();
        assert_eq!(1, signing_round.shares.len())
    }

    #[test]
    fn public_shares_done() {
        let mut rnd = get_rng();
        let mut signing_round =
            SigningRound::new(1, 1, 1, 1, vec![1], Default::default(), Default::default());
        // publich_shares_done starts out as false
        assert_eq!(false, signing_round.public_shares_done());

        // meet the conditions for all public keys received
        signing_round.state = States::DkgPublicGather;
        signing_round.commitments.insert(
            1,
            PolyCommitment {
                id: ID::new(&Scalar::new(), &Scalar::new(), &mut rnd),
                A: vec![],
            },
        );

        // public_shares_done should be true
        assert!(signing_round.public_shares_done());
    }

    #[test]
    fn can_dkg_end() {
        let mut rnd = get_rng();
        let mut signing_round =
            SigningRound::new(1, 1, 1, 1, vec![1], Default::default(), Default::default());
        // can_dkg_end starts out as false
        assert_eq!(false, signing_round.can_dkg_end());

        // meet the conditions for DKG_END
        signing_round.state = States::DkgPrivateGather;
        signing_round.commitments.insert(
            1,
            PolyCommitment {
                id: ID::new(&Scalar::new(), &Scalar::new(), &mut rnd),
                A: vec![],
            },
        );
        let shares: HashMap<u32, Vec<u8>> = HashMap::new();
        signing_round.shares.insert(1, shares);

        // can_dkg_end should be true
        assert!(signing_round.can_dkg_end());
    }

    #[test]
    fn dkg_ended() {
        let mut signing_round =
            SigningRound::new(1, 1, 1, 1, vec![1], Default::default(), Default::default());
        match signing_round.dkg_ended() {
            Ok(dkg_end) => match dkg_end {
                MessageTypes::DkgEnd(dkg_end) => match dkg_end.status {
                    DkgStatus::Failure(_) => assert!(true),
                    _ => assert!(false),
                },
                _ => assert!(false),
            },
            _ => assert!(false),
        }
    }
}
