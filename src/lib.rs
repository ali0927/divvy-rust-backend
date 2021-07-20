use error::ExchangeError;
use spl_token::state::Account as TokenAccount;
use state::{HpLiquidity, Market};

pub mod error;
pub mod instruction;
pub mod processor;
pub mod schema;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

fn calculate_available_liquidity(pool_usdt_state: &TokenAccount, pool_state: &HpLiquidity) -> Result<u64, ExchangeError> {
    let available_liquidity = pool_usdt_state.amount
        .checked_sub(pool_state.locked_liquidity)
        .ok_or(ExchangeError::AmountOverflow)?
        .checked_sub(pool_state.live_liquidity)
        .ok_or(ExchangeError::AmountOverflow)?
        .checked_sub(pool_state.bettor_balance)
        .ok_or(ExchangeError::AmountOverflow)?;
    return Ok(available_liquidity)
}

fn calculate_payout(odds: f64, risk: u64) -> u64 {
    if odds < 0.0 {
        return 100 / ((-odds as u64) * risk);
    } else {
        return odds as u64 * risk / 100;
    }
}

fn calculate_locked_liquidity(market_state: &Market) -> Result<u64, ExchangeError> {
    //Calculating max loss
    let mut locked_side_0 = 0u64;
    let mut locked_side_1 = 0u64;
    let mut locked_side_2 = 0u64;

    if market_state.market_sides[0].payout
        > market_state.market_sides[1]
            .risk
            .checked_add(market_state.market_sides[2].risk)
            .ok_or(ExchangeError::AmountOverflow)?
    {
        locked_side_0 = market_state.market_sides[0]
            .payout
            .checked_sub(market_state.market_sides[1].risk)
            .ok_or(ExchangeError::AmountOverflow)?
            .checked_sub(market_state.market_sides[2].risk)
            .ok_or(ExchangeError::AmountOverflow)?;
    };
    if market_state.market_sides[1].payout
        > market_state.market_sides[0]
            .risk
            .checked_add(market_state.market_sides[2].risk)
            .ok_or(ExchangeError::AmountOverflow)?
    {
        locked_side_1 = market_state.market_sides[1]
            .payout
            .checked_sub(market_state.market_sides[0].risk)
            .ok_or(ExchangeError::AmountOverflow)?
            .checked_sub(market_state.market_sides[2].risk)
            .ok_or(ExchangeError::AmountOverflow)?;
    };

    if market_state.market_sides[2].payout
        > market_state.market_sides[0]
            .risk
            .checked_add(market_state.market_sides[1].risk)
            .ok_or(ExchangeError::AmountOverflow)?
    {
        locked_side_2 = market_state.market_sides[2]
            .payout
            .checked_sub(market_state.market_sides[0].risk)
            .ok_or(ExchangeError::AmountOverflow)?
            .checked_sub(market_state.market_sides[1].risk)
            .ok_or(ExchangeError::AmountOverflow)?;
    };

    let locked_liquidity = *[locked_side_0, locked_side_1, locked_side_2]
        .iter()
        .max()
        .ok_or(ExchangeError::InvalidInstruction)?;

    return Ok(locked_liquidity);
}
