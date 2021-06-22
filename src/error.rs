use thiserror::Error;

use solana_program::program_error::ProgramError;

#[derive(Error, Debug, Copy, Clone)]
pub enum ExchangeError {
    /// Invalid instruction
    #[error("Invalid Instruction")]
    InvalidInstruction,
    /// Not Rent Exempt
    #[error("Not Rent Exempt")]
    NotRentExempt,
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
    /// Not enough liquidity
    #[error("Not enough liquidity")]
    NotEnoughLiquidity,
}

impl From<ExchangeError> for ProgramError {
    fn from(e: ExchangeError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
