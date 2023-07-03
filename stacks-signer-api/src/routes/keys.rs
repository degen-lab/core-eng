use std::convert::Infallible;

use crate::{
    db,
    error::{ErrorCode, ErrorResponse},
    key::Key,
    routes::{json_body, paginate_items, with_pool},
};
use serde::Deserialize;
use sqlx::SqlitePool;
use utoipa::IntoParams;
use warp::{http, Filter, Reply};

#[derive(Debug, Deserialize, Clone, IntoParams)]
/// The query parameters for the get keys route.
pub struct KeysQuery {
    /// The signer's ID.
    pub signer_id: i64,
    /// The user's ID.
    pub user_id: i64,
    /// The page number.
    pub page: Option<usize>,
    /// The limit of keys per page.
    pub limit: Option<usize>,
}

/// Route for adding a delegator key.
///
/// # Params
/// * pool: SqlitePool - The reference to the Sqlite database connection pool.
///
/// # Returns
/// * impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone:
///   The Warp filter for the add_key_route endpoint for routing HTTP requests.
pub fn add_key_route(
    pool: SqlitePool,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::post()
        .and(warp::path!("v1" / "keys"))
        .and(warp::path::end())
        .and(json_body::<Key>())
        .and(with_pool(pool))
        .and_then(add_key)
}

/// Route for deleting a key.
///
/// # Params
/// * pool: SqlitePool - The reference to the Sqlite database connection pool.
///
/// # Returns
/// * impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone:
///   The Warp filter for the delete_key_route endpoint for routing HTTP requests.
pub fn delete_key_route(
    pool: SqlitePool,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::delete()
        .and(warp::path!("v1" / "keys"))
        .and(warp::path::end())
        .and(json_body::<Key>())
        .and(with_pool(pool))
        .and_then(delete_key)
}

/// Route for fetching keys.
///
/// # Params
/// * pool: SqlitePool - The reference to the Sqlite database connection pool.
///
/// # Returns
/// * impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone:
///   The Warp filter for the get_keys_route endpoint for routing HTTP requests.
pub fn get_keys_route(
    pool: SqlitePool,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    warp::get()
        .and(warp::path!("v1" / "keys"))
        .and(warp::query::<KeysQuery>())
        .and(warp::path::end())
        .and(with_pool(pool))
        .and_then(get_keys)
}

/// Add a delegator key to the Signer
#[utoipa::path(
    post,
    path = "/v1/keys/",
    request_body = Key,
    responses(
        (status = OK, description = "Key added successfully."),
        (status = INTERNAL_SERVER_ERROR, description = "Internal server error occurred.", body = ErrorResponse)
    )
)]
pub async fn add_key(key: Key, pool: SqlitePool) -> Result<Box<dyn Reply>, Infallible> {
    // Insert the key into the database
    if let Err(e) = db::keys::add_key(&pool, &key).await {
        Ok(ErrorResponse::from(e).warp_reply(http::StatusCode::INTERNAL_SERVER_ERROR))
    } else {
        Ok(Box::new(http::StatusCode::OK))
    }
}

/// Delete a given delegator key.
#[utoipa::path(
    delete,
    path = "/v1/keys/",
    request_body = Key,
    responses(
        (status = OK, description = "Key deleted successfully."),
        (status = NOT_FOUND, description = "Key not found.", body = ErrorResponse),
        (status = INTERNAL_SERVER_ERROR, description = "Internal server error occurred.", body = ErrorResponse)
    )
)]
pub async fn delete_key(key: Key, pool: SqlitePool) -> Result<Box<dyn Reply>, Infallible> {
    match db::keys::delete_key(&pool, &key).await {
        Ok(result) => {
            if result.rows_affected() == 0 {
                let error_response = ErrorResponse {
                    error: ErrorCode::KeyNotFound,
                    message: None,
                };
                return Ok(error_response.warp_reply(http::StatusCode::NOT_FOUND));
            }
            Ok(Box::new(http::StatusCode::OK))
        }
        Err(e) => Ok(ErrorResponse::from(e).warp_reply(http::StatusCode::INTERNAL_SERVER_ERROR)),
    }
}

