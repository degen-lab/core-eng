use std::path::Path;

use anyhow::{anyhow, Context, Result};
use rusqlite::OpenFlags;

use crate::cli::{BlocksArgs, Network};

pub fn burns(network: Network, db_dir: &Path, args: &BlocksArgs) -> Result<()> {
    let mode = match network {
        Network::Mainnet => "mainnet/",
        Network::Testnet => "xenon/",
    };
    let db_file = db_dir.join(mode).join("burnchain/burnchain.sqlite");
    let conn = rusqlite::Connection::open_with_flags(db_file, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .context("Could not open database connection")?;

    let mut statement = conn
        .prepare(r#"
            SELECT JSON_EXTRACT(op, "$.LeaderBlockCommit.block_height") as block_height, JSON_EXTRACT(op, "$.LeaderBlockCommit.burn_fee") as burn_fee
            FROM burnchain_db_block_ops
            ORDER BY block_height DESC
        "#,)
        .context("Could not prepare SQL statement")?;

    let mut rows = statement
        .query::<[u8; 0]>([])
        .context("Could not execute query")?;

    let mut height_fee_pairs: Vec<(u64, u64)> = vec![];

    while let Some(row) = rows.next().context("Could not get row")? {
        let Some(height) = row
            .get::<_, Option<i64>>(0)
            .context("Could not get block height from row")?
        else {
            continue;
        };
        let Some(fee) = row
            .get::<_, Option<i64>>(1)
            .context("Could not get burn fee from row")?
        else {
            continue;
        };

        height_fee_pairs.push((height as u64, fee as u64));
    }

    if height_fee_pairs.is_empty() {
        return Err(anyhow!("Query returned no data"));
    }

    let last_block = height_fee_pairs.first().unwrap().0;
    let cutoff_block = last_block - args.blocks;

    let mut burn_fees: Vec<u64> = height_fee_pairs
        .into_iter()
        .filter(|(height, _)| *height >= cutoff_block)
        .map(|(_, fee)| fee)
        .collect();

    burn_fees.sort();

    println!(
        "Burn fee stats for last {} blocks: min={} max={} mean={} avg={}",
        args.blocks,
        burn_fees.first().unwrap(),
        burn_fees.last().unwrap(),
        burn_fees[burn_fees.len() / 2],
        burn_fees.iter().sum::<u64>() / burn_fees.len() as u64
    );

    Ok(())
}
