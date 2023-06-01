use std::{io::Cursor, str::FromStr};

use bitcoin::{
    Address as BitcoinAddress,
    consensus::Encodable,
    hashes::{hex::ToHex, sha256d::Hash}, Txid,
};
use rusqlite::types::Type::Integer;
use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};
use tracing::{debug, warn};

pub trait BitcoinNode {
    /// Broadcast the BTC transaction to the bitcoin node
    fn broadcast_transaction(&self, tx: &BitcoinTransaction) -> Result<Txid, Error>;
    /// Load the Bitcoin wallet from the given address
    fn load_wallet(&self, address: &BitcoinAddress) -> Result<(), Error>;
    /// Get all utxos from the given address
    fn list_unspent(&self, address: &BitcoinAddress) -> Result<Vec<UTXO>, Error>;
}

pub type BitcoinTransaction = bitcoin::Transaction;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO Error: {0}")]
    IOError(#[from] std::io::Error),
    #[error("RPC Error: {0}")]
    RPCError(String),
    #[error("{0}")]
    InvalidResponseJSON(String),
    #[error("Invalid utxo: {0}")]
    InvalidUTXO(String),
    #[error("Invalid transaction hash")]
    InvalidTxHash,
}

#[derive(Debug, Deserialize)]
struct RpcErrorResponse {
    error: RpcError,
}

#[derive(Debug, Deserialize)]
struct RpcError {
    code: i32,
    message: String,
}

#[allow(non_snake_case)]
#[derive(Debug, Deserialize, Serialize, Default, PartialEq, Eq, Clone)]
pub struct UTXO {
    pub txid: String,
    pub vout: u32,
    pub address: String,
    pub label: String,
    pub scriptPubKey: String,
    pub amount: u64,
    pub confirmations: u64,
    pub redeemScript: String,
    pub witnessScript: String,
    pub spendable: bool,
    pub solvable: bool,
    pub reused: bool,
    pub desc: String,
    pub safe: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct Wallet {
    name: String,
    warning: String,
}

pub struct LocalhostBitcoinNode {
    bitcoind_api: String,
}

impl BitcoinNode for LocalhostBitcoinNode {
    fn broadcast_transaction(&self, tx: &BitcoinTransaction) -> Result<Txid, Error> {
        let mut writer = Cursor::new(vec![]);
        tx.consensus_encode(&mut writer)?;
        let raw_tx = writer.into_inner().to_hex();

        let result = self
            .call("sendrawtransaction", vec![raw_tx])?
            .as_str()
            .ok_or(Error::InvalidResponseJSON(
                "No transaction hash in sendrawtransaction response".to_string(),
            ))?
            .to_string();

        Ok(Txid::from_hash(
            Hash::from_str(&result).map_err(|_| Error::InvalidTxHash)?,
        ))
    }

    fn load_wallet(&self, address: &BitcoinAddress) -> Result<(), Error> {
        let wallet_name: &str = "muffy.dat";
        let result = self.create_empty_wallet(wallet_name);
        if let Err(Error::RPCError(message)) = &result {
            if !message.ends_with("Database already exists.") {
                return result;
            }
            // If the database already exists, no problem. Just emit a warning.
            warn!(message);
            // If wallet is not listed, load it
            if !self.list_wallets()?.contains(&wallet_name.to_string()) {
                self.load_wallet_helper(wallet_name)?;
            }
        }
        // Import the address
        self.import_address(address)?;
        Ok(())
    }

    /// List the UTXOs filtered on a given address.
    fn list_unspent(&self, address: &BitcoinAddress) -> Result<Vec<UTXO>, Error> {
        // Construct the params using defaults found at https://developer.bitcoin.org/reference/rpc/listunspent.html?highlight=listunspent
        let addresses: Vec<String> = vec![address.to_string()];
        let min_conf = 0i64;
        let max_conf = 9999999i64;
        let params = (min_conf, max_conf, addresses);

        let response = self.call("listunspent", params)?;

        // Convert the response to a vector of unspent transactions
        let result: Result<Vec<UTXO>, Error> = response
            .as_array()
            .ok_or(Error::InvalidResponseJSON(
                "Listunspent response is not an array".to_string(),
            ))?
            .iter()
            .map(Self::raw_to_utxo)
            .collect();

        result
    }
}

impl LocalhostBitcoinNode {
    pub fn new(bitcoind_api: String) -> LocalhostBitcoinNode {
        Self { bitcoind_api }
    }

