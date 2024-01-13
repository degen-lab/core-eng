use std::time::{Duration, Instant};

use crate::stacks_node::{Error as StacksNodeError, PegInOp, PegOutRequestOp, StacksNode};
use bitcoin::XOnlyPublicKey;
use stackslib::{
    chainstate::stacks::StacksTransaction,
    codec::StacksMessageCodec,
    types::chainstate::StacksAddress,
    vm::{types::SequenceData, ClarityName, ContractName, Value as ClarityValue},
};
use stackslib::util::hash::Hash160;
use stackslib::vm::types::{PrincipalData, StandardPrincipalData};
use crate::config::{MinerStatus, PublicKeys, SignerKeyIds};
use reqwest::{
    blocking::{Client, Response},
    StatusCode,
};
use serde_json::{json, Value};
use tracing::{debug, info};
use url::Url;
use wsts::ecdsa::PublicKey;

/// Kinds of stacks node broadcast errors that can occur
#[derive(Debug, thiserror::Error)]
pub enum BroadcastError {
    #[error("Fee too low. Expected: {0}, Actual: {1}")]
    FeeTooLow(u64, u64),
    #[error("Not enough funds: {0}")]
    NotEnoughFunds(String),
    #[error("Conflicting nonce in mempool")]
    ConflictingNonceInMempool,
    #[error("{0}")]
    Other(String),
}

impl From<&serde_json::Value> for BroadcastError {
    fn from(value: &serde_json::Value) -> Self {
        let reason = value
            .get("reason")
            .and_then(|reason| reason.as_str())
            .unwrap_or("Unknown Reason");
        let reason_data = value.get("reason_data");
        match reason {
            "FeeTooLow" => {
                let expected = value
                    .get("expected")
                    .and_then(|expected| expected.as_u64())
                    .unwrap_or(0);
                let actual = value
                    .get("actual")
                    .and_then(|actual| actual.as_u64())
                    .unwrap_or(0);
                BroadcastError::FeeTooLow(expected, actual)
            }
            "NotEnoughFunds" => BroadcastError::NotEnoughFunds(
                reason_data.unwrap_or(&json!("No Reason Data")).to_string(),
            ),
            "ConflictingNonceInMempool" => BroadcastError::ConflictingNonceInMempool,
            _ => BroadcastError::Other(reason.to_string()),
        }
    }
}

#[derive(Clone, Debug)]
pub struct NodeClient {
    node_url: Url,
    client: Client,
    contract_name: ContractName,
    contract_address: StacksAddress,
    next_nonce: Option<u64>,
}

impl NodeClient {
    pub fn new(
        node_url: Url,
        contract_name: ContractName,
        contract_address: StacksAddress,
    ) -> Self {
        Self {
            node_url,
            client: Client::new(),
            contract_name,
            contract_address,
            next_nonce: None,
        }
    }

    fn build_url(&self, route: &str) -> Result<Url, StacksNodeError> {
        Ok(self.node_url.join(route)?)
    }

    fn get_response(&self, route: &str) -> Result<Response, StacksNodeError> {
        let url = self.build_url(route)?;
        debug!("Sending Request to Stacks Node: {}", &url);
        let now = Instant::now();
        let notify = |_err, dur| {
            debug!("Failed to connect to {}. Next attempt in {:?}", &url, dur);
        };

        let send_request = || {
            if now.elapsed().as_secs() > 5 {
                debug!("Timeout exceeded.");
                return Err(backoff::Error::Permanent(StacksNodeError::Timeout));
            }
            let request = self.client.get(url.as_str());
            let response = request.send().map_err(StacksNodeError::ReqwestError)?;
            Ok(response)
        };
        let backoff_timer = backoff::ExponentialBackoffBuilder::new()
            .with_initial_interval(Duration::from_millis(2))
            .with_max_interval(Duration::from_millis(128))
            .build();

        let response = backoff::retry_notify(backoff_timer, send_request, notify)
            .map_err(|_| StacksNodeError::Timeout)?;

        Ok(response)
    }

    fn get_burn_ops<T>(&self, block_height: u64, op: &str) -> Result<Vec<T>, StacksNodeError>
        where
            T: serde::de::DeserializeOwned,
    {
        let json = self
            .get_response(&format!("/v2/burn_ops/{block_height}/{op}"))?
            .json::<Value>()
            .map_err(|_| StacksNodeError::UnknownBlockHeight(block_height))?;
        Ok(serde_json::from_value(json[op].clone())?)
    }

    fn num_signers(&self, sender: &StacksAddress) -> Result<u128, StacksNodeError> {
        let function_name = "get-num-signers";
        let total_signers_hex = self.call_read(sender, function_name, &[])?;
        let total_signers = ClarityValue::try_deserialize_hex_untyped(&total_signers_hex)?;
        if let ClarityValue::UInt(total_signers) = total_signers {
            Ok(total_signers)
        } else {
            Err(StacksNodeError::MalformedClarityValue(
                function_name.to_string(),
                total_signers,
            ))
        }
    }

    fn signer_data(
        &self,
        sender: &StacksAddress,
        id: u128,
        public_keys: &mut PublicKeys,
        signer_key_ids: &mut SignerKeyIds,
    ) -> Result<(), StacksNodeError> {
        let function_name = "get-signer-data";
        let signer_data_hex = self.call_read(
            sender,
            function_name,
            &[&ClarityValue::UInt(id).to_string()],
        )?;
        let signer_data = ClarityValue::try_deserialize_hex_untyped(&signer_data_hex)?;
        if let ClarityValue::Optional(optional_data) = signer_data.clone() {
            if let Some(ClarityValue::Tuple(tuple_data)) = optional_data.data.map(|boxed| *boxed) {
                let public_key =
                    if let Some(ClarityValue::Sequence(SequenceData::Buffer(public_key))) =
                        tuple_data.data_map.get(&ClarityName::from("public-key"))
                    {
                        PublicKey::try_from(public_key.data.as_slice()).map_err(|_| {
                            StacksNodeError::MalformedClarityValue(
                                function_name.to_string(),
                                signer_data.clone(),
                            )
                        })?
                    } else {
                        return Err(StacksNodeError::MalformedClarityValue(
                            function_name.to_string(),
                            signer_data,
                        ));
                    };
                public_keys
                    .signers
                    .insert(id.try_into().unwrap(), public_key);
                if let Some(ClarityValue::Sequence(SequenceData::List(keys_ids))) =
                    tuple_data.data_map.get(&ClarityName::from("key-ids"))
                {
                    let mut this_signer_key_ids = Vec::new();
                    for key_id in &keys_ids.data {
                        if let ClarityValue::UInt(key_id) = key_id {
                            public_keys
                                .key_ids
                                .insert((*key_id).try_into().unwrap(), public_key);
                            this_signer_key_ids.push((*key_id).try_into().unwrap());
                        } else {
                            return Err(StacksNodeError::MalformedClarityValue(
                                function_name.to_string(),
                                signer_data,
                            ));
                        }
                    }
                    signer_key_ids.insert(id.try_into().unwrap(), this_signer_key_ids);
                }
            } else {
                return Err(StacksNodeError::NoSignerData(id));
            }
        }
        Err(StacksNodeError::MalformedClarityValue(
            function_name.to_string(),
            signer_data,
        ))
    }

