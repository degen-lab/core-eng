use std::any::Any;
use std::collections::BTreeMap;
use std::time::Duration;
use bitcoin::{Address, Txid};
use blockstack_lib::burnchains::bitcoin::address::BitcoinAddress;
use degen_base_signer::signing_round::UtxoError;

use degen_base_signer::config::{Config, Error as ConfigError};
use degen_base_signer::{
    net::{Error as HttpNetError, Message, NetListen},
    signing_round::{
        DkgBegin, DkgPublicShare, MessageTypes, NonceRequest, NonceResponse, Signable,
        SignatureShareRequest,
    },
};
use hashbrown::HashSet;
use p256k1::ecdsa::PublicKey;
use tracing::{debug, info, warn};
use wsts::{
    bip340::{Error as Bip340Error, SchnorrProof},
    common::{PolyCommitment, PublicNonce, Signature, SignatureShare},
    compute,
    errors::AggregatorError,
    v1, Point, Scalar,
};
use degen_base_signer::bitcoin_node::UTXO;
use degen_base_signer::signing_round::DegensScriptRequest;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    NetworkError(#[from] HttpNetError),
    #[error("No aggregate public key")]
    NoAggregatePublicKey,
    #[error("Aggregator failed to sign: {0}")]
    Aggregator(#[from] AggregatorError),
    #[error("BIP-340 error")]
    Bip340(Bip340Error),
    #[error("SchnorrProof failed to verify")]
    SchnorrProofFailed,
    #[error("Operation timed out")]
    Timeout,
    #[error("{0}")]
    ConfigError(#[from] ConfigError),
    #[error("Received invalid signer message.")]
    InvalidSignerMessage,
}

#[derive(clap::Subcommand, Debug)]
pub enum Command {
    Dkg,
    Sign { msg: Vec<u8> },
    DkgSign { msg: Vec<u8> },
    GetAggregatePublicKey,
    CreateScripts,
    SpendScripts, // TODO degens: has to sign the msg as in Sign and DkgSign
}

pub struct Coordinator<Network: NetListen> {
    id: u32, // Used for relay coordination
    current_dkg_id: u64,
    current_dkg_public_id: u64,
    current_sign_id: u64,
    current_sign_nonce_id: u64,
    total_signers: u32, // Assuming the signers cover all id:s in {1, 2, ..., total_signers}
    total_keys: u32,
    threshold: u32,
    network: Network,
    dkg_public_shares: BTreeMap<u32, DkgPublicShare>,
    public_nonces: BTreeMap<u32, NonceResponse>,
    signature_shares: BTreeMap<u32, Vec<SignatureShare>>,
    aggregate_public_key: Point,
    network_private_key: Scalar,
    public_key: PublicKey,
    // from user public key to script bitcoin address
    script_addresses: BTreeMap<PublicKey, BitcoinAddress>,
}

impl<Network: NetListen> Coordinator<Network> {
    pub fn new(id: u32, config: &Config, network: Network) -> Result<Self, Error> {
        Ok(Self {
            id,
            current_dkg_id: 0,
            current_dkg_public_id: 0,
            current_sign_id: 1,
            current_sign_nonce_id: 1,
            total_signers: config.total_signers,
            total_keys: config.total_keys,
            threshold: config.keys_threshold,
            network,
            dkg_public_shares: Default::default(),
            public_nonces: Default::default(),
            aggregate_public_key: Point::default(),
            signature_shares: Default::default(),
            network_private_key: config.network_private_key,
            public_key: config.coordinator_public_key,
            script_addresses: Default::default(),
        })
    }

    pub fn get_aggregate_public_key(&self) -> Result<Point, Error> {
        if self.aggregate_public_key == Point::default() {
            Err(Error::NoAggregatePublicKey)
        } else {
            Ok(self.aggregate_public_key)
        }
    }

    pub fn set_aggregate_public_key(&mut self, public_key: Point) {
        self.aggregate_public_key = public_key;
    }

    pub fn get_dkg_public_shares(&self) -> &BTreeMap<u32, DkgPublicShare> {
        &self.dkg_public_shares
    }

    pub fn set_dkg_public_shares(&mut self, dkg_public_shares: BTreeMap<u32, DkgPublicShare>) {
        self.dkg_public_shares = dkg_public_shares;
    }
}

impl<Network: NetListen> Coordinator<Network>
where
    Error: From<Network::Error>,
{
    pub fn run(&mut self, command: &Command) -> Result<(), Error> {
        match command {
            Command::Dkg => {
                self.run_distributed_key_generation()?;
                Ok(())
            }
            Command::Sign { msg } => {
                self.sign_message(msg)?;
                Ok(())
            }
            Command::DkgSign { msg } => {
                info!("sign msg: {:?}", msg);
                self.run_distributed_key_generation()?;
                self.sign_message(msg)?;
                Ok(())
            }
            Command::GetAggregatePublicKey => {
                let key = self.get_aggregate_public_key()?;
                info!("aggregate public key {}", key);
                Ok(())
            }
            Command::CreateScripts => {
                info!("create scripts and fund them by signers");
                self.run_create_scripts_generation()?;
                Ok(())
            }
            Command::SpendScripts => {
                info!("spend scripts");
                // self.
                Ok(())
            }
        }
    }

    pub fn run_distributed_key_generation(&mut self) -> Result<Point, Error> {
        self.current_dkg_id = self.current_dkg_id.wrapping_add(1);
        info!("Starting DKG round #{}", self.current_dkg_id);
        self.start_public_shares()?;
        let public_key = self.wait_for_public_shares()?;
        self.start_private_shares()?;
        self.wait_for_dkg_end()?;
        Ok(public_key)
    }

    fn start_scripts(&mut self) -> Result<(), Error> {
        let create_script = DegensScriptRequest {
            dkg_id: self.current_dkg_id,
        };
        let create_scripts_message = Message {
            sig: create_script.sign(&self.network_private_key).expect(""),
            msg: MessageTypes::DegensCreateScriptsRequest(create_script),
        };
        self.network.send_message(create_scripts_message)?;
        Ok(())
    }

    fn wait_for_create_scripts(&mut self) -> Result<Vec<Result<UTXO, UtxoError>>, Error> {
        let mut ids_to_await: HashSet<u32> = (1..=self.total_signers).collect();
        let mut utxos = vec![];

        while !ids_to_await.is_empty() {
            if let MessageTypes::DegensCreateScriptsResponse(response) = self.wait_for_next_message()?.msg {
                ids_to_await.remove(&response.signer_id);
                utxos.push(response.utxo);
            }
        }
        Ok(utxos)
    }

    pub fn run_create_scripts_generation(&mut self) -> Result<Vec<Result<UTXO, UtxoError>>, Error> {
        // things to be done by coordinator
        info!("Starting to create scripts");
        self.start_scripts().unwrap();

        let utxos = self.wait_for_create_scripts().unwrap();

        Ok(utxos)
    }

    fn start_public_shares(&mut self) -> Result<(), Error> {
        self.dkg_public_shares.clear();
        info!(
            "DKG Round #{}: Starting Public Share Distribution Round #{}",
            self.current_dkg_id, self.current_dkg_public_id
        );
        let dkg_begin = DkgBegin {
            dkg_id: self.current_dkg_id,
        };

        let dkg_begin_message = Message {
            sig: dkg_begin.sign(&self.network_private_key).expect(""),
            msg: MessageTypes::DkgBegin(dkg_begin),
        };
        self.network.send_message(dkg_begin_message)?;
        Ok(())
    }

    fn start_private_shares(&mut self) -> Result<(), Error> {
        info!(
            "DKG Round #{}: Starting Private Share Distribution",
            self.current_dkg_id
        );
        let dkg_begin = DkgBegin {
            dkg_id: self.current_dkg_id,
        };
        let dkg_private_begin_msg = Message {
            sig: dkg_begin.sign(&self.network_private_key).expect(""),
            msg: MessageTypes::DkgPrivateBegin(dkg_begin),
        };
        self.network.send_message(dkg_private_begin_msg)?;
        Ok(())
    }

    fn collect_nonces(&mut self) -> Result<(), Error> {
        self.public_nonces.clear();

        let nonce_request = NonceRequest {
            dkg_id: self.current_dkg_id,
            sign_id: self.current_sign_id,
            sign_nonce_id: self.current_sign_nonce_id,
        };

        let nonce_request_message = Message {
            sig: nonce_request
                .sign(&self.network_private_key)
                .expect("Failed to sign NonceRequest"),
            msg: MessageTypes::NonceRequest(nonce_request),
        };

        debug!(
            "dkg_id #{} sign_id #{} sign_nonce_id #{}. NonceRequest sent",
            self.current_dkg_id, self.current_sign_id, self.current_sign_nonce_id
        );
        self.network.send_message(nonce_request_message)?;

        loop {
            match self.wait_for_next_message()?.msg {
                MessageTypes::NonceRequest(_) => {}
                MessageTypes::NonceResponse(nonce_response) => {
                    let signer_id = nonce_response.signer_id;
                    self.public_nonces.insert(signer_id, nonce_response);
                    debug!(
                        "NonceResponse from signer #{:?}. Got {} nonce responses of threshold {}",
                        signer_id,
                        self.public_nonces.len(),
                        self.threshold,
                    );
                }
                msg => {
                    warn!("NonceLoop Got unexpected message {:?})", msg.type_id());
                }
            }

            if self.public_nonces.len() == usize::try_from(self.total_signers).unwrap() {
                debug!("Nonce threshold of {} met.", self.threshold);
                break;
            }
        }
        Ok(())
    }

    #[allow(non_snake_case)]
    fn compute_aggregate_nonce(&mut self, msg: &[u8]) -> Result<Point, Error> {
        info!("Computing aggregate nonce...");
        self.collect_nonces()?;
        // XXX this needs to be key_ids for v1 and signer_ids for v2
        let party_ids = self
            .public_nonces
            .values()
            .flat_map(|pn| pn.key_ids.clone())
            .collect::<Vec<u32>>();
        let nonces = self
            .public_nonces
            .values()
            .flat_map(|pn| pn.nonces.clone())
            .collect::<Vec<PublicNonce>>();
        let (_, R) = compute::intermediate(msg, &party_ids, &nonces);
        Ok(R)
    }

    fn request_signature_shares(
        &self,
        nonce_responses: &[NonceResponse],
        msg: &[u8],
    ) -> Result<(), Error> {
        let signature_share_request = SignatureShareRequest {
            dkg_id: self.current_dkg_id,
            sign_id: self.current_sign_id,
            correlation_id: 0,
            nonce_responses: nonce_responses.to_vec(),
            message: msg.to_vec(),
        };

        info!(
            "Sending SignShareRequest dkg_id #{} sign_id #{} to signers",
            signature_share_request.dkg_id, signature_share_request.sign_id
        );

        let signature_share_request_message = Message {
            sig: signature_share_request
                .sign(&self.network_private_key)
                .expect("Failed to sign SignShareRequest"),
            msg: MessageTypes::SignShareRequest(signature_share_request),
        };

        self.network.send_message(signature_share_request_message)?;

        Ok(())
    }

    fn collect_signature_shares(&mut self) -> Result<(), Error> {
        // get the parties who responded with a nonce
        let mut signers: HashSet<u32> = HashSet::from_iter(self.public_nonces.keys().cloned());
        while !signers.is_empty() {
            match self.wait_for_next_message()?.msg {
                MessageTypes::SignShareResponse(response) => {
                    if let Some(_party_id) = signers.take(&response.signer_id) {
                        info!(
                            "Insert signature shares for signer_id {}",
                            &response.signer_id
                        );
                        self.signature_shares
                            .insert(response.signer_id, response.signature_shares.clone());
                    }
                    debug!(
                        "signature shares for {} received.  left to receive: {:?}",
                        response.signer_id, signers
                    );
                }
                MessageTypes::SignShareRequest(_) => {}
                msg => {
                    warn!("SigShare loop got unexpected msg {:?}", msg.type_id());
                }
            }
        }
        Ok(())
    }

    #[allow(non_snake_case)]
    pub fn sign_message(&mut self, msg: &[u8]) -> Result<(Signature, SchnorrProof), Error> {
        debug!("Attempting to Sign Message");
        if self.aggregate_public_key == Point::default() {
            return Err(Error::NoAggregatePublicKey);
        }

        //Continually compute a new aggregate nonce until we have a valid even R
        loop {
            let R = self.compute_aggregate_nonce(msg)?;
            if R.has_even_y() {
                debug!("Success: R has even y coord: {}", &R);
                break;
            } else {
                warn!("Failure: R does not have even y coord: {}", R);
            }
        }

        // make an array of dkg public share polys for SignatureAggregator
        debug!(
            "collecting commitments from 1..{} in {:?}",
            self.total_keys,
            self.dkg_public_shares.keys().collect::<Vec<&u32>>()
        );
        let polys: Vec<PolyCommitment> = self
            .dkg_public_shares
            .values()
            .map(|ps| ps.public_share.clone())
            .collect();

        debug!(
            "SignatureAggregator::new total_keys: {} threshold: {} commitments: {}",
            self.total_keys,
            self.threshold,
            polys.len()
        );

        let mut aggregator = v1::SignatureAggregator::new(self.total_keys, self.threshold, polys)?;

        let nonce_responses: Vec<NonceResponse> = self.public_nonces.values().cloned().collect();

        // request signature shares
        self.request_signature_shares(&nonce_responses, msg)?;
        self.collect_signature_shares()?;

        let nonces = nonce_responses
            .iter()
            .flat_map(|nr| nr.nonces.clone())
            .collect::<Vec<PublicNonce>>();
        let shares = &self
            .public_nonces
            .iter()
            .flat_map(|(i, _)| self.signature_shares[i].clone())
            .collect::<Vec<SignatureShare>>();

        info!(
            "aggregator.sign({:?}, {:?}, {:?})",
            msg,
            nonces.len(),
            shares.len()
        );

        let sig = aggregator.sign(msg, &nonces, shares)?;

        info!("Signature ({}, {})", sig.R, sig.z);

        let proof = SchnorrProof::new(&sig).map_err(Error::Bip340)?;

        info!("SchnorrProof ({}, {})", proof.r, proof.s);

        if !proof.verify(&self.aggregate_public_key.x(), msg) {
            warn!("SchnorrProof failed to verify!");
            return Err(Error::SchnorrProofFailed);
        }

        Ok((sig, proof))
    }

    fn calculate_aggregate_public_key(&mut self) -> Result<Point, Error> {
        self.aggregate_public_key = self
            .dkg_public_shares
            .iter()
            .fold(Point::default(), |s, (_, dps)| s + dps.public_share.A[0]);
        Ok(self.aggregate_public_key)
    }

    fn wait_for_public_shares(&mut self) -> Result<Point, Error> {
        let mut ids_to_await: HashSet<u32> = (1..=self.total_signers).collect();

        info!(
            "DKG Round #{}: waiting for Dkg Public Shares from signers {:?}",
            self.current_dkg_id, ids_to_await
        );

        loop {
            if ids_to_await.is_empty() {
                let key = self.calculate_aggregate_public_key()?;
                // check to see if aggregate public key has even y
                if key.has_even_y() {
                    debug!("Aggregate public key has even y coord!");
                    info!("Aggregate public key: {}", key);
                    self.aggregate_public_key = key;
                    return Ok(key);
                } else {
                    warn!("DKG Round #{} Failed: Aggregate public key does not have even y coord, re-running dkg.", self.current_dkg_id);
                    ids_to_await = (1..=self.total_signers).collect();
                    self.start_public_shares()?;
                }
            }

            match self.wait_for_next_message()?.msg {
                MessageTypes::DkgPublicEnd(dkg_end_msg) => {
                    ids_to_await.remove(&dkg_end_msg.signer_id);
                    debug!(
                        "DKG_Public_End round #{} from signer #{}. Waiting on {:?}",
                        dkg_end_msg.dkg_id, dkg_end_msg.signer_id, ids_to_await
                    );
                }
                MessageTypes::DkgPublicShare(dkg_public_share) => {
                    self.dkg_public_shares
                        .insert(dkg_public_share.party_id, dkg_public_share.clone());

                    debug!(
                        "DKG round #{} DkgPublicShare from party #{}",
                        dkg_public_share.dkg_id, dkg_public_share.party_id
                    );
                }
                _ => {}
            }
        }
    }

    fn wait_for_dkg_end(&mut self) -> Result<(), Error> {
        let mut ids_to_await: HashSet<u32> = (1..=self.total_signers).collect();
        info!(
            "DKG Round #{}: waiting for Dkg End from signers {:?}",
            self.current_dkg_id, ids_to_await
        );
        while !ids_to_await.is_empty() {
            if let MessageTypes::DkgEnd(dkg_end_msg) = self.wait_for_next_message()?.msg {
                ids_to_await.remove(&dkg_end_msg.signer_id);
                debug!(
                    "DKG_End round #{} from signer #{}. Waiting on {:?}",
                    dkg_end_msg.dkg_id, dkg_end_msg.signer_id, ids_to_await
                );
            }
        }
        Ok(())
    }

    fn wait_for_next_message(&mut self) -> Result<Message, Error> {
        let get_next_message = || {
            self.network.poll(self.id);
            // We only ever receive already verified messages. No need to check result.
            self.network
                .next_message()
                .ok_or_else(|| "No message yet".to_owned())
                .map_err(backoff::Error::transient)
        };

        let notify = |_err, dur| {
            debug!("No message. Next poll in {:?}", dur);
        };

        let backoff_timer = backoff::ExponentialBackoffBuilder::new()
            .with_initial_interval(Duration::from_millis(2))
            .with_max_interval(Duration::from_millis(128))
            .build();
        backoff::retry_notify(backoff_timer, get_next_message, notify).map_err(|_| Error::Timeout)
    }

    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::DEVNET_COORDINATOR_ID;
    use bitcoin::secp256k1::{Secp256k1, SecretKey};

    use degen_base_signer::{
        config::{Config, PublicKeys, SignerKeyIds},
        net::{HttpNet, HttpNetListen},
        signer::Signer,
    };

    use hashbrown::HashMap;
    use p256k1::{ecdsa, scalar::Scalar};
    use rand::rngs::StdRng;
    use rand_core::{OsRng, RngCore, SeedableRng};
    use relay_server::Server as RelayServer;
    use std::{env, thread};
    use std::str::FromStr;
    use bitcoin::{KeyPair, Network, PrivateKey, XOnlyPublicKey};
    use blockstack_lib::address::AddressHashMode;
    use test_utils::parse_env;

    use blockstack_lib::chainstate::stacks::{StacksPrivateKey, TransactionVersion};
    use blockstack_lib::types::chainstate::{StacksAddress, StacksPublicKey};
    use serde::Deserialize;
    use degen_base_signer::util_versioning::address_version;
    use url::Url;

    fn create_signer_key_ids(signer_id: u32, keys_per_signer: u32) -> Vec<u32> {
        (0..keys_per_signer)
            .map(|i| keys_per_signer * signer_id + i + 1)
            .collect()
    }

    fn create_public_keys(signer_private_keys: &Vec<Scalar>, keys_per_signer: u32) -> PublicKeys {
        let signer_id_keys = signer_private_keys
            .iter()
            .enumerate()
            .map(|(i, key)| ((i + 1) as u32, ecdsa::PublicKey::new(key).unwrap()))
            .collect::<HashMap<u32, ecdsa::PublicKey>>();

        let key_ids = signer_id_keys
            .iter()
            .flat_map(|(signer_id, signer_key)| {
                (0..keys_per_signer).map(|i| (keys_per_signer * *signer_id - i, signer_key.clone()))
            })
            .collect::<HashMap<u32, ecdsa::PublicKey>>();

        PublicKeys {
            signers: signer_id_keys,
            key_ids,
        }
    }

    #[test]
    fn integration_test_frost_coordinator_should_be_able_to_successfully_run_dkg_sign() {
        let relay_url = "http://127.0.0.1:9776".to_string();
        let (coordinator_config, coordinator_net_listen) =
            spawn_processes_and_get_config(relay_url);

        let mut coordinator = Coordinator::new(
            DEVNET_COORDINATOR_ID,
            &coordinator_config,
            coordinator_net_listen,
        )
        .unwrap();

        coordinator
            .run(&Command::DkgSign {
                msg: vec![0, 1, 2, 3],
            })
            .unwrap();
    }

    #[test]
    fn integration_test_frost_coordinator_should_provide_valid_signatures_after_dkg() {
        let msg = vec![1, 3, 3, 7];
        let relay_url = "http://127.0.0.1:9777".to_string();
        let (coordinator_config, coordinator_net_listen) =
            spawn_processes_and_get_config(relay_url);

        let mut coordinator = Coordinator::new(
            DEVNET_COORDINATOR_ID,
            &coordinator_config,
            coordinator_net_listen,
        )
        .unwrap();

        let public_key = coordinator.run_distributed_key_generation().unwrap();
        let (_, schnorr_proof) = coordinator.sign_message(&msg).unwrap();

        schnorr_proof.verify(&public_key.x(), &msg);
    }

    #[test]
    fn integration_test_frost_coordinator_should_provide_valid_signatures_after_restart() {
        let msg = vec![1, 3, 3, 7];
        let relay_url = "http://127.0.0.1:9778".to_string();
        let (coordinator_config, coordinator_net_listen) =
            spawn_processes_and_get_config(relay_url);

        let mut coordinator = Coordinator::new(
            DEVNET_COORDINATOR_ID,
            &coordinator_config,
            coordinator_net_listen.clone(),
        )
        .unwrap();

        let public_key = coordinator.run_distributed_key_generation().unwrap();
        let dkg_public_shares = coordinator.get_dkg_public_shares().clone();

        let mut coordinator = Coordinator::new(
            DEVNET_COORDINATOR_ID,
            &coordinator_config,
            coordinator_net_listen,
        )
        .unwrap();

        coordinator.set_aggregate_public_key(public_key);
        coordinator.set_dkg_public_shares(dkg_public_shares);

        let (_, schnorr_proof) = coordinator.sign_message(&msg).unwrap();

        schnorr_proof.verify(&public_key.x(), &msg);
    }

    #[derive(Clone, Deserialize, Default, Debug)]
    #[serde(rename_all = "lowercase")]
    pub enum NetworkVersion {
        Mainnet,
        Testnet,
        #[default]
        Regtest,
    }

    fn parse_version(network: NetworkVersion) -> (TransactionVersion, bitcoin::Network) {
        // Determine what network we are running on
        match network {
            NetworkVersion::Mainnet => (TransactionVersion::Mainnet, Network::Bitcoin),
            NetworkVersion::Testnet => (TransactionVersion::Testnet, Network::Testnet),
            NetworkVersion::Regtest => (TransactionVersion::Testnet, Network::Regtest),
        }
    }

    fn parse_stacks_private_key(stacks_private_key: String, network: NetworkVersion) -> Result<(StacksPrivateKey, StacksAddress), Error> {
        let sender_key = StacksPrivateKey::from_hex(&stacks_private_key).unwrap();
        let pk = StacksPublicKey::from_private(&sender_key);

        let address = StacksAddress::from_public_keys(
            address_version(&parse_version(network).0),
            &AddressHashMode::SerializeP2PKH,
            1,
            &vec![pk],
        ).unwrap();

        Ok((sender_key, address))
    }

    fn parse_bitcoin_private_key(bitcoin_private_key: String) -> Result<(SecretKey, XOnlyPublicKey), Error> {
        let secp = Secp256k1::new();
        let sender_key = SecretKey::from_str(&bitcoin_private_key).unwrap();
        let key_pair_source = KeyPair::from_secret_key(&secp, &sender_key);
        let (xonly_public_key, _) = key_pair_source.x_only_public_key();

        Ok((sender_key, xonly_public_key))
    }

    fn spawn_processes_and_get_config(relay_url: String) -> (Config, HttpNetListen) {
        env::set_var("RUST_LOG", "info");

        let num_signers = parse_env::<u32>("num_signers", 6);
        let keys_per_signer = parse_env::<u32>("keys_per_signer", 3);
        let keys_threshold = parse_env::<u32>("keys_threshold", 15);
        let mut osrng = OsRng;
        let seed = osrng.next_u64();

        println!("seed: {}", seed);

        let mut rng = StdRng::seed_from_u64(seed);
        let coordinator_private_key = Scalar::random(&mut rng);
        let coordinator_public_key = ecdsa::PublicKey::new(&coordinator_private_key).unwrap();
        let signer_private_keys = (0..num_signers)
            .map(|_| Scalar::random(&mut rng))
            .collect::<Vec<Scalar>>();
        let signer_key_ids = (0..num_signers)
            .map(|i| (i + 1, create_signer_key_ids(i, keys_per_signer)))
            .collect::<SignerKeyIds>();
        let public_keys = create_public_keys(&signer_private_keys, keys_per_signer);
        let stacks_private_key_str =  "7287ba251d44a4d3fd9276c88ce34c5c52a038955511cccaf77e61068649c17801";
        let (stacks_private_key, stacks_address) = parse_stacks_private_key(stacks_private_key_str.to_string(), NetworkVersion::Regtest)
            .unwrap();
        let stacks_node_rpc_url = Url::try_from("http://localhost:20443").unwrap();
        let stacks_version = TransactionVersion::Testnet;
        let bitcoin_private_key_str = "2bd806c97f0e00af1a1fc3328fa763a9269723c8db8fac4f93af71db186d6e90";
        let (bitcoin_private_key, bitcoin_xonly_pubkey) = parse_bitcoin_private_key(bitcoin_private_key_str.to_string()).unwrap();
        let bitcoin_node_rpc_url = Url::try_from("http://devnet:devnet@localhost:18443").unwrap();
        let transaction_fee = 2000;
        let bitcoin_network = Network::Regtest;

        let coordinator_config = Config::new(
            stacks_private_key,
            stacks_address,
            stacks_node_rpc_url.clone(),
            stacks_version,
            bitcoin_private_key,
            bitcoin_xonly_pubkey,
            bitcoin_node_rpc_url.clone(),
            transaction_fee,
            bitcoin_network,
            keys_threshold,
            coordinator_public_key,
            public_keys.clone(),
            signer_key_ids.clone(),
            coordinator_private_key,
            relay_url.clone(),
        );


        let signer_configs = signer_private_keys
            .iter()
            .map(|k| {
                let (stacks_private_key, stacks_address) = parse_stacks_private_key(
                    StacksPrivateKey::new().to_hex(),
                    NetworkVersion::Regtest
                    ).unwrap();
                Config::new(
                    stacks_private_key,
                    stacks_address,
                    stacks_node_rpc_url.clone(),
                    stacks_version,
                    bitcoin_private_key,
                    bitcoin_xonly_pubkey,
                    bitcoin_node_rpc_url.clone(),
                    transaction_fee,
                    bitcoin_network,
                    keys_threshold,
                    coordinator_public_key,
                    public_keys.clone(),
                    signer_key_ids.clone(),
                    k.clone(),
                    relay_url.clone(),
                )
            })
            .collect::<Vec<Config>>();

        let net: HttpNet = HttpNet::new(relay_url.clone());
        let coordinator_net_listen: HttpNetListen = HttpNetListen::new(net.clone(), vec![]);

        thread::spawn(move || {
            let relay_socket_address = relay_url.strip_prefix("http://").unwrap();
            RelayServer::run(relay_socket_address)
        });

        for i in 0..num_signers {
            let config = signer_configs[i as usize].clone();
            thread::spawn(move || {
                let mut signer = Signer::new(config, i + 1);
                signer.start_p2p_sync().unwrap();
            });
        }

        (coordinator_config, coordinator_net_listen)
    }
}
