use parse_display::{Display, FromStr};
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

#[derive(
    Debug,
    PartialEq,
    Eq,
    Clone,
    Copy,
    Deserialize,
    Serialize,
    sqlx::Type,
    Display,
    FromStr,
    ToSchema,
)]
#[sqlx(rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
#[display(style = "lowercase")]
/// The current status of a signer.
pub enum SignerStatus {
    /// The signer is active and can be used to sign transactions.
    // This implies that the signer has successfully registered its key and is ready to sign.
    Active,
    /// The signer is inactive and not available to sign transactions.
    // This implies that the singer has either not successfully reigstered its key or has
    // removed itself from the registered signers and is not participating in signing rounds.
    Inactive,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Deserialize, Serialize, ToResponse, ToSchema)]
/// A signer is a user that can participate in a signing round to sign transactions.
pub struct Signer {
    /// The signer's ID.
    pub signer_id: i64,
    /// The user's ID.
    pub user_id: i64,
    /// The current status of the signer.
    pub status: SignerStatus,
}
