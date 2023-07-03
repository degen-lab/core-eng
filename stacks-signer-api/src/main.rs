use clap::Parser;
use rand::Rng;
use sqlx::SqlitePool;
use stacks_signer_api::{
    db::{self, transaction::add_transaction, vote::add_vote},
    error::{ErrorCode, ErrorResponse},
    key::Key,
    routes::all_routes,
    signer::{Signer, SignerStatus},
    transaction::{Transaction, TransactionAddress, TransactionKind, TransactionResponse},
    vote::{Vote, VoteChoice, VoteMechanism, VoteRequest, VoteResponse, VoteStatus, VoteTally},
};
use std::{
    env,
    net::{IpAddr, SocketAddr},
    path::Path,
    sync::Arc,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use utoipa::OpenApi;
use utoipa_swagger_ui::Config;
use warp::{
    http::Uri,
    hyper::{Response, StatusCode},
    path::{FullPath, Tail},
    Filter, Rejection, Reply,
};

#[derive(OpenApi)]
#[openapi(
    paths(
        stacks_signer_api::routes::transaction::get_transaction_by_id,
        stacks_signer_api::routes::transaction::get_transactions,
        stacks_signer_api::routes::vote::vote,
        stacks_signer_api::routes::signers::get_signers,
        stacks_signer_api::routes::signers::add_signer,
        stacks_signer_api::routes::signers::delete_signer,
        stacks_signer_api::routes::keys::add_key,
        stacks_signer_api::routes::keys::delete_key,
        stacks_signer_api::routes::keys::get_keys,
    ),
    components(
        schemas(
            Transaction,
            TransactionAddress,
            TransactionKind,
            TransactionResponse,
            VoteResponse,
            VoteChoice,
            VoteMechanism,
            VoteRequest,
            VoteStatus,
            VoteTally,
            Signer,
            ErrorCode,
            ErrorResponse,
            SignerStatus,
            Key,
        ),
        responses(TransactionResponse, VoteResponse, Signer, ErrorResponse, Key)
    )
)]
struct ApiDoc;

pub fn initiate_tracing_subscriber() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}

/// The available CLI subcommands
#[derive(clap::Subcommand, Debug, Clone)]
enum Command {
    Docs(DocsArgs),
    Swagger(SwaggerArgs),
    Simulator(SimulatorArgs),
    Run(RunArgs),
}

#[derive(Parser, Debug, Clone)]
struct ServerArgs {
    /// Port to run API server on
    #[arg(short, long, default_value = "3030")]
    pub port: u16,
    /// Address to run API server on
    #[arg(short, long, default_value = "0.0.0.0")]
    pub address: IpAddr,
}

#[derive(Parser, Debug, Clone)]
struct DocsArgs {
    //Output file to save docs to. Prints to stdout if not provided
    #[arg(long, short)]
    pub output: Option<String>,
}

#[derive(Parser, Debug, Clone)]
struct SwaggerArgs {
    /// Path of hosted open api doc file
    #[arg(long, default_value = "/api-docs.json")]
    pub path: String,
    /// Port and Address to run Swagger UI server on
    #[command(flatten)]
    pub server: ServerArgs,
}

#[derive(Parser, Debug, Clone)]
struct SimulatorArgs {
    /// Port and address to run API server on
    #[command(flatten)]
    pub server: ServerArgs,
}

#[derive(Parser, Debug, Clone)]
struct RunArgs {
    /// Port and address to run API server on
    #[command(flatten)]
    pub server: ServerArgs,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Subcommand action to take
    #[command(subcommand)]
    command: Command,
}

async fn init_pool() -> anyhow::Result<SqlitePool> {
    let _ = dotenv::dotenv();

    // Initialize the connection pool__
    db::init_pool(env::var("DATABASE_URL").ok())
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize database connection pool: {}", e))
}

/// Run the Signer API server on the provided port and address
async fn run(pool: SqlitePool, server_args: ServerArgs) -> anyhow::Result<()> {
    // Create the routes
    let routes = all_routes(pool);

    // Run the warp server
    let socket = SocketAddr::new(server_args.address, server_args.port);
    println!("Serving warp server on {}", socket);
    warp::serve(routes).run(socket).await;
    Ok(())
}

/// Generate the OpenAPI json docs and save to file or print to stdout
fn generate_docs(output: &Option<String>) -> anyhow::Result<()> {
    let docs = ApiDoc::openapi();
    let openapi_json = docs
        .to_pretty_json()
        .map_err(|e| anyhow::anyhow!("Could not generate openapi json file: {}", e.to_string()))?;
    if let Some(output_file) = output {
        std::fs::write(output_file, openapi_json)
            .map_err(|e| anyhow::anyhow!("Failed to write OpenAPI json docs to file: {}", e))?;
        return Ok(());
    }
    println!("{}", openapi_json);
    Ok(())
}

/// Run the Signer API server with a database of simulated data
async fn run_simulator(args: SimulatorArgs) -> anyhow::Result<()> {
    // Initialize the connection pool
    let pool = init_pool().await?;
    let (txs, votes) = generate_txs_votes();
    for tx in txs {
        add_transaction(&pool, &tx)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to add tx: {}", e))?;
    }
    for vote in votes {
        add_vote(&vote, &pool)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to add vote: {}", e))?;
    }

    run(pool, args.server).await
}