    /// Make the Bitcoin RPC method call with the corresponding parameters
    fn call(
        &self,
        method: &str,
        params: impl ureq::serde::Serialize,
    ) -> Result<serde_json::Value, Error> {
        debug!("Making Bitcoin RPC {} call...", method);
        let json_rpc =
            ureq::json!({"jsonrpc": "2.0", "id": "stx", "method": method, "params": params});
        println!("{:#?}", &json_rpc);
        // let response =
        match ureq::post(&self.bitcoind_api)
            .send_json(json_rpc) {
            Ok(response) => {
                let json_response = response.into_json::<serde_json::Value>()?;
                let json_result = json_response
                    .get("result")
                    .ok_or_else(|| Error::InvalidResponseJSON("Missing entry 'result'.".to_string()))?
                    .to_owned();
                Ok(json_result)
            }
            Err(ureq::Error::Status(code, response)) => {
                let rpc_response: RpcErrorResponse = serde_json::from_str(&response.into_string()?)
                    .map_err(|e| Error::InvalidResponseJSON(e.to_string()))?;
                println!("Response {:#?}", &rpc_response);
                Err(Error::RPCError(rpc_response.error.message))
            }
            Err(error) => { Err(Error::RPCError(error.to_string())) }
        }
        // TODO: degens - check whether this would output the right way or not - with TM code
        // why is this failing post request

        // let response = ureq::post(&self.bitcoind_api)
        //     .send_json(json_rpc)
        //     .map_err(|e| Error::RPCError(e.to_string()))?;
        // let json_response = response.into_json::<serde_json::Value>()?;
        // let json_result = json_response
        //     .get("result")
        //     .ok_or_else(|| Error::InvalidResponseJSON("Missing entry 'result'.".to_string()))?
        //     .to_owned();
        // Ok(json_result)
    }

    fn create_empty_wallet(&self, wallet_name: &str) -> Result<(), Error> {
        // TODO: degens - check if we want a legacy wallet or a normal one
        // legacy wallet can't import taproot addresses
        // normal wallet can't import addresses, but can import descriptors
        let wallet_name = wallet_name; //"degen_wallet";
        let disable_private_keys = false;
        let blank = true; // how to have no HD seed
        // TODO: degens - try to keep blank=true, change descriptors=true and import p2tr_address
        let passphrase = "";
        let avoid_reuse = false;
        let descriptors = false; // this makes it legacy
        let load_on_startup = true;
        let params = (
            wallet_name,
            disable_private_keys,
            blank,
            passphrase,
            avoid_reuse,
            descriptors,
            load_on_startup,
        );
        debug!("Creating wallet...");
        let wallet = serde_json::from_value::<Wallet>(self.call("createwallet", params)?)
            .map_err(|e| Error::InvalidResponseJSON(e.to_string()))?;
        if !wallet.warning.is_empty() {
            warn!(
                "Wallet {} was not loaded cleanly: {}",
                wallet.name, wallet.warning
            );
            println!("wallet {:?}", &wallet);
        }
        Ok(())
    }

    fn import_address(&self, address: &BitcoinAddress) -> Result<(), Error> {
        let address = address.to_string();
        debug!("Importing address {}...", address);
        let label = "";
        let rescan = true;
        let p2sh = false;
        let params = (address, label, rescan, p2sh);
        self.call("importaddress", params)?;
        Ok(())
    }

    pub fn list_wallets(&self) -> Result<Vec<String>, Error> {
        debug!("Listing wallets...");
        let wallets = serde_json::from_value::<Vec<String>>(self.call("listwallets", ())?)
            .map_err(|e| Error::InvalidResponseJSON(e.to_string()))?;
        Ok(wallets)
    }

    // we can call to get the wallets directly here, or a list of wallets
    // we'll go with a list of wallets as it is more modular
    // and we simply parse all of them there
    pub fn unload_wallets(&self, wallets: &Vec<String>) -> Result<(), Error> {
        debug!("Unloading wallets...");
        for wallet_name in wallets {
            let load_on_startup = false;
            let params = (wallet_name.clone(), load_on_startup);
            self.call("unloadwallet", params)?;
        }
        Ok(())
    }