    fn call_read(
        &self,
        sender: &StacksAddress,
        function_name: &str,
        function_args: &[&str],
    ) -> Result<String, StacksNodeError> {
        debug!("Calling read-only function {}...", function_name);
        let body = json!({"sender": sender.to_string(), "arguments": function_args}).to_string();
        let url = self.build_url(&format!(
            "/v2/contracts/call-read/{}/{}/{function_name}",
            self.contract_address,
            self.contract_name.as_str()
        ))?;

        let response = self
            .client
            .post(url)
            .header("content-type", "application/json")
            .body(body)
            .send()?
            .json::<serde_json::Value>()?;

        debug!("response: {:?}", response);
        if !response
            .get("okay")
            .map(|val| val.as_bool().unwrap_or(false))
            .unwrap_or(false)
        {
            let cause = response
                .get("cause")
                .ok_or(StacksNodeError::InvalidJsonEntry("cause".to_string()))?;
            return Err(StacksNodeError::ReadOnlyFailure(format!(
                "{}: {}",
                function_name, cause
            )));
        }
        let result = response
            .get("result")
            .ok_or(StacksNodeError::InvalidJsonEntry("result".to_string()))?
            .as_str()
            .ok_or_else(|| StacksNodeError::ReadOnlyFailure("Expected string result.".to_string()))?
            .to_string();
        Ok(result)
    }
}

impl StacksNode for NodeClient {
    fn get_peg_in_ops(&self, block_height: u64) -> Result<Vec<PegInOp>, StacksNodeError> {
        debug!("Retrieving peg-in ops...");
        self.get_burn_ops::<PegInOp>(block_height, "peg_in")
    }

    fn get_peg_out_request_ops(
        &self,
        block_height: u64,
    ) -> Result<Vec<PegOutRequestOp>, StacksNodeError> {
        debug!("Retrieving peg-out request ops...");
        self.get_burn_ops::<PegOutRequestOp>(block_height, "peg_out_request")
    }

    fn burn_block_height(&self) -> Result<u64, StacksNodeError> {
        debug!("Retrieving burn block height...");
        let json = self.get_response("/v2/info")?.json::<Value>()?;
        let entry = "burn_block_height";
        json[entry]
            .as_u64()
            .ok_or_else(|| StacksNodeError::InvalidJsonEntry(entry.to_string()))
    }

    fn next_nonce(&mut self, address: &StacksAddress) -> Result<u64, StacksNodeError> {
        debug!("Retrieving next nonce...");
        let address = address.to_string();
        let entry = "possible_next_nonce";
        let route = format!("/extended/v1/address/{}/nonces", address);
        let response = self.get_response(&route)?;
        if response.status() == StatusCode::NOT_FOUND {
            return Err(StacksNodeError::UnknownAddress(address));
        }
        let json = response
            .json::<Value>()
            .map_err(|_| StacksNodeError::BehindChainTip)?;
        let nonce = json
            .get(entry)
            .and_then(|nonce| nonce.as_u64())
            .ok_or_else(|| StacksNodeError::InvalidJsonEntry(entry.to_string()))?;
        self.next_nonce = Some(nonce);
        Ok(nonce)
    }

    fn get_user_balance(&mut self, address: &StacksAddress) -> Result<u64, StacksNodeError> {
        debug!("Retrieving account balance...");
        let address = address.to_string();
        let entry = "balance";
        let route = format!("/v2/accounts/{}", address);
        let response = self.get_response(&route)?;
        if response.status() == StatusCode::NOT_FOUND {
            return Err(StacksNodeError::UnknownAddress(address));
        }
        let json = response
            .json::<Value>()
            .map_err(|_| StacksNodeError::BehindChainTip)?;
        info!("{:#?}", json
            .get(entry));
        let balance = json
            .get(entry)
            .and_then(|balance| Some(u64::from_str_radix(&balance.as_str().unwrap()[2..], 16).unwrap()))
            .ok_or_else(|| StacksNodeError::InvalidJsonEntry(entry.to_string()))?;

        Ok(balance)
    }

    fn get_mempool_transactions(&mut self) -> Result<u64, StacksNodeError> {
        debug!("Retrieving mempool transactions...");
        let entry = "results";
        let route = format!("/extended/v1/tx/mempool?limit=50");
        let response = self.get_response(&route)?;
        // if response.status() == StatusCode::NOT_FOUND {
        //     return Err(StacksNodeError::UnknownAddress(address));
        // }
        let json = response
            .json::<Value>()
            .map_err(|_| StacksNodeError::BehindChainTip)?;
        let transactions = json
            .get(entry)
            .and_then(|transaction| transaction.as_array())
            .ok_or_else(|| StacksNodeError::InvalidJsonEntry(entry.to_string()))?;
        let contract_call_transactions: Vec<&Value> = transactions
            .iter()
            .filter(|tx| tx["tx_type"] == "contract_call")
            .collect();

        println!("{:#?}\n{:#?}", contract_call_transactions, contract_call_transactions.len());
        Ok(0)
    }

    fn broadcast_transaction(&self, tx: &StacksTransaction) -> Result<(), StacksNodeError> {
        debug!("Broadcasting transaction...");
        debug!("Transaction: {:?}", tx);
        let url = self.build_url("/v2/transactions")?;
        let mut buffer = vec![];

        tx.consensus_serialize(&mut buffer)?;

        let response = self
            .client
            .post(url)
            .header("content-type", "application/octet-stream")
            .body(buffer)
            .send()?;

        // TODO: degens - fix broadcast stx transaction
        if response.status() != StatusCode::OK {
            let json_response = response.json::<serde_json::Value>()?;
            return Err(StacksNodeError::from(BroadcastError::from(&json_response)));
        }

        Ok(())
    }

    fn keys_threshold(&self, sender: &StacksAddress) -> Result<u128, StacksNodeError> {
        let function_name = "get-threshold";
        let threshold_hex = self.call_read(sender, function_name, &[])?;
        let threshold = ClarityValue::try_deserialize_hex_untyped(&threshold_hex)?;
        if let ClarityValue::UInt(keys_threshold) = threshold {
            Ok(keys_threshold)
        } else {
            Err(StacksNodeError::MalformedClarityValue(
                function_name.to_string(),
                threshold,
            ))
        }
    }

    fn public_keys(&self, sender: &StacksAddress) -> Result<PublicKeys, StacksNodeError> {
        let total_signers = self.num_signers(sender)?;
        // Retrieve all the signers
        let mut public_keys = PublicKeys::default();
        let mut signer_key_ids = SignerKeyIds::default();
        for id in 1..=total_signers {
            self.signer_data(sender, id, &mut public_keys, &mut signer_key_ids)?;
        }
        Ok(public_keys)
    }

    fn signer_key_ids(&self, sender: &StacksAddress) -> Result<SignerKeyIds, StacksNodeError> {
        let total_signers = self.num_signers(sender)?;
        // Retrieve all the signers
        let mut public_keys = PublicKeys::default();
        let mut signer_key_ids = SignerKeyIds::default();
        for id in 1..=total_signers {
            self.signer_data(sender, id, &mut public_keys, &mut signer_key_ids)?;
        }
        Ok(signer_key_ids)
    }

    fn coordinator_public_key(
        &self,
        sender: &StacksAddress,
    ) -> Result<Option<PublicKey>, StacksNodeError> {
        Ok(None)
        // let function_name = "get-coordinator-data";
        // let coordinator_data_hex = self.call_read(sender, function_name, &[])?;
        // let coordinator_data = ClarityValue::try_deserialize_hex_untyped(&coordinator_data_hex)?;
        // if let ClarityValue::Optional(optional_data) = coordinator_data.clone() {
        //     if let Some(ClarityValue::Tuple(tuple_data)) = optional_data.data.map(|boxed| *boxed) {
        //         let value = tuple_data
        //             .data_map
        //             .get(&ClarityName::from("key"))
        //             .ok_or_else(|| {
        //                 StacksNodeError::MalformedClarityValue(
        //                     function_name.to_string(),
        //                     coordinator_data.clone(),
        //                 )
        //             })?;
        //         if let ClarityValue::Sequence(SequenceData::Buffer(coordinator_public_key)) = value
        //         {
        //             let public_key = PublicKey::try_from(coordinator_public_key.data.as_slice())
        //                 .map_err(|_| {
        //                     StacksNodeError::MalformedClarityValue(
        //                         function_name.to_string(),
        //                         coordinator_data,
        //                     )
        //                 })?;
        //             return Ok(Some(public_key));
        //         } else {
        //             return Err(StacksNodeError::MalformedClarityValue(
        //                 function_name.to_string(),
        //                 coordinator_data,
        //             ));
        //         }
        //     }
        //     return Ok(None);
        //     // Err(StacksNodeError::MalformedClarityValue(
        //     //     function_name.to_string(),
        //     //     coordinator_data,
        //     // ))
        // } else {
        //     Ok(None)
        // }
    }

