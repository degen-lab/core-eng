use std::convert::Infallible;

use crate::{
    db,
    error::{ErrorCode, ErrorResponse},
    routes::{json_body, paginate_items, with_pool},
    signer::{Signer, SignerStatus},
};

use serde::Deserialize;
use sqlx::SqlitePool;
use utoipa::IntoParams;
use warp::Filter;
use warp::{http, Reply};

#[derive(Debug, Deserialize, Clone, IntoParams)]
/// Query parameters for the get signers route.
pub struct SignerQuery {
    /// The current status of the signers to retrieve.
    pub status: Option<SignerStatus>,
    /// The page number.
    pub page: Option<usize>,
    /// The limit of signers per page.
    pub limit: Option<usize>,
}

/// Route for adding a signer.
///
/// # Params
/// * pool: SqlitePool - The reference to the Sqlite database connection pool.
///
/// # Returns
/// * impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone:
///   The Warp filter for the add_signer_route endpoint for routing HTTP requests.
pub fn add_signer_route(
    pool: SqlitePool,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path!("v1" / "signers"))
        .and(warp::path::end())
        .and(json_body::<Signer>())
        .and(with_pool(pool))
        .and_then(add_signer)
}

/// Route for deleting a signer.
///
/// # Params
/// * pool: SqlitePool - The reference to the Sqlite database connection pool.
///
/// # Returns
/// * impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone:
///   The Warp filter for the delete_signer_route endpoint for routing HTTP requests.
pub fn delete_signer_route(
    pool: SqlitePool,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::delete()
        .and(warp::path!("v1" / "signers"))
        .and(warp::path::end())
        .and(json_body::<Signer>())
        .and(with_pool(pool))
        .and_then(delete_signer)
}

/// Route for fetching all signers.
///
/// # Params
/// * pool: SqlitePool - The reference to the Sqlite database connection pool.
///
/// # Returns
/// * impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone:
///   The Warp filter for the get_signers_route endpoint for routing HTTP requests.
// TODO: update this to only retrieve the signers for a given user
pub fn get_signers_route(
    pool: SqlitePool,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::get()
        .and(warp::path!("v1" / "signers"))
        .and(warp::query::<SignerQuery>())
        .and(warp::path::end())
        .and(with_pool(pool))
        .and_then(get_signers)
}

/// Add a new signer or update an existing signer's status.
#[utoipa::path(
    post,
    path = "/v1/signers/",
    request_body = Signer,
    responses(
        (status = OK, description = "Signer added sucessfully."),
        (status = INTERNAL_SERVER_ERROR, description = "Internal server error occurred.", body = ErrorResponse)
    )
)]
pub async fn add_signer(signer: Signer, pool: SqlitePool) -> Result<Box<dyn Reply>, Infallible> {
    if let Err(e) = db::signers::add_signer(&pool, &signer).await {
        Ok(ErrorResponse::from(e).warp_reply(http::StatusCode::INTERNAL_SERVER_ERROR))
    } else {
        Ok(Box::new(http::StatusCode::OK))
    }
}

/// Delete a given signer.
#[utoipa::path(
    delete,
    path = "/v1/signers/",
    request_body = Signer,
    responses(
        (status = OK, description = "Signer deleted sucessfully."),
        (status = NOT_FOUND, description = "Signer not found.", body = ErrorResponse),
        (status = INTERNAL_SERVER_ERROR, description = "Internal server error occurred.", body = ErrorResponse)
    )
)]
pub async fn delete_signer(signer: Signer, pool: SqlitePool) -> Result<Box<dyn Reply>, Infallible> {
    match db::signers::delete_signer(&pool, &signer).await {
        Ok(result) => {
            if result.rows_affected() == 0 {
                let error_response = ErrorResponse {
                    error: ErrorCode::SignerNotFound,
                    message: None,
                };
                return Ok(error_response.warp_reply(http::StatusCode::NOT_FOUND));
            }
            Ok(Box::new(http::StatusCode::OK))
        }
        Err(e) => Ok(ErrorResponse::from(e).warp_reply(http::StatusCode::INTERNAL_SERVER_ERROR)),
    }
}