    fn load_wallet_helper(&self, wallet_name: &str) -> Result<(), Error> {
        debug!("Loading wallet {}...", wallet_name);
        let load_on_startup = false;
        let params = (wallet_name, load_on_startup);
        self.call("loadwallet", params)?;
        Ok(())
    }

    pub fn get_blockcount(&self) -> Result<u64, Error> {
        debug!("Getting block count...");
        let result = self.call("getblockcount", ())?;
        let worked = serde_json::from_value::<Number>(result)
            .map_err(|e| Error::InvalidResponseJSON(e.to_string()))?
            .as_u64().expect("block count not a number");
        Ok(worked)
    }

    fn raw_to_utxo(raw: &Value) -> Result<UTXO, Error> {
        Ok(UTXO {
            txid: raw["txid"]
                .as_str()
                .ok_or(Error::InvalidResponseJSON(
                    "Could not parse txid".to_string(),
                ))?
                .to_string(),
            vout: raw["vout"].as_u64().ok_or(Error::InvalidResponseJSON(
                "Could not parse vout".to_string(),
            ))? as u32,
            address: raw["address"]
                .as_str()
                .ok_or(Error::InvalidResponseJSON(
                    "Could not parse address".to_string(),
                ))?
                .to_string(),
            label: raw["label"]
                .as_str()
                .ok_or(Error::InvalidResponseJSON(
                    "Could not parse label".to_string(),
                ))?
                .to_string(),
            scriptPubKey: raw["scriptPubKey"]
                .as_str()
                .ok_or(Error::InvalidResponseJSON(
                    "Could not parse scriptPubKey".to_string(),
                ))?
                .to_string(),
            amount: raw["amount"].as_f64().map(|amount| amount as u64).ok_or(
                Error::InvalidResponseJSON("Could not parse amount".to_string()),
            )?,
            confirmations: raw["confirmations"]
                .as_u64()
                .ok_or(Error::InvalidResponseJSON(
                    "Could not parse confirmations".to_string(),
                ))?,
            redeemScript: "".to_string(),
            witnessScript: "".to_string(),
            spendable: raw["spendable"]
                .as_bool()
                .ok_or(Error::InvalidResponseJSON(
                    "Could not parse spendable".to_string(),
                ))?,
            solvable: raw["solvable"].as_bool().ok_or(Error::InvalidResponseJSON(
                "Could not parse solvable".to_string(),
            ))?,
            reused: false,
            desc: "".to_string(),
            safe: raw["safe"].as_bool().ok_or(Error::InvalidResponseJSON(
                "Could not parse safe".to_string(),
            ))?,
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn should_map_json_to_utxo() {
        let value = json!({
            "address": "bcrt1qykqup0h6ry9x3c89llzpznrvm9nfd7fqwnt0hu",
            "amount": 50.00000000,
            "confirmations": 123,
            "label": "",
            "parent_descs": [],
            "safe": true,
            "scriptPubKey": "00142581c0befa190a68e0e5ffc4114c6cd96696f920",
            "solvable": false,
            "spendable": false,
            "txid": "19b7fb5fd6dc25b76aeedb812b7fdc7bf8fac343913706c8b39d23ef7375860c",
            "vout": 0,
        });

        let res = LocalhostBitcoinNode::raw_to_utxo(&value).unwrap();

        assert_eq!(
            res,
            UTXO {
                txid: "19b7fb5fd6dc25b76aeedb812b7fdc7bf8fac343913706c8b39d23ef7375860c"
                    .to_string(),
                vout: 0,
                address: "bcrt1qykqup0h6ry9x3c89llzpznrvm9nfd7fqwnt0hu".to_string(),
                label: "".to_string(),
                scriptPubKey: "00142581c0befa190a68e0e5ffc4114c6cd96696f920".to_string(),
                amount: 50,
                confirmations: 123,
                redeemScript: "".to_string(),
                witnessScript: "".to_string(),
                spendable: false,
                solvable: false,
                reused: false,
                desc: "".to_string(),
                safe: true,
            }
        );
    }
}
