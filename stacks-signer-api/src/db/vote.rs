use sqlx::{Row, SqlitePool};

use crate::{db::Error, vote::Vote};

// SQL queries used for performing various operations on the "votes" table.
const SQL_INSERT_VOTE: &str = r#"INSERT OR REPLACE INTO votes (
    txid, vote_status, vote_choice, vote_mechanism, target_consensus, current_consensus
) VALUES (?, ?, ?, ?, ?, ?);"#;
const SQL_SELECT_VOTE_BY_ID: &str = r#"SELECT * FROM votes WHERE txid = ?"#;

/// Add a given vote to the database.
///
/// # Params
/// * pool: SqlitePool - The reference to the SQLite database connection pool.
/// * vote: Vote - The vote object to add to the database.
///
/// # Returns
/// * Result<(), Error>: The result of the database operation.
pub async fn add_vote(vote: &Vote, pool: &SqlitePool) -> Result<(), Error> {
    sqlx::query(SQL_INSERT_VOTE)
        .bind(&vote.txid)
        .bind(vote.vote_tally.vote_status.to_string())
        .bind(vote.vote_choice.map(|choice| choice.to_string()))
        .bind(vote.vote_mechanism.to_string())
        .bind(vote.vote_tally.target_consensus as i64)
        .bind(vote.vote_tally.current_consensus as i64)
        .execute(pool)
        .await?;
    Ok(())
}

/// Get a vote with a specific transaction ID from the database.
///
/// # Params
/// * txid: String - The transaction ID to search for.
/// * pool: SqlitePool - The reference to the SQLite database connection pool.
///
/// # Returns
/// * Result<Vote>: The vote found in the database.
pub async fn get_vote_by_id(txid: &str, pool: &SqlitePool) -> Result<Vote, Error> {
    let row = sqlx::query(SQL_SELECT_VOTE_BY_ID)
        .bind(txid)
        .fetch_one(pool)
        .await?;
    let txid: String = row.try_get("txid")?;

    let vote_status: String = row.try_get("vote_status")?;
    let vote_status = vote_status.parse()?;

    let vote_choice: Option<String> = row.try_get("vote_choice")?;
    let vote_choice = if let Some(vote_choice) = vote_choice {
        Some(vote_choice.parse()?)
    } else {
        None
    };

    let vote_mechanism: String = row.try_get("vote_mechanism")?;
    let vote_mechanism = vote_mechanism.parse()?;

    let target_consensus: i64 = row.try_get("target_consensus")?;
    let current_consensus: i64 = row.try_get("current_consensus")?;

    let vote = Vote {
        txid,
        vote_tally: crate::vote::VoteTally {
            vote_status,
            target_consensus: target_consensus as u64,
            current_consensus: current_consensus as u64,
        },
        vote_choice,
        vote_mechanism,
    };
    Ok(vote)
}