/// Get a list of signers, optionally filtered by status.
#[utoipa::path(
    get,
    path = "/v1/signers/",
    request_body = Signer,
    responses(
        (status = OK, description = "Signers retrieved successfully.", body = Vec<Signer>),
        (status = INTERNAL_SERVER_ERROR, description = "Internal server error occurred.", body = ErrorResponse)
    ),
    params(SignerQuery)
)]
pub async fn get_signers(
    query: SignerQuery,
    pool: SqlitePool,
) -> Result<Box<dyn Reply>, Infallible> {
    if query.page == Some(0) {
        return Ok(Box::new(http::StatusCode::BAD_REQUEST));
    }
    match db::signers::get_signers(&pool, query.status).await {
        Ok(signers) => {
            let displayed_signers = paginate_items(&signers, query.page, query.limit);
            Ok(Box::new(warp::reply::with_status(
                warp::reply::json(&displayed_signers),
                http::StatusCode::OK,
            )))
        }
        Err(e) => Ok(ErrorResponse::from(e).warp_reply(http::StatusCode::INTERNAL_SERVER_ERROR)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_pool;
    use crate::signer::Signer;
    use warp::http;

    async fn init_db() -> SqlitePool {
        let pool = init_pool(None)
            .await
            .expect("Failed to initialize a new database pool.");
        insert_test_data(pool.clone()).await;
        pool
    }

    // Insert test data into the database
    async fn insert_test_data(pool: SqlitePool) {
        db::signers::add_signer(
            &pool,
            &Signer {
                signer_id: 1,
                user_id: 2,
                status: SignerStatus::Active,
            },
        )
        .await
        .unwrap();
        db::signers::add_signer(
            &pool,
            &Signer {
                signer_id: 3,
                user_id: 4,
                status: SignerStatus::Inactive,
            },
        )
        .await
        .unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_get_signers() {
        let pool = init_db().await;

        let api = warp::test::request()
            .path("/v1/signers?status=active")
            .method("GET")
            .header("content-type", "application/json")
            .reply(&get_signers_route(pool))
            .await;
        let body = api.body();

        let signers: Vec<Signer> =
            serde_json::from_slice(body).expect("failed to deserialize Signers");
        assert_eq!(signers.len(), 1);
        assert_eq!(
            signers[0],
            Signer {
                signer_id: 1,
                user_id: 2,
                status: SignerStatus::Active,
            }
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_add_signer() {
        let pool = init_db().await;

        let new_signer = Signer {
            signer_id: 5,
            user_id: 6,
            status: SignerStatus::Active,
        };

        let api = warp::test::request()
            .path("/v1/signers")
            .method("POST")
            .json(&new_signer)
            .reply(&add_signer_route(pool))
            .await;

        assert_eq!(api.status(), http::StatusCode::OK);
        assert_eq!(api.body(), r#""#);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_delete_signer() {
        let pool = init_db().await;

        let signer_to_delete = Signer {
            signer_id: 1,
            user_id: 2,
            status: SignerStatus::Active,
        };

        let api = warp::test::request()
            .path("/v1/signers")
            .method("DELETE")
            .header("content-type", "application/json")
            .json(&signer_to_delete)
            .reply(&delete_signer_route(pool))
            .await;

        assert_eq!(api.status(), http::StatusCode::OK);
        assert_eq!(api.body(), r#""#);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_delete_signer_not_found() {
        let pool = init_db().await;

        let signer_to_delete = Signer {
            signer_id: 5,
            user_id: 2,
            status: SignerStatus::Active,
        };

        let api = warp::test::request()
            .path("/v1/signers")
            .method("DELETE")
            .header("content-type", "application/json")
            .json(&signer_to_delete)
            .reply(&delete_signer_route(pool))
            .await;

        assert_eq!(api.status(), http::StatusCode::NOT_FOUND);
        assert_eq!(
            api.body(),
            serde_json::to_string(&ErrorResponse {
                error: ErrorCode::SignerNotFound,
                message: None
            })
            .unwrap()
            .as_str()
        );
    }
}
