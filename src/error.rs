use num_derive::FromPrimitive as DeriveFromPrimitive;
use num_traits::FromPrimitive as TraitsFromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

#[derive(Error, Debug, Copy, Clone, DeriveFromPrimitive, PartialEq, Eq)]
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
    /// Not enough liquidity
    #[error("Not enough liquidity")]
    NotEnoughLiquidity,
    #[error("Bet risk is zero")]
    BetRiskZero,

    // Initialized errors
    #[error("HP liquidity already initialized")]
    HpLiquidityAlreadyInitialized,
    #[error("Market not initialized")]
    MarketNotInitialized,
    #[error("Bet already initialized")]
    BetAlreadyInitialized,
    #[error("Feed not initialized")]
    FeedNotInitialized,

    // Assertion errors
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

impl PrintProgramError for ExchangeError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + TraitsFromPrimitive,
    {
        match self {
            ExchangeError::InvalidInstruction => msg!("Invalid Instruction"),
            ExchangeError::NotValidAuthority => msg!("Not Valid Authority"),
            ExchangeError::ExpectedAmountMismatch => msg!("Expected Amount Mismatch"),
            ExchangeError::ExpectedDataMismatch => msg!("Expected Data Mismatch"),
            ExchangeError::AmountOverflow => msg!("Amount Overflow"),
            ExchangeError::MarketAlreadySettled => msg!("Market already settled"),
            ExchangeError::MarketNotSettled => msg!("Market not settled"),
            ExchangeError::BetAlreadySettled => msg!("Bet already settled"),
            ExchangeError::NotValidResult => msg!("Not valid result"),
            ExchangeError::InvalidFeedAccount => msg!("Invalid feed account"),
            ExchangeError::NotEnoughLiquidity => msg!("Not enough liquidity"),
            ExchangeError::BetRiskZero => msg!("Bet risk is zero"),

            // Initialized errors
            ExchangeError::HpLiquidityAlreadyInitialized => {
                msg!("HP liquidity already initialized")
            }
            ExchangeError::MarketNotInitialized => msg!("Market not initialized"),
            ExchangeError::BetAlreadyInitialized => msg!("Bet already initialized"),
            ExchangeError::FeedNotInitialized => msg!("Feed not initialized"),

            // Assertion errors
            ExchangeError::MarketSideRiskUnderflow => msg!("Market side risk underflow."),
            ExchangeError::MarketSidePayoutUnderflow => msg!("Market side payout underflow."),
            ExchangeError::MarketSideRiskRemaining => {
                msg!("All bets in market settled and market side risk is positive.")
            }
            ExchangeError::MarketSidePayoutRemaining => {
                msg!("All bets in market settled and market side payout is positive.")
            }
            ExchangeError::MarketBettorBalanceRemaining => {
                msg!("All bets in market settled and market bettor balance is positive.")
            }
            ExchangeError::HousePoolBettorBalanceRemaining => {
                msg!("All bets settled and house pool bettor balance is positive.")
            }
            ExchangeError::UnexpectedAvailableLiquidity => {
                msg!("The balance in the house pool does not equal available liquidity.")
            }
        }
    }
}

impl From<ExchangeError> for ProgramError {
    fn from(e: ExchangeError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for ExchangeError {
    fn type_of() -> &'static str {
        "ExchangeError"
    }
}
