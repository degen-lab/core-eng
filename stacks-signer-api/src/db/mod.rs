#![deny(missing_docs)]

/// Module that handles keys-related database operations
pub mod keys;
/// Module that handles signers-related database operations
pub mod signers;
/// Module that handles transaction-related database operations
pub mod transaction;
/// Module that handles vote-related database operations
pub mod vote;

use parse_display::ParseError;
use sqlx::SqlitePool;

/// Custom error type for this database module
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// Sqlx related error
    #[error("Sqlx Error: {0}")]
    SqlxError(#[from] sqlx::Error),
    /// Parse related error
    #[error("Parsing error occurred due to malformed data")]
    MalformedData(#[from] ParseError),
}
impl warp::reject::Reject for Error {}

// SQL schema for creating the `sbtc_signers` table
const SQL_SCHEMA_SIGNERS: &str = r#"
    CREATE TABLE IF NOT EXISTS sbtc_signers (
        signer_id INTEGER NOT NULL,
        user_id INTEGER NOT NULL,
        status TEXT NOT NULL,

        PRIMARY KEY(signer_id, user_id)
    );
    "#;

// SQL schema for creating the `keys` table
const SQL_SCHEMA_KEYS: &str = r#"
        CREATE TABLE IF NOT EXISTS keys (
            key TEXT NOT NULL,
            signer_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
    
            PRIMARY KEY(key, signer_id, user_id),
            FOREIGN KEY(signer_id, user_id) REFERENCES sbtc_signers(signer_id, user_id) ON DELETE CASCADE
        );
        "#;
const SQL_SCHEMA_TRANSACTIONS: &str = r#"
        CREATE TABLE transactions (
            txid TEXT PRIMARY KEY,
            transaction_kind TEXT NOT NULL,
            transaction_block_height INTEGER,
            transaction_deadline_block_height INTEGER NOT NULL,
            transaction_amount INTEGER NOT NULL,
            transaction_fees INTEGER NOT NULL,
            memo BLOB NOT NULL,
            transaction_originator_address TEXT NOT NULL,
            transaction_debit_address TEXT NOT NULL,
            transaction_credit_address TEXT NOT NULL
        );
"#;

const SQL_KEY_SIGNER_TRIGGER: &str = r#"
    CREATE TRIGGER add_default_signer
    AFTER INSERT ON keys
    FOR EACH ROW
        WHEN NOT EXISTS (SELECT 1 FROM sbtc_signers WHERE signer_id = NEW.signer_id AND user_id = NEW.user_id)
        BEGIN
            INSERT INTO sbtc_signers (signer_id, user_id, status)
            VALUES (NEW.signer_id, NEW.user_id, 'inactive');
        END;
"#;
const SQL_SCHEMA_VOTE: &str = r#"
        CREATE TABLE votes (
            txid TEXT PRIMARY KEY,
            vote_status TEXT NOT NULL,
            vote_choice TEXT,
            vote_mechanism TEXT NOT NULL,
            target_consensus INTEGER NOT NULL,
            current_consensus INTEGER NOT NULL,

            FOREIGN KEY(txid) REFERENCES transactions(txid) ON DELETE CASCADE
        );
"#;

const SQL_TRANSACTION_VOTE_TRIGGER: &str = r#"
        CREATE TRIGGER add_empty_vote
            AFTER INSERT ON transactions
            FOR EACH ROW
                WHEN NEW.txid NOT IN (SELECT txid FROM votes)
                BEGIN
                    INSERT INTO votes (
                        txid, vote_status, vote_choice, vote_mechanism, target_consensus, current_consensus
                    ) VALUES (
                        NEW.txid, 'pending', NULL, 'manual', 70, 0
                    );
                END;
"#;

/// Initialize the database pool from the given file path or in memory if none is provided.
///
/// # Params
/// * path: Option<&str> - Optional file path to the SQLite database, or None to use in-memory storage.
///
/// # Returns
/// * Result<SqlitePool, Error>: Result containing the initialized SqlitePool, or an Error if initialization failed.
pub async fn init_pool(path: Option<String>) -> Result<SqlitePool, Error> {
    let pool = match path {
        Some(path) => SqlitePool::connect(&path).await?,
        None => SqlitePool::connect("sqlite::memory:").await?,
    };
    sqlx::query(SQL_SCHEMA_SIGNERS).execute(&pool).await?;
    sqlx::query(SQL_SCHEMA_KEYS).execute(&pool).await?;
    sqlx::query(SQL_SCHEMA_TRANSACTIONS).execute(&pool).await?;
    sqlx::query(SQL_SCHEMA_VOTE).execute(&pool).await?;
    sqlx::query(SQL_KEY_SIGNER_TRIGGER).execute(&pool).await?;
    sqlx::query(SQL_TRANSACTION_VOTE_TRIGGER)
        .execute(&pool)
        .await?;
    Ok(pool)
}