    fn bitcoin_wallet_public_key(
        &self,
        sender: &StacksAddress,
    ) -> Result<Option<XOnlyPublicKey>, StacksNodeError> {
        Ok(None)
        // let function_name = "get-bitcoin-wallet-public-key";
        // let bitcoin_wallet_public_key_hex = self.call_read(sender, function_name, &[])?;
        // let bitcoin_wallet_public_key =
        //     ClarityValue::try_deserialize_hex_untyped(&bitcoin_wallet_public_key_hex)?;
        // if let ClarityValue::Optional(optional_data) = bitcoin_wallet_public_key.clone() {
        //     if let Some(ClarityValue::Sequence(SequenceData::Buffer(public_key))) =
        //         optional_data.data.map(|boxed| *boxed)
        //     {
        //         let xonly_pubkey = XOnlyPublicKey::from_slice(&public_key.data).map_err(|_| {
        //             StacksNodeError::MalformedClarityValue(
        //                 function_name.to_string(),
        //                 bitcoin_wallet_public_key,
        //             )
        //         })?;
        //         return Ok(Some(xonly_pubkey));
        //     } else {
        //         return Ok(None);
        //     }
        // }
        // Err(StacksNodeError::MalformedClarityValue(
        //     function_name.to_string(),
        //     bitcoin_wallet_public_key,
        // ))
    }

    fn get_status(&self, sender: &StacksAddress) -> Result<MinerStatus, StacksNodeError> {
        let function_name = "get-address-status";

        let data_hex = self.call_read(sender, function_name, &[("0x".to_owned() + &(hex::encode(ClarityValue::Principal(PrincipalData::from(*sender)).serialize_to_vec()))).as_str()])?;
        // response and string
        // match string based on message
        let data = ClarityValue::try_deserialize_hex_untyped(&data_hex)?;
        if let ClarityValue::Response(optional_data) = data.clone() {
            let display_value: String = optional_data.data.clone().expect_ascii();
            // if let Ok(ClarityValue::Sequence(SequenceData::String(local_status))) = optional_data.data.expect_ascii() {
            match display_value.as_str() {
                "is-miner" => Ok(MinerStatus::Miner),
                "is-pending" => Ok(MinerStatus::Pending),
                "is-waiting" =>  Ok(MinerStatus::Waiting),
                "is-none" => Ok(MinerStatus::NormalUser),
                _ =>  Err(StacksNodeError::MalformedClarityValue(
                    function_name.to_string(),
                    data,
                ))
            }
        } else {
            Err(StacksNodeError::MalformedClarityValue(
                function_name.to_string(),
                data,
            ))
        }
    }

    fn get_warn_number_user(
        &self,
        sender: &StacksAddress,
        warned_address: &StacksAddress,
    ) -> Result<u128, StacksNodeError> {
        let function_name = "get-warnings-user";

        let total_warnings_hex = self.call_read(sender, function_name, &[("0x".to_owned() + &(hex::encode(ClarityValue::Principal(PrincipalData::from(*warned_address)).serialize_to_vec()))).as_str()])?;
        let total_warnings =  ClarityValue::try_deserialize_hex_untyped(&total_warnings_hex)?;
        if let ClarityValue::UInt(total_signers) = total_warnings {
            Ok(total_signers)
        } else {
            Err(StacksNodeError::MalformedClarityValue(
                function_name.to_string(),
                total_warnings,
            ))
        }
    }

    fn get_notifier(&self, sender: &StacksAddress) -> Result<PrincipalData, StacksNodeError> {
        let function_name = "get-notifier";
        let notifier_hex = self.call_read(sender, function_name, &[])?;
        let notifier =  ClarityValue::try_deserialize_hex_untyped(&notifier_hex)?;

        if let ClarityValue::Principal(notifier) = notifier {
            Ok(notifier)
        } else {
            Err(StacksNodeError::MalformedClarityValue(
                function_name.to_string(),
                notifier,
            ))
        }
    }

    fn is_blacklisted(
        &self,
        sender: &StacksAddress,
        address: &StacksAddress
    ) -> Result<bool, StacksNodeError> {
        let function_name = "is-blacklisted";

        let is_blacklisted_hex = self.call_read(sender, function_name, &[("0x".to_owned() + &(hex::encode(ClarityValue::Principal(PrincipalData::from(*address)).serialize_to_vec()))).as_str()])?;
        let is_blacklisted =  ClarityValue::try_deserialize_hex_untyped(&is_blacklisted_hex)?;
        if let ClarityValue::Bool(is_blacklisted) = is_blacklisted {
            Ok(is_blacklisted)
        } else {
            Err(StacksNodeError::MalformedClarityValue(
                function_name.to_string(),
                is_blacklisted,
            ))
        }
    }

    fn is_block_claimed(
        &self,
        sender: &StacksAddress,
        block_height: u128
    ) -> Result<bool, StacksNodeError> {
        let function_name = "is-claimed";

        let is_claimed_hex = self.call_read(sender, function_name, &[("0x".to_owned() + &(hex::encode(ClarityValue::UInt(block_height).serialize_to_vec()))).as_str()])?;
        let is_claimed =  ClarityValue::try_deserialize_hex_untyped(&is_claimed_hex)?;
        if let ClarityValue::Bool(is_claimed) = is_claimed {
            Ok(is_claimed)
        } else {
            Err(StacksNodeError::MalformedClarityValue(
                function_name.to_string(),
                is_claimed,
            ))
        }
    }

    fn is_enough_voted_to_enter(
        &self,
        sender: &StacksAddress,
    ) -> Result<bool, StacksNodeError> {
        let function_name = "is-user-accepted";

        let data_hex = self.call_read(sender, function_name, &[])?;
        let data =  ClarityValue::try_deserialize_hex_untyped(&data_hex)?;
        if let ClarityValue::Bool(is_enough) = data {
            Ok(is_enough)
        } else {
            Err(StacksNodeError::MalformedClarityValue(
                function_name.to_string(),
                data,
            ))
        }
    }

    fn is_enough_blocks_passed_for_pending_miners(
        &self,
        sender: &StacksAddress,
    ) -> Result<bool, StacksNodeError> {
        let function_name = "blocks-passed-for-pending-miners";

        let data_hex = self.call_read(sender, function_name, &[])?;
        let data =  ClarityValue::try_deserialize_hex_untyped(&data_hex)?;
        if let ClarityValue::Bool(is_enough) = data {
            Ok(is_enough)
        } else {
            Err(StacksNodeError::MalformedClarityValue(
                function_name.to_string(),
                data,
            ))
        }
    }

