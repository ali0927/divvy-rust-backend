use thiserror::Error;

use solana_program::program_error::ProgramError;

#[derive(Error, Debug, Copy, Clone)]
pub enum ExchangeError {
    /// Invalid instruction
    #[error("Invalid Instruction")]
    InvalidInstruction,
    /// Not Valid Authority
    #[error("Not Valid Authority")]
    NotValidAuthority,
    /// Expected Amount Mismatch
    #[error("Expected Amount Mismatch")]
    ExpectedAmountMismatch,
    /// Expected Data Mismatch
    #[error("Expected Data Mismatch")]
    ExpectedDataMismatch,
    /// Amount Overflow
    #[error("Amount Overflow")]
    AmountOverflow,
    /// Market not initialized
    #[error("Market not initialized")]
    MarketNotInitialized,
    /// Market already settled
    #[error("Market already settled")]
    MarketAlreadySettled,
    /// Market not settled
    #[error("Market not settled")]
    MarketNotSettled,
    /// Bet already settled
    #[error("Bet already settled")]
    BetAlreadySettled,
    /// Not a valid result
    #[error("Not valid result")]
    NotValidResult,
    /// Invalid feed account
    #[error("Invalid feed account")]
    InvalidFeedAccount,
    #[error("Feed not initialized")]
    FeedNotInitialized,
    /// Not enough liquidity
    #[error("Not enough liquidity")]
    NotEnoughLiquidity,
    #[error("Bet already initialized")]
    BetAlreadyInitialized,
    #[error("Market side risk underflow.")]
    MarketSideRiskUnderflow,
    #[error("Market side payout underflow.")]
    MarketSidePayoutUnderflow,
    #[error("All bets in market settled and market side risk is positive.")]
    MarketSideRiskRemaining,
    #[error("All bets in market settled and market side payout is positive.")]
    MarketSidePayoutRemaining,
    #[error("All bets in market settled and market bettor balance is positive.")]
    MarketBettorBalanceRemaining,
    #[error("All bets settled and house pool bettor balance is positive.")]
    HousePoolBettorBalanceRemaining,
    #[error("The balance in the house pool does not equal available liquidity.")]
    UnexpectedAvailableLiquidity,
}

impl From<ExchangeError> for ProgramError {
    fn from(e: ExchangeError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