/// Serve the Swagger UI page on the provided address and port using the generated OpenAPI json doc
async fn run_swagger(args: &SwaggerArgs) -> anyhow::Result<()> {
    // Initialize the connection pool
    let pool = init_pool().await?;
    // Configure where we host the doc in swagger-ui
    let path_buf = Path::new(&args.path);
    let config = Arc::new(Config::from(args.path.clone()));
    let file_name = path_buf
        .file_name()
        .ok_or(anyhow::anyhow!("Invalid file path provided."))?
        .to_str()
        .ok_or(anyhow::anyhow!("Invalid file path provided."))?
        .to_string();
    let api_doc = warp::path(file_name)
        .and(warp::get())
        .map(|| warp::reply::json(&ApiDoc::openapi()));

    let swagger_ui = warp::path("swagger-ui")
        .and(warp::get())
        .and(warp::path::full())
        .and(warp::path::tail())
        .and(warp::any().map(move || config.clone()))
        .and_then(serve_swagger);

    let socket = SocketAddr::new(args.server.address, args.server.port);
    println!(
        "Serving swagger UI on http://{}:{}/swagger-ui/",
        args.server.address, args.server.port
    );

    warp::serve(api_doc.or(swagger_ui).or(all_routes(pool)))
        .run(socket)
        .await;
    Ok(())
}

async fn serve_swagger(
    full_path: FullPath,
    tail: Tail,
    config: Arc<Config<'static>>,
) -> Result<Box<dyn Reply + 'static>, Rejection> {
    if full_path.as_str() == "/swagger-ui" {
        return Ok(Box::new(warp::redirect::found(Uri::from_static(
            "/swagger-ui/",
        ))));
    }

    let path = tail.as_str();
    match utoipa_swagger_ui::serve(path, config) {
        Ok(file) => {
            if let Some(file) = file {
                Ok(Box::new(
                    Response::builder()
                        .header("Content-Type", file.content_type)
                        .body(file.bytes),
                ))
            } else {
                Ok(Box::new(StatusCode::NOT_FOUND))
            }
        }
        Err(error) => Ok(Box::new(
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(error.to_string()),
        )),
    }
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    // First enable tracing
    initiate_tracing_subscriber();

    if let Err(error) = match cli.command {
        Command::Docs(args) => generate_docs(&args.output),
        Command::Swagger(args) => run_swagger(&args).await,
        Command::Simulator(args) => run_simulator(args).await,
        Command::Run(args) => {
            // Initialize the connection pool
            match init_pool().await {
                Ok(pool) => run(pool, args.server).await,
                Err(e) => Err(e),
            }
        }
    } {
        println!("Error occurred running command: {}", error);
    }
}

/// Generate some simulated transactions for mocked backend
fn generate_txs_votes() -> (Vec<Transaction>, Vec<Vote>) {
    let mut txs = vec![];
    let mut votes = vec![];
    for i in 0..10 {
        let tx = generate_transaction(i);
        votes.push(generate_vote(tx.txid.clone()));
        txs.push(tx);
    }
    (txs, votes)
}

fn generate_vote(txid: String) -> Vote {
    let mut rng = rand::thread_rng();
    let vote_mechanism = if rng.gen_range(0..2) == 0 {
        VoteMechanism::Auto
    } else {
        VoteMechanism::Manual
    };
    let status = rng.gen_range(0..4);
    let (vote_status, current_consensus) = if status == 0 {
        (VoteStatus::Pending, rng.gen_range(1..55))
    } else if status == 1 {
        (VoteStatus::Approved, rng.gen_range(70..100))
    } else if status == 2 {
        (VoteStatus::Rejected, rng.gen_range(70..100))
    } else {
        (VoteStatus::NoConsensus, rng.gen_range(1..69))
    };
    let choice = rng.gen_range(0..3);
    let vote_choice = if vote_mechanism == VoteMechanism::Auto {
        if choice == 0 {
            Some(VoteChoice::Approve)
        } else {
            Some(VoteChoice::Reject)
        }
    } else if choice == 0 {
        Some(VoteChoice::Approve)
    } else if choice == 1 {
        Some(VoteChoice::Reject)
    } else {
        None
    };
    Vote {
        txid,
        vote_mechanism,
        vote_tally: VoteTally {
            current_consensus,
            target_consensus: 70,
            vote_status,
        },
        vote_choice,
    }
}

fn generate_transaction(i: usize) -> Transaction {
    let mut rng = rand::thread_rng();
    let rand_kind = rng.gen_range(0..4);
    let transaction_kind = if rand_kind == 0 {
        TransactionKind::DepositReveal
    } else if rand_kind == 1 {
        TransactionKind::WithdrawalReveal
    } else if rand_kind == 2 {
        TransactionKind::WithdrawalFulfill
    } else {
        TransactionKind::WalletHandoff
    };
    let transaction_block_height = rng.gen();
    Transaction {
        txid: hex::encode([i as u8; 32]),
        transaction_kind,
        transaction_block_height,
        transaction_deadline_block_height: transaction_block_height.unwrap_or(0)
            + rng.gen_range(1..10),
        transaction_amount: rng.gen(),
        transaction_fees: rng.gen_range(10..1000),
        memo: vec![],
        transaction_originator_address: TransactionAddress::Bitcoin("originator".to_string()),
        transaction_debit_address: TransactionAddress::Bitcoin("escrow bitcoin wallet".to_string()),
        transaction_credit_address: TransactionAddress::Stacks("sBTC protocol address".to_string()),
    }
}
