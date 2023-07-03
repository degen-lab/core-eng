use crate::{
    db::Error,
    signer::{Signer, SignerStatus},
};

use sqlx::{sqlite::SqliteQueryResult, SqlitePool};

/// Helper function for adding a signer to the database.
pub async fn add_signer(pool: &SqlitePool, signer: &Signer) -> Result<SqliteQueryResult, Error> {
    sqlx::query!(
        "INSERT OR REPLACE INTO sbtc_signers (signer_id, user_id, status) VALUES (?1, ?2, ?3)",
        signer.signer_id,
        signer.user_id,
        signer.status
    )
    .execute(pool)
    .await
    .map_err(Error::from)
}

/// Helper functio for deleting a signer from the databse.
pub async fn delete_signer(pool: &SqlitePool, signer: &Signer) -> Result<SqliteQueryResult, Error> {
    sqlx::query!(
        "DELETE FROM sbtc_signers WHERE signer_id = ?1 AND user_id = ?2",
        signer.signer_id,
        signer.user_id
    )
    .execute(pool)
    .await
    .map_err(Error::from)
}

/// Helper function for retrieving a list of signers from the database given an optional status.
pub async fn get_signers(
    pool: &SqlitePool,
    status: Option<SignerStatus>,
) -> Result<Vec<Signer>, Error> {
    if let Some(status) = status.map(|s| s.to_string()) {
        sqlx::query!(
    "SELECT signer_id, user_id, status FROM sbtc_signers WHERE status = ?1 ORDER BY signer_id ASC", status).fetch_all(pool).await
        .map_err(Error::from)?
        .iter()
        .map(|row| Ok(Signer {signer_id: row.signer_id, user_id: row.user_id, status: row.status.parse()?}))
        .collect()
    } else {
        sqlx::query!("SELECT signer_id, user_id, status FROM sbtc_signers ORDER BY signer_id ASC")
            .fetch_all(pool)
            .await
            .map_err(Error::from)?
            .iter()
            .map(|row| {
                Ok(Signer {
                    signer_id: row.signer_id,
                    user_id: row.user_id,
                    status: row.status.parse()?,
                })
            })
            .collect()
    }
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
    async fn test_add_signer() {
        let pool = init_db().await;
        let expected_signer = Signer {
            signer_id: 1,
            user_id: 1,
            status: SignerStatus::Active,
        };

        add_signer(&pool, &expected_signer)
            .await
            .expect("failed to add signer");
        let signers = get_signers(&pool, None)
            .await
            .expect("Failed to get signers");
        assert_eq!(signers.len(), 1);
        assert_eq!(expected_signer, signers[0]);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_delete_signer() {
        let pool = init_db().await;
        let signer = Signer {
            signer_id: 1,
            user_id: 1,
            status: SignerStatus::Active,
        };

        add_signer(&pool, &signer)
            .await
            .expect("failed to add signer");

        assert_eq!(
            delete_signer(&pool, &signer)
                .await
                .expect("failed to delete signer")
                .rows_affected(),
            1
        );
        let signers = get_signers(&pool, None)
            .await
            .expect("failed to get signers");
        assert_eq!(signers.len(), 0);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    #[ntest::timeout(1000)]
    async fn test_get_signers() {
        let pool = init_db().await;
        let signers_to_insert = vec![
            Signer {
                signer_id: 1,
                user_id: 1,
                status: SignerStatus::Active,
            },
            Signer {
                signer_id: 2,
                user_id: 2,
                status: SignerStatus::Active,
            },
            Signer {
                signer_id: 3,
                user_id: 3,
                status: SignerStatus::Inactive,
            },
        ];

        for signer in &signers_to_insert {
            add_signer(&pool, signer)
                .await
                .expect("failed to add signer");
        }

        let signers = get_signers(&pool, Some(SignerStatus::Active))
            .await
            .expect("failed to get signers");

        assert_eq!(signers, &signers_to_insert[0..2]);

        let signers = get_signers(&pool, None)
            .await
            .expect("failed to get signers");

        assert_eq!(signers, signers_to_insert);
    }
}
