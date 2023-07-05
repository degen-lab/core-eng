use std::time::{Duration, Instant};

use crate::stacks_node::{Error as StacksNodeError, PegInOp, PegOutRequestOp, StacksNode};
use blockstack_lib::{
    chainstate::stacks::StacksTransaction, codec::StacksMessageCodec,
    types::chainstate::StacksAddress,
};
use reqwest::{
    blocking::{Client, Response},
    StatusCode,
};
use serde_json::Value;
use tracing::{debug, warn};

pub struct NodeClient {
    node_url: String,
    client: Client,
}

impl NodeClient {
    pub fn new(url: &str) -> Self {
        Self {
            node_url: url.to_string(),
            client: Client::new(),
        }
    }

    fn build_url(&self, route: &str) -> String {
        format!("{}{}", self.node_url, route)
    }

    fn get_response(&self, route: &str) -> Result<Response, StacksNodeError> {
        let url = self.build_url(route);
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
            let request = self.client.get(&url);
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

    fn next_nonce(&self, address: &StacksAddress) -> Result<u64, StacksNodeError> {
        debug!("Retrieving next nonce...");
        let address = address.to_string();
        let entry = "nonce";
        let json = self
            .get_response(&format!("/v2/accounts/{}", address))?
            .json::<Value>()
            .map_err(|_| StacksNodeError::BehindChainTip)?;
        json[entry]
            .as_u64()
            .ok_or_else(|| StacksNodeError::InvalidJsonEntry(entry.to_string()))
    }

    fn broadcast_transaction(&self, tx: &StacksTransaction) -> Result<(), StacksNodeError> {
        debug!("Broadcasting transaction...");
        let url = self.build_url("/v2/transactions");
        let mut buffer = vec![];

        tx.consensus_serialize(&mut buffer)?;

        let response = self
            .client
            .post(url)
            .header("content-type", "application/octet-stream")
            .body(buffer)
            .send()?;

        if response.status() != StatusCode::OK {
            let json_response = response
                .json::<serde_json::Value>()
                .map_err(|_| StacksNodeError::BehindChainTip)?;
            let error_str = json_response.as_str().unwrap_or("Unknown Reason");
            warn!(
                "Failed to broadcast transaction to the stacks node: {:?}",
                error_str
            );
            return Err(StacksNodeError::BroadcastFailure(error_str.to_string()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        io::{BufWriter, Read, Write},
        net::{SocketAddr, TcpListener},
        thread::{sleep, spawn},
        time::Duration,
    };

    use blockstack_lib::{
        chainstate::stacks::{
            CoinbasePayload, SinglesigHashMode, SinglesigSpendingCondition, TransactionAnchorMode,
            TransactionAuth, TransactionPayload, TransactionPostConditionMode,
            TransactionPublicKeyEncoding, TransactionSpendingCondition, TransactionVersion,
        },
        util::{hash::Hash160, secp256k1::MessageSignature},
    };

    use super::*;

    #[test]
    fn should_send_tx_bytes_to_node() {
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

        let mut mock_server_addr = SocketAddr::from(([127, 0, 0, 1], 0));
        let mock_server = TcpListener::bind(mock_server_addr).unwrap();

        mock_server_addr.set_port(mock_server.local_addr().unwrap().port());

        let h = spawn(move || {
            sleep(Duration::from_millis(100));

            let client = NodeClient::new(&format!("http://{}", mock_server_addr));
            client.broadcast_transaction(&tx).unwrap();
        });

        let mut request_bytes = [0u8; 1024];

        {
            let mut stream = mock_server.accept().unwrap().0;

            stream.read(&mut request_bytes).unwrap();
            stream.write("HTTP/1.1 200 OK\n\n".as_bytes()).unwrap();
        }

        h.join().unwrap();

        assert!(
            request_bytes
                .windows(bytes_len)
                .any(|window| window == &tx_bytes[..bytes_len]),
            "Request bytes did not contain the transaction bytes"
        );
    }
}
