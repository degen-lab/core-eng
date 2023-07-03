use parse_display::{Display, FromStr};
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

use crate::vote::{VoteChoice, VoteMechanism, VoteTally};

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, ToSchema)]
/// The address of either a credit or debit transaction
pub enum TransactionAddress {
    /// A Bitcoin address
    Bitcoin(String),
    /// A Stacks address
    Stacks(String),
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, ToSchema, Display, FromStr)]
#[serde(rename_all = "lowercase")]
#[display(style = "lowercase")]
/// The type of transaction being requested
pub enum TransactionKind {
    /// A BTC to sBTC deposit reveal transaction
    DepositReveal,
    /// An sBTC to BTC withdrawal transaction
    WithdrawalReveal,
    /// A BTC withdrawal Fullfill transaction
    WithdrawalFulfill,
    /// A sBTC wallet handoff transaction
    WalletHandoff,
}

impl Default for Transaction {
    fn default() -> Self {
        Self {
            txid: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            transaction_kind: TransactionKind::DepositReveal,
            transaction_block_height: Default::default(),
            transaction_deadline_block_height: Default::default(),
            transaction_amount: Default::default(),
            transaction_fees: Default::default(),
            memo: Default::default(),
            transaction_originator_address: TransactionAddress::Bitcoin("originator".to_string()),
            transaction_debit_address: TransactionAddress::Bitcoin(
                "escrow bitcoin wallet".to_string(),
            ),
            transaction_credit_address: TransactionAddress::Stacks(
                "sBTC protocol address".to_string(),
            ),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, ToSchema)]
/// A transaction
pub struct Transaction {
    /// The hexadecimal bitcoin transaction ID
    pub txid: String,
    /// The type of transaction being voted on
    pub transaction_kind: TransactionKind,
    /// The height of the Bitcoin block that mined the commit transaction.
    pub transaction_block_height: Option<u64>,
    /// The height of the Bitcoin block at which a vote is due
    pub transaction_deadline_block_height: u64,
    /// The amount of sats in the transaction
    pub transaction_amount: u64,
    /// The amount of sats in the fee subsidy
    pub transaction_fees: u64,
    /// A message from the user in the transaction.
    pub memo: Vec<u8>,
    /// Originating address
    pub transaction_originator_address: TransactionAddress,
    /// The address of the debit wallet
    pub transaction_debit_address: TransactionAddress,
    /// The address of the credit account
    pub transaction_credit_address: TransactionAddress,
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize, ToResponse, ToSchema)]
#[response(
    description = "Transaction response returns single Transaction entity and its current vote status"
)]
/// The response returned from a transaction request
pub struct TransactionResponse {
    /// The transaction
    pub transaction: Transaction,
    /// The current vote tally of the given transaction
    pub vote_tally: VoteTally,
    /// The vote choice
    pub vote_choice: Option<VoteChoice>,
    /// The vote mechanism used
    pub vote_mechanism: VoteMechanism,
}
