use parse_display::{Display, FromStr};
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

#[derive(FromStr, Display, Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
#[display(style = "lowercase")]
/// Vote options for a transaction ballot.
pub enum VoteChoice {
    /// Approve the transaction.
    Approve,
    /// Reject the transaction
    Reject,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize, ToSchema, Display, FromStr)]
#[serde(rename_all = "lowercase")]
#[display(style = "lowercase")]
/// Mechanism by which a vote was cast
pub enum VoteMechanism {
    /// The vote was cast automatically
    Auto,
    /// The vote was cast manually
    Manual,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize, ToSchema, Display, FromStr)]
#[serde(rename_all = "lowercase")]
#[display(style = "lowercase")]
/// The status of a transaction vote
pub enum VoteStatus {
    /// The vote is incomplete and pending votes
    Pending,
    /// The vote is complete and the transaction is approved
    Approved,
    /// The vote is complete and the transaction rejected
    Rejected,
    /// The vote is complete, but consensus not reached
    NoConsensus,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, ToSchema)]
/// A vote request for a transaction.
pub struct VoteRequest {
    /// The hexadecimal transaction ID.
    pub txid: String,
    /// The public key of the signer delegator
    pub signing_delegator: String,
    /// The vote choice.
    pub vote_choice: VoteChoice,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize, ToResponse, ToSchema)]
/// A response for a cast vote.
pub struct VoteResponse {
    /// The caller's vote
    pub vote_choice: VoteChoice,
    /// The vote's current status
    pub vote_tally: VoteTally,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize, ToSchema)]
/// The current vote tally for a transaction.
pub struct VoteTally {
    /// The percentage votes required for consensus
    pub target_consensus: u64,
    /// the current consensus
    pub current_consensus: u64,
    /// the vote status
    pub vote_status: VoteStatus,
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize)]
/// The current vote info for a transaction
pub struct Vote {
    /// The voted on hexadecimal transaction ID.
    pub txid: String,
    /// The vote tally.
    pub vote_tally: VoteTally,
    /// The vote choice.
    pub vote_choice: Option<VoteChoice>,
    /// The current vote mechanism of the vote choice
    pub vote_mechanism: VoteMechanism,
}
