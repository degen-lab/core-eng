use crate::{db::Error, key::Key};

use sqlx::{sqlite::SqliteQueryResult, SqlitePool};

/// Add a key to the database.
pub async fn add_key(pool: &SqlitePool, key: &Key) -> Result<SqliteQueryResult, Error> {
    sqlx::query!(
        "INSERT OR REPLACE INTO keys (signer_id, user_id, key) VALUES (?1, ?2, ?3)",
        key.signer_id,
        key.user_id,
        key.key
    )
    .execute(pool)
    .await
    .map_err(Error::from)
}

/// Delete a key from the database
pub async fn delete_key(pool: &SqlitePool, key: &Key) -> Result<SqliteQueryResult, Error> {
    sqlx::query!(
        "DELETE FROM keys WHERE signer_id = ?1 AND user_id = ?2 AND key = ?3",
        key.signer_id,
        key.user_id,
        key.key,
    )
    .execute(pool)
    .await
    .map_err(Error::from)
}

/// Helper function for retrieving a list of keys for the given signer ID and user ID from the database.
pub async fn get_keys(pool: &SqlitePool, signer_id: i64, user_id: i64) -> Result<Vec<Key>, Error> {
    Ok(sqlx::query!(
        "SELECT * FROM keys WHERE signer_id = ?1 AND user_id = ?2",
        signer_id,
        user_id,
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| Key {
        signer_id: row.signer_id,
        user_id: row.user_id,
        key: row.key,
    })
    .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::init_pool;

    async fn init_db() -> SqlitePool {
        init_pool(None)
            .await
            .expect("Failed to initialize a new database pool.")
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_add_key() {
        let pool = init_db().await;
        let expected_key = Key {
            signer_id: 1,
            user_id: 1,
            key: "key".to_string(),
        };

        add_key(&pool, &expected_key)
            .await
            .expect("failed to add key");
        let keys = get_keys(&pool, expected_key.signer_id, expected_key.user_id)
            .await
            .expect("failed to get keys");
        assert_eq!(keys.len(), 1);
        assert_eq!(expected_key, keys[0]);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_delete_key() {
        let pool = init_db().await;
        let key = Key {
            signer_id: 1,
            user_id: 1,
            key: "key".to_string(),
        };

        add_key(&pool, &key).await.expect("failed to add key");

        delete_key(&pool, &key).await.expect("failed to delete key");
        let keys = get_keys(&pool, key.signer_id, key.user_id)
            .await
            .expect("failed to get keys");
        assert!(keys.is_empty());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_get_keys() {
        let pool = init_db().await;
        let keys_to_insert = vec![
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
                signer_id: 2,
                user_id: 1,
                key: "key3".to_string(),
            },
            Key {
                signer_id: 2,
                user_id: 1,
                key: "key1".to_string(),
            },
        ];

        for key in &keys_to_insert {
            add_key(&pool, key).await.expect("failed to add key");
        }

        let keys = get_keys(&pool, 1, 1).await.expect("failed to get keys");

        assert_eq!(keys.len(), 2);
        assert_eq!(keys, keys_to_insert[0..2]);

        let keys = get_keys(&pool, 2, 1).await.expect("failed to get keys");
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&keys_to_insert[2]));
        assert!(keys.contains(&keys_to_insert[3]));
    }
}
