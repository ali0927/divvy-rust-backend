use error::ExchangeError;
use spl_token::state::Account as TokenAccount;
use state::{HpLiquidity};

pub mod error;
pub mod instruction;
pub mod processor;
pub mod schema;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;


//TODO fix this
fn calculate_available_liquidity(
    pool_usdt_state: &TokenAccount,
    pool_state: &HpLiquidity,
) -> Result<u64, ExchangeError> {
    let available_liquidity = pool_usdt_state
        .amount;
    return Ok(available_liquidity);
}