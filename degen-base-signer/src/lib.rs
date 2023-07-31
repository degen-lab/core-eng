pub mod config;
pub mod logging;
pub mod net;
pub mod signer;
pub mod signing_round;
pub mod state_machine;
pub mod util;
pub mod bitcoin_node;
pub mod bitcoin_wallet;
pub mod stacks_wallet;
pub mod stacks_node;
pub mod peg_wallet;
pub mod peg_queue;
pub mod util_versioning;


// set via _compile-time_ envars
const GIT_BRANCH: Option<&'static str> = option_env!("GIT_BRANCH");
const GIT_COMMIT: Option<&'static str> = option_env!("GIT_COMMIT");

#[cfg(debug_assertions)]
const BUILD_TYPE: &str = "debug";
#[cfg(not(debug_assertions))]
const BUILD_TYPE: &'static str = "release";

pub fn version() -> String {
    format!(
        "frost-signer {} {} {}",
        BUILD_TYPE,
        GIT_BRANCH.unwrap_or(""),
        GIT_COMMIT.unwrap_or("")
    )
}