    fn is_auto_exchange(&self, sender: &StacksAddress) -> Result<bool, StacksNodeError> {
        let function_name = "get-auto-exchange";

        let is_auto_exchange_hex = self.call_read(sender, function_name, &[("0x".to_owned() + &(hex::encode(ClarityValue::Principal(PrincipalData::from(*sender)).serialize_to_vec()))).as_str()])?;
        let is_auto_exchange =  ClarityValue::try_deserialize_hex_untyped(&is_auto_exchange_hex)?;

        if let ClarityValue::Bool(is_auto_exchange) = is_auto_exchange {
            Ok(is_auto_exchange)
        } else if let ClarityValue::Optional(is_auto_exchange) = is_auto_exchange.clone() {
            Ok(false)
        } else {
            Err(StacksNodeError::MalformedClarityValue(
                function_name.to_string(),
                is_auto_exchange,
            ))
        }
    }

    fn get_reward_info_for_block_height(
        &self,
        sender: &StacksAddress,
        block_height: u128,
    ) -> Result<(u128, PrincipalData), StacksNodeError> {
        let mut final_reward: u128 = 0;
        let mut final_claimer: PrincipalData = PrincipalData::from(*sender);
        // TODO: degens - this should not be the sender address in the end

        let function_name = "get-reward-at-block-read";
        let reward_data_hex = self.call_read(
            sender,
            function_name,
            &[("0x".to_owned() + &(hex::encode(ClarityValue::UInt(block_height).serialize_to_vec()))).as_str()],
        )?;
        let reward_data = ClarityValue::try_deserialize_hex_untyped(&reward_data_hex)?;
        // we have directly a tuple for those values, should not have optional
        // it can be a value or null

        if let ClarityValue::Tuple(tuple_data) = reward_data.clone() {
            if let Some(ClarityValue::Optional(local_reward)) = tuple_data.data_map.get(&ClarityName::from("reward")) {
                if let ClarityValue::UInt(reward) = &*local_reward.data.clone().unwrap_or(Box::new(ClarityValue::Bool(false))) {
                    final_reward = reward.clone();
                } else {
                    return Err(StacksNodeError::MalformedClarityValue(
                        function_name.to_string(),
                        reward_data
                    ));
                }
            } else {
                return Err(StacksNodeError::MalformedClarityValue(
                    function_name.to_string(),
                    reward_data
                ));
            }

            if let Some(ClarityValue::Optional(local_claimer)) = tuple_data.data_map.get(&ClarityName::from("claimer")) {
                if let ClarityValue::Principal(claimer) = &*local_claimer.data.clone().unwrap_or(Box::new(ClarityValue::Bool(false))) {
                    final_claimer = claimer.clone();
                } else {
                    return Err(StacksNodeError::MalformedClarityValue(
                        function_name.to_string(),
                        reward_data
                    ));
                }
            } else {
                return Err(StacksNodeError::MalformedClarityValue(
                    function_name.to_string(),
                    reward_data
                ));
            }
        } else {
            return Err(StacksNodeError::MalformedClarityValue(
                function_name.to_string(),
                reward_data
            ));
        }
        // TODO: check if this is the case here
        // TODO: what if the sender actually is the one who claimed the block? this will return error for no reason
        // if StacksAddress::from(final_claimer.clone()) == *sender   {
        //     return Err(StacksNodeError::MalformedClarityValue(
        //         function_name.to_string(),
        //         reward_data
        //     ));
        // }

        Ok((final_reward, final_claimer))
    }

    fn get_miners_list(&self, sender: &StacksAddress) -> Result<Vec<StacksAddress>, StacksNodeError> {
        // input: no arguments
        // output: list(Principal)
        let mut miners:Vec<StacksAddress> = Vec::new();
        let function_name = "get-miners-list";
        let miners_data_hex = self.call_read(sender, function_name, &[])?;
        let miners_data = ClarityValue::try_deserialize_hex_untyped(&miners_data_hex)?;
        if let ClarityValue::Sequence(SequenceData::List(miners_clarity)) = miners_data.clone() {
            for miner_clarity in miners_clarity.data {
                if let ClarityValue::Principal(miner_address) = miner_clarity {
                    miners.push(StacksAddress::from(miner_address));
                } else {
                    return Err(StacksNodeError::MalformedClarityValue(
                        function_name.to_string(),
                        miners_data
                    ));
                }
            }
        } else {
            return Err(StacksNodeError::MalformedClarityValue(
                function_name.to_string(),
                miners_data,
            ));
        }
        return Ok(miners);
    }

    fn get_waiting_list(&self, sender: &StacksAddress) -> Result<Vec<StacksAddress>, StacksNodeError> {
        // input: no arguments
        // output: list(Principal)
        let mut waiting_list: Vec<StacksAddress> = Vec::new();
        let function_name = "get-waiting-list";
        let waiting_list_data_hex = self.call_read(sender, function_name, &[])?;
        let waiting_list_data = ClarityValue::try_deserialize_hex_untyped(&waiting_list_data_hex)?;
        if let ClarityValue::Sequence(SequenceData::List(waiting_list_clarity)) = waiting_list_data.clone() {
            for waiting_user in waiting_list_clarity.data {
                if let ClarityValue::Principal(waiting_user_address) = waiting_user {
                    waiting_list.push(StacksAddress::from(waiting_user_address));
                } else {
                    return Err(StacksNodeError::MalformedClarityValue(
                        function_name.to_string(),
                        waiting_list_data
                    ));
                }
            }
        } else {
            return Err(StacksNodeError::MalformedClarityValue(
                function_name.to_string(),
                waiting_list_data,
            ));
        }
        return Ok(waiting_list);
    }

    fn get_pool_total_spend_per_block(&self, sender: &StacksAddress) -> Result<u128, StacksNodeError> {
        // input: no arguments
        // output: uint
        let function_name = "get-pool-total-spend-per-block";
        let amount_data_hex = self.call_read(sender, function_name, &[])?;
        let amount_data = ClarityValue::try_deserialize_hex_untyped(&amount_data_hex)?;
        if let ClarityValue::UInt(amount) = amount_data {
            Ok(amount)
        } else {
            Err(StacksNodeError::MalformedClarityValue(
                function_name.to_string(),
                amount_data,
            ))
        }
    }

