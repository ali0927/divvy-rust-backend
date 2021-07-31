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

fn calculate_available_liquidity(
    pool_usdt_state: &TokenAccount,
    pool_state: &HpLiquidity,
) -> Result<u64, ExchangeError> {
    let available_liquidity = pool_usdt_state
        .amount
        .checked_sub(pool_state.locked_liquidity)
        .ok_or(ExchangeError::AmountOverflow)?
        .checked_sub(pool_state.live_liquidity)
        .ok_or(ExchangeError::AmountOverflow)?;
    return Ok(available_liquidity);
}