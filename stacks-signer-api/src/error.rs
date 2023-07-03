use crate::db::Error as DatabaseError;
use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};
use warp::{http::StatusCode, reply::json, Reply};

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, ToSchema)]
/// The error code that ocurred
pub enum ErrorCode {
    /// Database error
    DatabaseError,
    /// Signer not found
    SignerNotFound,
    /// Key not found
    KeyNotFound,
}

impl From<DatabaseError> for ErrorResponse {
    fn from(e: DatabaseError) -> Self {
        Self {
            error: ErrorCode::DatabaseError,
            message: Some(e.to_string()),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Deserialize, Serialize, ToSchema, ToResponse)]
/// An error response
pub struct ErrorResponse {
    /// The error code
    pub error: ErrorCode,
    /// A more detailed error message if available
    pub message: Option<String>,
}

impl ErrorResponse {
    /// Create new warp reply with the status code and error response
    pub fn warp_reply(&self, status: StatusCode) -> Box<dyn Reply> {
        Box::new(warp::reply::with_status(json(self), status))
    }
}
