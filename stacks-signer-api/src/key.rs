use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, ToSchema, ToResponse)]
/// A key is a public key of a delegator signer.
pub struct Key {
    /// The signer's ID.
    pub signer_id: i64,
    /// The user's ID.
    pub user_id: i64,
    /// The public key of the signer.
    pub key: String,
}
