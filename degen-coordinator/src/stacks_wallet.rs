use crate::{
    peg_wallet::{Error as PegWalletError, StacksWallet as StacksWalletTrait},
    stacks_node::{PegInOp, PegOutRequestOp},
};
use bitcoin::Address as BitcoinAddress;
use blockstack_lib::{
    address::{
        AddressHashMode, C32_ADDRESS_VERSION_MAINNET_SINGLESIG,
        C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
    },
    chainstate::stacks::{
        StacksTransaction, StacksTransactionSigner, TransactionAnchorMode, TransactionAuth,
        TransactionContractCall, TransactionPayload, TransactionPostConditionMode,
        TransactionSpendingCondition, TransactionVersion,
    },
    core::{CHAIN_ID_MAINNET, CHAIN_ID_TESTNET},
    types::{
        chainstate::{StacksAddress, StacksPrivateKey, StacksPublicKey},
        Address,
    },
    vm::{
        errors::RuntimeErrorType,
        types::{ASCIIData, StacksAddressExtensions},
        ClarityName, ContractName, Value,
    },
};

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum Error {
    ///Error due to invalid configuration values
    #[error("{0}")]
    ConfigError(String),
    ///Error occured while signing a transaction
    #[error("Failed to sign transaction: {0}")]
    SigningError(String),
    ///Error occurred due to a malformed op
    #[error("{0}")]
    MalformedOp(String),
    ///Error occurred at Clarity runtime
    #[error("Clarity runtime error ocurred: {0}")]
    ClarityRuntimeError(#[from] RuntimeErrorType),
}

pub struct StacksWallet {
    contract_address: StacksAddress,
    contract_name: ContractName,
    sender_key: StacksPrivateKey,
    version: TransactionVersion,
    address: StacksAddress,
    fee: u64,
}

impl StacksWallet {
    pub fn new(
        contract: String,
        sender_key: &str,
        version: TransactionVersion,
        fee: u64,
    ) -> Result<Self, Error> {
        let sender_key = StacksPrivateKey::from_hex(sender_key)
            .map_err(|e| Error::ConfigError(e.to_string()))?;

        let pk = StacksPublicKey::from_private(&sender_key);

        let address = StacksAddress::from_public_keys(
            address_version(&version),
            &AddressHashMode::SerializeP2PKH,
            1,
            &vec![pk],
        )
        .ok_or(Error::ConfigError(
            "Failed to generate stacks address from private key.".to_string(),
        ))?;

        let contract_info: Vec<&str> = contract.split('.').collect();
        if contract_info.len() != 2 {
            return Err(Error::ConfigError(
                "Invalid sBTC contract. Expected a period seperated contract address and contract name."
            .to_string()));
        }
        let contract_address = contract_info[0];
        let contract_name = contract_info[1].to_owned();

        let contract_address = StacksAddress::from_string(contract_address).ok_or(
            Error::ConfigError("Invalid sBTC contract address.".to_string()),
        )?;
        let contract_name = ContractName::try_from(contract_name)
            .map_err(|_| Error::ConfigError("Invalid sBTC contract name.".to_string()))?;
        Ok(Self {
            contract_address,
            contract_name,
            sender_key,
            version,
            address,
            fee,
        })
    }

    fn build_transaction_signed(
        &self,
        function_name: impl Into<String>,
        function_args: Vec<Value>,
        nonce: u64,
    ) -> Result<StacksTransaction, Error> {
        // First build an unsigned transaction
        let unsigned_tx = self.build_transaction_unsigned(function_name, function_args, nonce)?;

        // Do the signing
        let mut tx_signer = StacksTransactionSigner::new(&unsigned_tx);
        tx_signer
            .sign_origin(&self.sender_key)
            .map_err(|e| Error::SigningError(e.to_string()))?;

        // Retrieve the signed transaction from the signer
        let signed_tx = tx_signer.get_tx().ok_or(Error::SigningError(
            "Unable to retrieve signed transaction from the signer.".to_string(),
        ))?;
        Ok(signed_tx)
    }

    fn build_transaction_unsigned(
        &self,
        function_name: impl Into<String>,
        function_args: Vec<Value>,
        nonce: u64,
    ) -> Result<StacksTransaction, Error> {
        // First build the payload from the provided function and its arguments
        let payload = self.build_transaction_payload(function_name, function_args)?;

        // Next build the authorization from the provided sender key
        let public_key = StacksPublicKey::from_private(&self.sender_key);
        let mut spending_condition = TransactionSpendingCondition::new_singlesig_p2pkh(public_key)
            .ok_or_else(|| {
                Error::SigningError(
                    "Failed to create transaction spending condition from provided sender_key."
                        .to_string(),
                )
            })?;
        spending_condition.set_nonce(nonce);
        spending_condition.set_tx_fee(self.fee);
        let auth = TransactionAuth::Standard(spending_condition);

        // Viola! We have an unsigned transaction
        let mut unsigned_tx = StacksTransaction::new(self.version, auth, payload);
        unsigned_tx.anchor_mode = TransactionAnchorMode::Any;
        unsigned_tx.post_condition_mode = TransactionPostConditionMode::Allow;
        unsigned_tx.chain_id = if self.version == TransactionVersion::Testnet {
            CHAIN_ID_TESTNET
        } else {
            CHAIN_ID_MAINNET
        };

        Ok(unsigned_tx)
    }

    fn build_transaction_payload(
        &self,
        function_name: impl Into<String>,
        function_args: Vec<Value>,
    ) -> Result<TransactionPayload, RuntimeErrorType> {
        let function_name = ClarityName::try_from(function_name.into())?;
        let payload = TransactionContractCall {
            address: self.contract_address,
            contract_name: self.contract_name.clone(),
            function_name,
            function_args,
        };
        Ok(payload.into())
    }
}

impl StacksWalletTrait for StacksWallet {
    fn build_mint_transaction(
        &self,
        op: &PegInOp,
        nonce: u64,
    ) -> Result<StacksTransaction, PegWalletError> {
        let function_name = "mint!";

        // Build the function arguments
        let amount = Value::UInt(op.amount.into());
        let principal = Value::from(op.recipient.clone());
        //Note that this tx_id is only used to print info in the contract call.
        let tx_id = Value::from(ASCIIData {
            data: op.txid.to_string().as_bytes().to_vec(),
        });
        let function_args: Vec<Value> = vec![amount, principal, tx_id];
        let tx = self.build_transaction_signed(function_name, function_args, nonce)?;
        Ok(tx)
    }

    fn build_burn_transaction(
        &self,
        op: &PegOutRequestOp,
        nonce: u64,
    ) -> Result<StacksTransaction, PegWalletError> {
        let function_name = "burn!";

        // Build the function arguments
        let amount = Value::UInt(op.amount.into());
        // Retrieve the stacks address to burn from
        let address = op
            .stx_address(address_version(&self.version))
            .map_err(|_| {
                Error::MalformedOp(
                    "Failed to recover stx address from peg-out request op.".to_string(),
                )
            })?;
        let principal_data = address.to_account_principal();
        let principal = Value::Principal(principal_data);
        //Note that this tx_id is only used to print info inside the contract call.
        let tx_id = Value::from(ASCIIData {
            data: op.txid.to_string().as_bytes().to_vec(),
        });
        let function_args: Vec<Value> = vec![amount, principal, tx_id];

        let tx = self.build_transaction_signed(function_name, function_args, nonce)?;
        Ok(tx)
    }

    fn build_set_btc_address_transaction(
        &self,
        address: &BitcoinAddress,
        nonce: u64,
    ) -> Result<StacksTransaction, PegWalletError> {
        let function_name = "set-bitcoin-wallet-address";
        // Build the function arguments
        let address = Value::from(ASCIIData {
            data: address.to_string().into_bytes(),
        });
        let function_args = vec![address];
        let tx = self.build_transaction_signed(function_name, function_args, nonce)?;
        Ok(tx)
    }

    fn address(&self) -> &StacksAddress {
        &self.address
    }
}

fn address_version(version: &TransactionVersion) -> u8 {
    match version {
        TransactionVersion::Mainnet => C32_ADDRESS_VERSION_MAINNET_SINGLESIG,
        TransactionVersion::Testnet => C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        peg_wallet::StacksWallet as StacksWalletTrait,
        stacks_wallet::StacksWallet,
        util::test::{build_peg_out_request_op, PRIVATE_KEY_HEX, PUBLIC_KEY_HEX},
    };
    use bitcoin::{secp256k1::Secp256k1, XOnlyPublicKey};
    use blockstack_lib::{
        address::C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
        burnchains::Txid,
        chainstate::{
            burn::operations::{PegInOp, PegOutRequestOp},
            stacks::{address::PoxAddress, TransactionVersion},
        },
        types::chainstate::{BurnchainHeaderHash, StacksAddress},
        util::{hash::Hash160, secp256k1::MessageSignature},
        vm::types::{PrincipalData, StandardPrincipalData},
    };
    use rand::Rng;
    use std::str::FromStr;

    fn pox_address() -> PoxAddress {
        PoxAddress::Standard(
            StacksAddress::new(
                C32_ADDRESS_VERSION_TESTNET_SINGLESIG,
                Hash160::from_data(&rand::thread_rng().gen::<[u8; 20]>()),
            ),
            None,
        )
    }

    fn stacks_wallet() -> StacksWallet {
        StacksWallet::new(
            "SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE.sbtc-alpha".to_string(),
            PRIVATE_KEY_HEX,
            TransactionVersion::Testnet,
            10,
        )
        .expect("Failed to construct a stacks wallet for testing.")
    }

    #[test]
    fn build_mint_transaction_test() {
        let p = PegInOp {
            recipient: PrincipalData::Standard(StandardPrincipalData(0, [0u8; 20])),
            peg_wallet_address: pox_address(),
            amount: 55155,
            memo: Vec::default(),
            txid: Txid([0u8; 32]),
            vtxindex: 0,
            block_height: 0,
            burn_header_hash: BurnchainHeaderHash([0; 32]),
        };
        let wallet = stacks_wallet();
        let tx = wallet
            .build_mint_transaction(&p, 0)
            .expect("Failed to construct mint transaction.");
        tx.verify()
            .expect("build_mint_transaction generated a transaction with an invalid signature");
    }

    #[test]
    fn build_burn_transaction_test() {
        let wallet = stacks_wallet();
        let op = build_peg_out_request_op(PRIVATE_KEY_HEX, 10, 1, 3);
        let tx = wallet
            .build_burn_transaction(&op, 0)
            .expect("Failed to construct burn transaction.");
        tx.verify()
            .expect("build_burn_transaction generated a transaction with an invalid signature.");
    }

    #[test]
    fn invalid_burn_op_test() {
        let wallet = stacks_wallet();
        // Construct an invalid peg-out request op.
        let op = PegOutRequestOp {
            amount: 1000,
            recipient: pox_address(),
            signature: MessageSignature([0x00; 65]),
            peg_wallet_address: pox_address(),
            fulfillment_fee: 0,
            memo: vec![],
            txid: Txid([0x04; 32]),
            vtxindex: 0,
            block_height: 0,
            burn_header_hash: BurnchainHeaderHash([0x00; 32]),
        };
        assert_eq!(
            wallet
                .build_burn_transaction(&op, 0)
                .err()
                .unwrap()
                .to_string(),
            "Stacks Wallet Error: Failed to recover stx address from peg-out request op."
        );
    }

    #[test]
    fn build_set_btc_address_transaction_test() {
        let wallet = stacks_wallet();
        let internal_key = XOnlyPublicKey::from_str(PUBLIC_KEY_HEX).unwrap();
        let secp = Secp256k1::verification_only();
        let address = bitcoin::Address::p2tr(&secp, internal_key, None, bitcoin::Network::Testnet);

        let tx = wallet
            .build_set_btc_address_transaction(&address, 0)
            .expect("Failed to construct a set btc address transaction.");
        tx.verify().expect(
            "build_set_btc_address_transaction generated a transaction with an invalid signature.",
        );
    }

    #[test]
    fn stacks_wallet_invalid_config_test() {
        // Test an invalid key
        assert_eq!(
            StacksWallet::new(
                "SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE.sbtc-alpha".to_string(),
                "",
                TransactionVersion::Testnet,
                10
            )
            .err()
            .unwrap()
            .to_string(),
            "Invalid private key hex string"
        );
        // Test an invalid contract
        assert_eq!(
            StacksWallet::new(
                "SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTEsbtc-alpha".to_string(),
                PRIVATE_KEY_HEX, TransactionVersion::Testnet, 10)
                .err()
                .unwrap()
                .to_string(),
                "Invalid sBTC contract. Expected a period seperated contract address and contract name."
        );
        // Test an invalid contract address
        assert_eq!(
            StacksWallet::new(
                "SP3FBR2AGK5H9QBDH3EEN6DF8E.sbtc-alpha".to_string(),
                PRIVATE_KEY_HEX,
                TransactionVersion::Testnet,
                10
            )
            .err()
            .unwrap()
            .to_string(),
            "Invalid sBTC contract address."
        );
        // Test an invalid contract name
        assert_eq!(
            StacksWallet::new(
                "SP3FBR2AGK5H9QBDH3EEN6DF8EK8JY7RX8QJ5SVTE.12".to_string(),
                PRIVATE_KEY_HEX,
                TransactionVersion::Testnet,
                10
            )
            .err()
            .unwrap()
            .to_string(),
            "Invalid sBTC contract name."
        );
    }
}