    fn is_proposed_for_removal(
        &self,
        sender: &StacksAddress,
        address: &StacksAddress
    ) -> Result<bool, StacksNodeError> {
        let function_name = "get-proposed-removal-list";

        let mut proposed_for_removal_list:Vec<StacksAddress> = Vec::new();
        let proposed_removal_list_hex = self.call_read(sender, function_name, &[])?;
        let proposed_for_removal_data = ClarityValue::try_deserialize_hex_untyped(&proposed_removal_list_hex)?;

        if let ClarityValue::Sequence(SequenceData::List(proposed_for_removal_clarity)) = proposed_for_removal_data.clone() {
            for proposed_miner in proposed_for_removal_clarity.data {
                if let ClarityValue::Principal(proposed_address) = proposed_miner {
                    proposed_for_removal_list.push(StacksAddress::from(proposed_address));
                } else {
                    return Err(StacksNodeError::MalformedClarityValue(
                        function_name.to_string(),
                        proposed_for_removal_data
                    ));
                }
            }
        } else {
            return Err(StacksNodeError::MalformedClarityValue(
                function_name.to_string(),
                proposed_for_removal_data,
            ));
        }

        return Ok(proposed_for_removal_list.contains(address))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufWriter, Read, Write},
        net::{SocketAddr, TcpListener},
        thread::spawn,
    };
    use bincode::config;

    use stackslib::{
        address::{AddressHashMode, C32_ADDRESS_VERSION_TESTNET_SINGLESIG},
        burnchains::Address,
        chainstate::stacks::{
            CoinbasePayload, SinglesigHashMode, SinglesigSpendingCondition, TransactionAnchorMode,
            TransactionAuth, TransactionPayload, TransactionPostConditionMode,
            TransactionPublicKeyEncoding, TransactionSpendingCondition, TransactionVersion,
        },
        types::chainstate::{StacksPrivateKey, StacksPublicKey},
        util::{hash::Hash160, secp256k1::MessageSignature},
    };
    use stackslib::util::secp256k1::Secp256k1PrivateKey;
    use crate::peg_wallet::StacksWallet as PegWallet;
    use crate::stacks_node::Error;
    use crate::stacks_wallet::StacksWallet;

    use crate::util_versioning::test::PRIVATE_KEY_HEX;

    use super::*;

    struct ReadOnlyConfig {
        stacks_node: NodeClient,
        coordinator_wallet: StacksWallet,
        signer_1_wallet: StacksWallet,
        signer_2_wallet: StacksWallet,
    }

    impl ReadOnlyConfig {
        pub fn new() -> Self {
            let mut stacks_node = NodeClient::new(
                Url::parse("https://stacks-node-api.testnet.stacks.co/").unwrap(),
                ContractName::from( "mining-m3-testing-v2"),
                StacksAddress::from_string("ST02D2KP0630FS1BCJ7YM4TYMDH6NS9QKR0B57R3").unwrap(),
            );
            let coordinator_wallet = StacksWallet::new(
                ContractName::from("mining-m3-testing-v2"),
                StacksAddress::from_string("ST02D2KP0630FS1BCJ7YM4TYMDH6NS9QKR0B57R3").unwrap(),
                StacksPrivateKey::from_hex("c2eae79ad466a0a98d64e24fc27d0a8eaf75891c9029d5f821a67743affa874201").unwrap(),
                StacksAddress::from_string("ST02D2KP0630FS1BCJ7YM4TYMDH6NS9QKR0B57R3").unwrap(),
                TransactionVersion::Testnet,
                20000);
            let signer_1_wallet = StacksWallet::new(
                ContractName::from("mining-m3-testing-v2"),
                StacksAddress::from_string("ST02D2KP0630FS1BCJ7YM4TYMDH6NS9QKR0B57R3").unwrap(),
                StacksPrivateKey::from_hex("811ad2e8f9bafb837c6f7df8521d71a2782b19701715b511018eaa93c3ed84da01").unwrap(),
                StacksAddress::from_string("ST109H2F95ZKHDKW4G7DQSCV6ZRXJD9EA126HH4E1").unwrap(),
                TransactionVersion::Testnet,
                20000);
            let signer_2_wallet = StacksWallet::new(
                ContractName::from("mining-m3-testing-v2"),
                StacksAddress::from_string("ST02D2KP0630FS1BCJ7YM4TYMDH6NS9QKR0B57R3").unwrap(),
                StacksPrivateKey::from_hex("5bfaefc764eb822296d21fc1ff36ff88c59c62419baa5c5cfb90194cd84af8e801").unwrap(),
                StacksAddress::from_string("ST1T1XCR5RJ9NBFVMVAKWJDZ14M4RT9WJNNAK1A8Z").unwrap(),
                TransactionVersion::Testnet,
                20000);

            Self {
                stacks_node,
                coordinator_wallet,
                signer_1_wallet,
                signer_2_wallet,
            }
        }
    }

    struct TestConfig {
        sender: StacksAddress,
        mock_server: TcpListener,
        client: NodeClient,
        coordinator_wallet: StacksWallet,
        signer_wallet: StacksWallet,
    }

    impl TestConfig {
        pub fn new() -> Self {
            let sender_key = StacksPrivateKey::from_hex("eb92abc7cd7ab8d7590763d6aee37c60fd5bafa6048d4f0760a27dcded7d11c501")
                .expect("Unable to generate stacks private key from hex string");

            let pk = StacksPublicKey::from_private(&sender_key);

            let sender = StacksAddress::from_public_keys(
                C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
                &AddressHashMode::SerializeP2PKH,
                1,
                &vec![pk],
            )
                .expect("Failed to generate address from private key");

            let mut mock_server_addr = SocketAddr::from(([127, 0, 0, 1], 0));
            let mock_server = TcpListener::bind(mock_server_addr).unwrap();

            mock_server_addr.set_port(mock_server.local_addr().unwrap().port());
            let client = NodeClient::new(
                Url::parse(&format!("http://{}", mock_server_addr))
                    .expect("Failed to parse mock server address"),
                ContractName::from("mining-pool"),
                StacksAddress::from_string("ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM").unwrap(),
            );
            let coordinator_wallet = StacksWallet::new(
                ContractName::from("mining-pool"),
                StacksAddress::from_string("ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM").unwrap(),
                StacksPrivateKey::from_hex("753b7cc01a1a2e86221266a154af739463fce51219d97e4f856cd7200c3bd2a601").unwrap(),
                StacksAddress::from_string("ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM").unwrap(),
                TransactionVersion::Testnet,
                2000);
            let signer_wallet = StacksWallet::new(
                ContractName::from("mining-pool"),
                StacksAddress::from_string("ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM").unwrap(),
                StacksPrivateKey::from_hex("eb92abc7cd7ab8d7590763d6aee37c60fd5bafa6048d4f0760a27dcded7d11c501").unwrap(),
                StacksAddress::from_string("ST2XK3JZ0RYKPS38N9HHMYVHGTSABPNF98RPCJDQS").unwrap(),
                TransactionVersion::Testnet,
                2000);
            Self {
                sender,
                mock_server,
                client,
                coordinator_wallet,
                signer_wallet,
            }
        }
    }

    #[test]
    fn smart_contract_flow() {
        let mut config = ReadOnlyConfig::new();
        let signer_1_address = config.signer_1_wallet.address().clone();
        let signer_2_address = config.signer_2_wallet.address().clone();
        let coordinator_address = config.coordinator_wallet.address().clone();
        let mut stacks_node_clone = config.stacks_node.clone();

        // Function: get pool total spend per block -> u3730000000
        //
        // let h = spawn(move || config.stacks_node.get_pool_total_spend_per_block(&coordinator_address));
        // let status = h.join().unwrap().unwrap();
        //
        // assert_eq!(status, 3730000000);

        // Function: get address status -> Coordinator is a miner
        //
        // let h = spawn(move || config.stacks_node.get_status(&coordinator_address));
        // let status = h.join().unwrap().unwrap();
        //
        // assert_eq!(status, MinerStatus::Miner);


        // Function: get address status -> Signer 2 is a normal user
        //
        // let h = spawn(move || config.stacks_node.get_status(&signer_2_address));
        // let status = h.join().unwrap().unwrap();
        //
        // assert_eq!(status, MinerStatus::NormalUser);


        // Function: get miners list -> Coordinator + Signer 1
        //
        // let h = spawn(move || config.stacks_node.get_miners_list(&signer_1_address));
        // let status = h.join().unwrap().unwrap();
        //
        // assert_eq!(status, vec![coordinator_address, signer_1_address]);


        // Function: get auto exchange -> Coordinator - false
        //
        // let h = spawn(move || config.stacks_node.is_auto_exchange(&coordinator_address));
        // let status = h.join().unwrap().unwrap();
        //
        // assert_eq!(status, false);


        // Function: ask to join -> Signer 2 - response (ok true)
        // Example: 0x09002f8da81226a3a5cdccd0cc56b68d30070fea83547164f0c37ce1b1851dbd
        //
        // let h = spawn(move || config.signer_2_wallet.ask_to_join(stacks_node_clone.next_nonce(&signer_2_address).unwrap(), [0u8; 32].serialize_to_vec()));
        // let tx = h.join().unwrap().unwrap();
        //
        // let h = spawn(move || config.stacks_node.broadcast_transaction(&tx));
        // let broadcasted = h.join().unwrap();
        //
        // match broadcasted {
        //     Ok(()) => {}
        //     Err(e) => {panic!("{}", e)}
        // }


        // Function: get address status -> Signer 2 is waiting
        //
        // let h = spawn(move || config.stacks_node.get_status(&signer_2_address));
        // let status = h.join().unwrap().unwrap();
        //
        // assert_eq!(status, MinerStatus::Waiting);


        // Function: is blacklisted -> Signer 2 - false
        //
        // let h = spawn(move || config.stacks_node.is_blacklisted(&coordinator_address, &signer_2_address));
        // let status = h.join().unwrap().unwrap();
        //
        // assert_eq!(status, false);


        // Function: vote positive join request - Coordinator -> Signer 2 - response (ok true)
        // Example: 0x25389e6706b2ebc79f8c2930fc26f89f092b9df838a98d64d821b710e36b454e
        //
        // let h = spawn(move || config.coordinator_wallet.vote_positive_join_request(stacks_node_clone.next_nonce(&coordinator_address).unwrap(), signer_2_address));
        // let tx = h.join().unwrap().unwrap();
        //
        // let h = spawn(move || config.stacks_node.broadcast_transaction(&tx));
        // let broadcasted = h.join().unwrap();
        //
        // match broadcasted {
        //     Ok(()) => {}
        //     Err(e) => {panic!("{}", e)}
        // }


        // Function: vote negative join request - Coordinator -> Signer 2 - response (err u108) - already voted
        // Example: 0xf963742accbb6b218bcdebfac41bd7b32b0ddf7cc041c89b84e7a83dbb8cb94e
        //
        // let h = spawn(move || config.coordinator_wallet.vote_negative_join_request(stacks_node_clone.next_nonce(&coordinator_address).unwrap(), signer_2_address));
        // let tx = h.join().unwrap().unwrap();
        //
        // let h = spawn(move || config.stacks_node.broadcast_transaction(&tx));
        // let broadcasted = h.join().unwrap();
        //
        // match broadcasted {
        //     Ok(()) => {}
        //     Err(e) => {panic!("{}", e)}
        // }


        // Function: is enough voted to enter -> Signer 2 - true
        //
        // let h = spawn(move || config.stacks_node.is_enough_voted_to_enter(&signer_2_address));
        // let status = h.join().unwrap().unwrap();
        //
        // assert_eq!(status, true);


        // Function: try enter pool - Signer 2 - response (ok true)
        // Example: 0xca92cf119414a2fa484b787f1680ce0df3d403b3eebd6ff0b16af5589a6796bc
        //
        // let h = spawn(move || config.signer_2_wallet.call_try_enter(stacks_node_clone.next_nonce(&signer_2_address).unwrap()));
        // let tx = h.join().unwrap().unwrap();
        //
        // let h = spawn(move || config.stacks_node.broadcast_transaction(&tx));
        // let broadcasted = h.join().unwrap();
        //
        // match broadcasted {
        //     Ok(()) => {}
        //     Err(e) => {panic!("{}", e)}
        // }


        // Function: get address status -> Signer 2 is pending
        //
        // let h = spawn(move || config.stacks_node.get_status(&signer_2_address));
        // let status = h.join().unwrap().unwrap();
        //
        // assert_eq!(status, MinerStatus::Pending);


        // Function: can enter pending miners -> true
        //
        // let h = spawn(move || config.stacks_node.is_enough_blocks_passed_for_pending_miners(&signer_2_address));
        // let status = h.join().unwrap().unwrap();
        //
        // assert_eq!(status, true);


        // Function: add pending miners to pool - Signer 2 - response (ok true)
        // Example: 0x72d1b3938cbcb868710d509ba1fccbd00113e7db790c6509ab5b7419b5fe2db2
        //
        // let h = spawn(move || config.signer_2_wallet.add_pending_miners_to_pool(stacks_node_clone.next_nonce(&signer_2_address).unwrap()));
        // let tx = h.join().unwrap().unwrap();
        //
        // let h = spawn(move || config.stacks_node.broadcast_transaction(&tx));
        // let broadcasted = h.join().unwrap();
        //
        // match broadcasted {
        //     Ok(()) => {}
        //     Err(e) => {panic!("{}", e)}
        // }


        // Function: get address status -> Signer 2 is miner
        //
        // let h = spawn(move || config.stacks_node.get_status(&signer_2_address));
        // let status = h.join().unwrap().unwrap();
        //
        // assert_eq!(status, MinerStatus::Miner);


        // Function: get miners list -> Coordinator + Signer 1 + Signer 2
        //
        // let h = spawn(move || config.stacks_node.get_miners_list(&signer_1_address));
        // let status = h.join().unwrap().unwrap();
        //
        // assert_eq!(status, vec![coordinator_address, signer_1_address, signer_2_address]);


        // Function: get notifier -> Coordinator
        //
        // let h = spawn(move || config.stacks_node.get_notifier(&signer_1_address));
        // let status = h.join().unwrap().unwrap();
        //
        // assert_eq!(status, PrincipalData::from(coordinator_address));


        // Function: get warnings -> Signer 2 - 0
        //
        // let h = spawn(move || config.stacks_node.get_warn_number_user(&coordinator_address, &signer_2_address));
        // let status = h.join().unwrap().unwrap();
        //
        // assert_eq!(status, 0);


        // Function: warn - Signer 2 - response (ok true)
        // Example: 0x1c9b21c037a71cca0008d5546cdfece10aaeaca4de6abd56259fb0b043f08e3a
        //
        // let h = spawn(move || config.coordinator_wallet.warn_miner(stacks_node_clone.next_nonce(&coordinator_address).unwrap(), signer_2_address));
        // let tx = h.join().unwrap().unwrap();
        //
        // let h = spawn(move || config.stacks_node.broadcast_transaction(&tx));
        // let broadcasted = h.join().unwrap();
        //
        // match broadcasted {
        //     Ok(()) => {}
        //     Err(e) => {panic!("{}", e)}
        // }


        // Function: get warnings -> Signer 2 - 1
        //
        // let h = spawn(move || config.stacks_node.get_warn_number_user(&coordinator_address, &signer_2_address));
        // let status = h.join().unwrap().unwrap();
        //
        // assert_eq!(status, 1);


        // Function: warn - Signer 2 - response (ok true)
        // Example: 0xda326274b176c008c37b87e263134e6f2856f8cd002c9c5f12e408bb827776c3
        //
        // let h = spawn(move || config.coordinator_wallet.warn_miner(stacks_node_clone.next_nonce(&coordinator_address).unwrap(), signer_2_address));
        // let tx = h.join().unwrap().unwrap();
        //
        // let h = spawn(move || config.stacks_node.broadcast_transaction(&tx));
        // let broadcasted = h.join().unwrap();
        //
        // match broadcasted {
        //     Ok(()) => {}
        //     Err(e) => {panic!("{}", e)}
        // }


        // Function: get warnings -> Signer 2 - 2
        //
        // let h = spawn(move || config.stacks_node.get_warn_number_user(&coordinator_address, &signer_2_address));
        // let status = h.join().unwrap().unwrap();
        //
        // assert_eq!(status, 2);


        // Function: propose removal - Coordinator -> Signer 2 - response (ok true)
        // Example: 0x4b92dc1314da17a3156599215686133f6af6e4311dd54c058e35a4d8dc9c915atrue
        //
        // let h = spawn(move || config.coordinator_wallet.propose_removal(stacks_node_clone.next_nonce(&coordinator_address).unwrap(), signer_2_address));
        // let tx = h.join().unwrap().unwrap();
        //
        // let h = spawn(move || config.stacks_node.broadcast_transaction(&tx));
        // let broadcasted = h.join().unwrap();
        //
        // match broadcasted {
        //     Ok(()) => {}
        //     Err(e) => {panic!("{}", e)}
        // }


        // Function: vote negative remove request - Coordinator -> Signer 2 - response (ok true)
        // Example: 0xf0d0e101bb024dd02b04dc8646f99b7a83d064c6ef684189bb5f94e1b313f2a6
        //
        // let h = spawn(move || config.coordinator_wallet.vote_negative_remove_request(stacks_node_clone.next_nonce(&coordinator_address).unwrap(), signer_2_address));
        // let tx = h.join().unwrap().unwrap();
        //
        // let h = spawn(move || config.stacks_node.broadcast_transaction(&tx));
        // let broadcasted = h.join().unwrap();
        //
        // match broadcasted {
        //     Ok(()) => {}
        //     Err(e) => {panic!("{}", e)}
        // }


        // Function: propose removal - Coordinator -> Signer 2 - response (err u116) - signer 2 already proposed for removal (also need signer 1 negative vote)
        // Example: 0xf68bff6a1be0d0ef59146bc1ff4d0cd08e8acd1228c79a20b7c013be48b9892a
        //
        // let h = spawn(move || config.coordinator_wallet.propose_removal(stacks_node_clone.next_nonce(&coordinator_address).unwrap(), signer_2_address));
        // let tx = h.join().unwrap().unwrap();
        //
        // let h = spawn(move || config.stacks_node.broadcast_transaction(&tx));
        // let broadcasted = h.join().unwrap();
        //
        // match broadcasted {
        //     Ok(()) => {}true
        //     Err(e) => {panic!("{}", e)}
        // }


        // Function: vote negative remove request - Signer 1 -> Signer 2 - response (ok true)
        // Example: 0xa9f8ac532ba37bc4538427f0b22aa6d611ea5bd8a852fd85062ad7d195315688
        //
        // let h = spawn(move || config.signer_1_wallet.vote_negative_remove_request(stacks_node_clone.next_nonce(&signer_1_address).unwrap(), signer_2_address));
        // let tx = h.join().unwrap().unwrap();
        //
        // let h = spawn(move || config.stacks_node.broadcast_transaction(&tx));
        // let broadcasted = h.join().unwrap();
        //
        // match broadcasted {
        //     Ok(()) => {}
        //     Err(e) => {panic!("{}", e)}
        // }


        // Function: propose removal - Coordinator -> Signer 2 - response (ok true)
        // Example: 0x6060697283c4570e73cd51cac142ced7c3019253b0f5bf8e82d17fb89325e63c
        //
        // let h = spawn(move || config.coordinator_wallet.propose_removal(stacks_node_clone.next_nonce(&coordinator_address).unwrap(), signer_2_address));
        // let tx = h.join().unwrap().unwrap();
        //
        // let h = spawn(move || config.stacks_node.broadcast_transaction(&tx));
        // let broadcasted = h.join().unwrap();
        //
        // match broadcasted {
        //     Ok(()) => {}
        //     Err(e) => {panic!("{}", e)}
        // }


        // Function: vote positive remove request - Coordinator -> Signer 2 - response (ok true)
        // Example: 0xfcb091e97b84d32cfa0056a9adb0db1ff1593f95ceaadb520e679d23671e4f2a
        //
        // let h = spawn(move || config.coordinator_wallet.vote_positive_remove_request(stacks_node_clone.next_nonce(&coordinator_address).unwrap(), signer_2_address));
        // let tx = h.join().unwrap().unwrap();
        //
        // let h = spawn(move || config.stacks_node.broadcast_transaction(&tx));
        // let broadcasted = h.join().unwrap();
        //
        // match broadcasted {
        //     Ok(()) => {}
        //     Err(e) => {panic!("{}", e)}
        // }


        // Function: is blacklisted -> Signer 2 - true
        //
        // let h = spawn(move || config.stacks_node.is_blacklisted(&coordinator_address, &signer_2_address));
        // let status = h.join().unwrap().unwrap();
        //
        // assert_eq!(status, true);


        // Function: get miners list -> Coordinator + Signer 1
        //
        // let h = spawn(move || config.stacks_node.get_miners_list(&signer_1_address));
        // let status = h.join().unwrap().unwrap();
        //
        // assert_eq!(status, vec![coordinator_address, signer_1_address]);
    }

    #[test]
    fn get_address_status() {
        let config = TestConfig::new();
        let address = config.signer_wallet.address().clone();

        let h = spawn(move || config.client.get_status(&address));

        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x070d0000000769732d6e6f6e65\"}"
        );
        let result = h.join().unwrap().unwrap();

        assert_eq!(result, MinerStatus::NormalUser);
    }

    #[test]
    fn get_warn_number_user() {
        let config = TestConfig::new();
        let address = config.client.contract_address;

        let h = spawn(move || config.client.get_warn_number_user(
            &address,
            &StacksAddress::from_string("ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM").unwrap()
        ));

        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x0100000000000000000000000000000000\"}"
        );
        let result = h.join().unwrap().unwrap();

        assert_eq!(result, 0);
    }

    #[test]
    fn get_notifier() {
        let config = TestConfig::new();
        let address = config.client.contract_address;

        let h = spawn(move || config.client.get_notifier(&address));

        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x051a6d78de7b0625dfbfc16c3a8a5735f6dc3dc3f2ce\"}"
        );
        let result = h.join().unwrap().unwrap();

        assert_eq!(result, PrincipalData::from(StacksAddress::from_string("ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM").unwrap()));
    }

    #[test]
    fn is_blacklisted() {
        let config = TestConfig::new();
        let address = config.client.contract_address;

        let h = spawn(move || config.client.is_blacklisted(
            &address,
            &StacksAddress::from_string("ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM").unwrap()
        ));

        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x04\"}"
        );
        let result = h.join().unwrap().unwrap();

        assert_eq!(result, false);
    }

    #[test]
    fn is_block_claimed() {
        let config = TestConfig::new();
        let address = config.client.contract_address;

        let h = spawn(move || config.client.is_block_claimed(
            &address,
            10,
        ));

        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x04\"}"
        );
        let result = h.join().unwrap().unwrap();

        assert_eq!(result, false);
    }

    #[test]
    fn is_enough_voted_to_enter() {
        let config = TestConfig::new();
        let address = config.client.contract_address;

        let h = spawn(move || config.client.is_enough_voted_to_enter(&address));

        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x04\"}"
        );
        let result = h.join().unwrap().unwrap();

        assert_eq!(result, false);
    }

    #[test]
    fn is_enough_blocks_passed_for_pending_miners() {
        let config = TestConfig::new();
        let address = config.client.contract_address;

        let h = spawn(move || config.client.is_enough_blocks_passed_for_pending_miners(&address));

        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x04\"}"
        );
        let result = h.join().unwrap().unwrap();

        assert_eq!(result, false);
    }

    #[test]
    fn is_auto_exchange() {
        let config = TestConfig::new();
        let address = config.client.contract_address;

        let h = spawn(move || config.client.is_blacklisted(
            &address,
            &StacksAddress::from_string("ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM").unwrap()
        ));

        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x04\"}"
        );
        let result = h.join().unwrap().unwrap();

        assert_eq!(result, false);
    }

    #[test]
    fn get_reward_info_for_block_height() {
        let config = TestConfig::new();
        let address = config.client.contract_address;

        let h = spawn(move || config.client.get_reward_info_for_block_height(
            &address,
            10,
        ));

        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x0c0000000207636c61696d65720a051aee9369fb719c0ba43ddf4d94638a970b84775f47067265776172640a010000000000000000000000003b9f5de0\"}"
        );
        let result = h.join().unwrap().unwrap();

        assert_eq!(result, (1000300000, PrincipalData::from(StacksAddress::from_string("ST3Q96TFVE6E0Q91XVX6S8RWAJW5R8XTZ8YEBM8RQ").unwrap())));
    }

    #[test]
    fn get_miners_list() {
        let config = TestConfig::new();
        let address = config.client.contract_address;

        let h = spawn(move || config.client.get_miners_list(
            &address,
        ));

        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x0b00000001051a6d78de7b0625dfbfc16c3a8a5735f6dc3dc3f2ce\"}"
        );
        let result = h.join().unwrap().unwrap();

        assert_eq!(result, [StacksAddress::from_string("ST1PQHQKV0RJXZFY1DGX8MNSNYVE3VGZJSRTPGZGM").unwrap()]);
    }

    #[test]
    fn get_waiting_list() {
        let config = TestConfig::new();
        let address = config.client.contract_address;

        let h = spawn(move || config.client.get_waiting_list(
            &address,
        ));

        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x0b00000000\"}"
        );
        let result = h.join().unwrap().unwrap();
        info!("{result:?}");

        assert_eq!(result, []);
    }

    fn write_response(mock_server: TcpListener, bytes: &[u8]) -> [u8; 1024] {
        let mut request_bytes = [0u8; 1024];
        {
            let mut stream = mock_server.accept().unwrap().0;

            stream.read(&mut request_bytes).unwrap();
            stream.write(bytes).unwrap();
        }
        request_bytes
    }

    #[test]
    fn call_read_success_test() {
        let config = TestConfig::new();
        let h = spawn(move || {
            config
                .client
                .call_read(&config.sender, "function-name", &[])
        });
        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x070d0000000473425443\"}",
        );
        let result = h.join().unwrap().unwrap();
        assert_eq!(result, "0x070d0000000473425443");
    }

    #[test]
    fn call_read_failure_test() {
        let config = TestConfig::new();
        let h = spawn(move || {
            config
                .client
                .call_read(&config.sender, "function-name", &[])
        });
        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"okay\":false,\"cause\":\"Some reason\"}",
        );
        let result = h.join().unwrap();
        assert!(matches!(result, Err(StacksNodeError::ReadOnlyFailure(_))));
    }

    #[test]
    fn signer_data_none_test() {
        let config = TestConfig::new();

        let h = spawn(move || {
            let mut public_keys = PublicKeys::default();
            let mut signer_key_ids = SignerKeyIds::default();
            config
                .client
                .signer_data(&config.sender, 1u128, &mut public_keys, &mut signer_key_ids)
        });
        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x09\"}",
        );
        let result = h.join().unwrap();
        assert!(matches!(result, Err(StacksNodeError::NoSignerData(_))));
    }

    #[test]
    fn keys_threshold_test() {
        let config = TestConfig::new();

        let h = spawn(move || config.client.keys_threshold(&config.sender));

        write_response(config.mock_server, b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x0100000000000000000000000000000af0\"}");
        let result = h.join().unwrap().unwrap();
        assert_eq!(result, 2800);
    }

    #[test]
    fn keys_threshold_invalid_test() {
        let config = TestConfig::new();

        let h = spawn(move || config.client.keys_threshold(&config.sender));
        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x09\"}",
        );
        let result = h.join().unwrap();
        assert!(matches!(
            result,
            Err(StacksNodeError::MalformedClarityValue(..))
        ));
    }

    #[test]
    fn num_signers_test() {
        let config = TestConfig::new();

        let h = spawn(move || config.client.num_signers(&config.sender));
        write_response(config.mock_server,
                       b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x0100000000000000000000000000000fa0\"}"
        );
        let result = h.join().unwrap().unwrap();
        assert_eq!(result, 4000);
    }

    #[test]
    fn num_signers_invalid_test() {
        let config = TestConfig::new();

        let h = spawn(move || config.client.num_signers(&config.sender));
        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"okay\":true,\"result\":\"0x09\"}",
        );
        let result = h.join().unwrap();
        assert!(matches!(
            result,
            Err(StacksNodeError::MalformedClarityValue(..))
        ));
    }

    #[test]
    fn next_nonce_success_test() {
        let mut config = TestConfig::new();

        let h = spawn(move || {
            let nonce = config.client.next_nonce(&config.sender).unwrap();
            let next_nonce = config.client.next_nonce(&config.sender).unwrap();
            (nonce, next_nonce)
        });
        write_response(config.mock_server,
                       b"HTTP/1.1 200 OK\n\n{\"balance\":\"0x00000000000000000000000000000000\",\"locked\":\"0x00000000000000000000000000000000\",\"unlock_height\":0,\"nonce\":20,\"balance_proof\":\"\",\"nonce_proof\":\"\"}"
        );
        let (nonce, next_nonce) = h.join().unwrap();
        assert_eq!(nonce, 20);
        assert_eq!(next_nonce, 21);
    }

    #[test]
    fn next_nonce_failure_test() {
        let mut config = TestConfig::new();

        let h = spawn(move || config.client.next_nonce(&config.sender));
        write_response(
            config.mock_server,
            b"HTTP/1.1 404 Not Found\n\n/v2/accounts/SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE",
        );
        let result = h.join().unwrap();
        assert!(matches!(result, Err(StacksNodeError::UnknownAddress(_))));
    }

    #[test]
    fn burn_block_height_success_test() {
        let config = TestConfig::new();

        let h = spawn(move || config.client.burn_block_height());
        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"peer_version\":420759911,\"burn_block_height\":2430220}",
        );
        let result = h.join().unwrap().unwrap();
        assert_eq!(result, 2430220);
    }

    #[test]
    fn burn_block_height_failure_test() {
        let config = TestConfig::new();

        let h = spawn(move || config.client.burn_block_height());
        write_response(
            config.mock_server,
            b"HTTP/1.1 200 OK\n\n{\"peer_version\":420759911,\"burn_block_height2\":2430220}",
        );
        let result = h.join().unwrap();
        assert!(matches!(result, Err(StacksNodeError::InvalidJsonEntry(_))));
    }

    #[test]
    fn should_send_tx_bytes_to_node() {
        let config = TestConfig::new();
        let tx = StacksTransaction {
            version: TransactionVersion::Testnet,
            chain_id: 0,
            auth: TransactionAuth::Standard(TransactionSpendingCondition::Singlesig(
                SinglesigSpendingCondition {
                    hash_mode: SinglesigHashMode::P2PKH,
                    signer: Hash160([0; 20]),
                    nonce: 0,
                    tx_fee: 0,
                    key_encoding: TransactionPublicKeyEncoding::Uncompressed,
                    signature: MessageSignature([0; 65]),
                },
            )),
            anchor_mode: TransactionAnchorMode::Any,
            post_condition_mode: TransactionPostConditionMode::Allow,
            post_conditions: vec![],
            payload: TransactionPayload::Coinbase(CoinbasePayload([0; 32]), None),
        };

        let mut tx_bytes = [0u8; 1024];
        {
            let mut tx_bytes_writer = BufWriter::new(&mut tx_bytes[..]);
            tx.consensus_serialize(&mut tx_bytes_writer).unwrap();
            tx_bytes_writer.flush().unwrap();
        }

        let bytes_len = tx_bytes
            .iter()
            .enumerate()
            .rev()
            .find(|(_, &x)| x != 0)
            .unwrap()
            .0
            + 1;

        let h = spawn(move || config.client.broadcast_transaction(&tx));

        let request_bytes = write_response(config.mock_server, b"HTTP/1.1 200 OK\n\n");
        h.join().unwrap().unwrap();

        assert!(
            request_bytes
                .windows(bytes_len)
                .any(|window| window == &tx_bytes[..bytes_len]),
            "Request bytes did not contain the transaction bytes"
        );
    }
}