/// Get all delegator keys for a given signer ID and user ID.
#[utoipa::path(
    get,
    path = "/v1/signers/",
    request_body = Signer,
    responses(
        (status = OK, description = "Keys retrieved successfully.", body = Vec<Key>),
        (status = INTERNAL_SERVER_ERROR, description = "Internal server error occurred.", body = ErrorResponse)
    ),
    params(KeysQuery)
)]
pub async fn get_keys(query: KeysQuery, pool: SqlitePool) -> Result<Box<dyn Reply>, Infallible> {
    if query.page == Some(0) {
        return Ok(Box::new(http::StatusCode::BAD_REQUEST));
    }
    match db::keys::get_keys(&pool, query.signer_id, query.user_id).await {
        Ok(keys) => {
            let displayed_keys = paginate_items(&keys, query.page, query.limit);
            Ok(Box::new(warp::reply::with_status(
                warp::reply::json(&displayed_keys),
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
    use warp::http::StatusCode;

    async fn init_db() -> SqlitePool {
        let pool = init_pool(None)
            .await
            .expect("Failed to initialize a new database pool.");
        for key in test_data() {
            add_key(key, pool.clone()).await.unwrap();
        }
        pool
    }

    fn test_data() -> Vec<Key> {
        vec![
            Key {
                signer_id: 1,
                user_id: 1,
                key: "key1".to_string(),
            },
            Key {
                signer_id: 1,
                user_id: 1,
                key: "key2".to_string(),
            },
            Key {
                signer_id: 1,
                user_id: 1,
                key: "key3".to_string(),
            },
            Key {
                signer_id: 1,
                user_id: 1,
                key: "key4".to_string(),
            },
            Key {
                signer_id: 1,
                user_id: 1,
                key: "key5".to_string(),
            },
            Key {
                signer_id: 10,
                user_id: 1,
                key: "key1".to_string(),
            },
        ]
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_get_keys() {
        let pool = init_db().await;

        let api = warp::test::request()
            .path("/v1/keys?signer_id=1&user_id=1")
            .method("GET")
            .header("content-type", "application/json")
            .reply(&get_keys_route(pool))
            .await;

        assert_eq!(api.status(), StatusCode::OK);
        let keys: Vec<Key> =
            serde_json::from_slice(api.body()).expect("failed to deserialize Keys");
        assert_eq!(keys, test_data()[..5]);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_add_key() {
        let pool = init_db().await;

        let new_key = Key {
            signer_id: 1,
            user_id: 1,
            key: "key6".to_string(),
        };

        let api = warp::test::request()
            .path("/v1/keys")
            .method("POST")
            .json(&new_key)
            .reply(&add_key_route(pool))
            .await;

        assert_eq!(api.status(), StatusCode::OK);
        assert_eq!(api.body(), r#""#);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_delete_key() {
        let pool = init_db().await;

        let key_to_delete = Key {
            signer_id: 1,
            user_id: 1,
            key: "key1".to_string(),
        };

        let api = warp::test::request()
            .path("/v1/keys")
            .method("DELETE")
            .header("content-type", "application/json")
            .json(&key_to_delete)
            .reply(&delete_key_route(pool))
            .await;

        assert_eq!(api.status(), StatusCode::OK);
        assert_eq!(api.body(), r#""#);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_delete_key_not_found() {
        let pool = init_db().await;

        let key_no_matching_user = Key {
            signer_id: 1,
            user_id: 2, // We don't have this user id
            key: "key1".to_string(),
        };

        let key_no_matching_key = Key {
            signer_id: 1,
            user_id: 1,
            key: "invalid key".to_string(), // We don't have this key
        };

        let key_no_matching_signer = Key {
            signer_id: 2, // We don't have this signer id
            user_id: 1,
            key: "key1".to_string(),
        };

        let keys_to_attempt = vec![
            key_no_matching_user,
            key_no_matching_key,
            key_no_matching_signer,
        ];

        for key in keys_to_attempt {
            let api = warp::test::request()
                .path("/v1/keys")
                .method("DELETE")
                .header("content-type", "application/json")
                .json(&key)
                .reply(&delete_key_route(pool.clone()))
                .await;

            assert_eq!(api.status(), StatusCode::NOT_FOUND);
            assert_eq!(
                api.body(),
                serde_json::to_string(&ErrorResponse {
                    error: ErrorCode::KeyNotFound,
                    message: None
                })
                .unwrap()
                .as_str()
            );
        }
    }
}
