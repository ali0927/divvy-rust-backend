use solana_program::program_error::ProgramError;
use std::convert::TryInto;

use crate::error::ExchangeError::InvalidInstruction;

pub enum ExchangeInstruction {
    Deposit {
        /// The amount party A expects to receive of token Y
        amount: u64,
        bump_seed: u8,
    },
    Withdraw {
        /// the amount the taker expects to be paid in the other token, as a u64 because that's the max possible supply of a token
        amount: u64,
        bump_seed: u8,
    },

    //Init Bet
    Initbet {
        amount: u64,
        odds: u64,
        market_side: u64,
    },
    //Settle Bet
    Settle {
        bump_seed: u8,
    },
    //Init Moneyline Market
    InitMoneylineMarket,
    //Settle moneyline market
    SettleMoneylineMarket,
    //This is initial setup for giving ownership of hp mint & usdt account to the contract
    Ownership {
        bump_seed: u8,
    },
}

impl ExchangeInstruction {
    /// Unpacks a byte buffer into a [ExchangeInstruction](enum.ExchangeInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        //let (bump, _rest1) = input.split_last().ok_or(InvalidInstruction)?;
        let (tag, rest) = input.split_first().ok_or(InvalidInstruction)?;
        Ok(match tag {
            0 => Self::Deposit {
                amount: Self::unpack_amount(rest)?,
                bump_seed: Self::unpack_last(rest)?,
            },
            1 => Self::Withdraw {
                amount: Self::unpack_amount(rest)?,
                bump_seed: Self::unpack_last(rest)?,
            },
            2 => Self::Initbet {
                amount: Self::unpack_amount(rest)?,
                odds: Self::unpack_odds(rest)?,
                market_side: Self::unpack_market_side(rest)?,
            },
            3 => Self::Settle {
                bump_seed: Self::unpack_last(rest)?,
            },
            4 => Self::InitMoneylineMarket,
            5 => Self::SettleMoneylineMarket,
            10 => Self::Ownership {
                bump_seed: Self::unpack_last(rest)?,
            },
            _ => return Err(InvalidInstruction.into()),
        })
    }

    fn unpack_last(input: &[u8]) -> Result<u8, ProgramError> {
        let (last, _rest) = input.split_last().ok_or(InvalidInstruction)?;
        Ok(last.clone())
    }

    fn unpack_amount(input: &[u8]) -> Result<u64, ProgramError> {
        let amount = input
            .get(..8)
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or(InvalidInstruction)?;
        Ok(amount)
    }

    fn unpack_odds(input: &[u8]) -> Result<u64, ProgramError> {
        let odds = input
            .get(8..16)
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or(InvalidInstruction)?;
        Ok(odds)
    }

    fn unpack_market_side(input: &[u8]) -> Result<u64, ProgramError> {
        let market_side = input
            .get(16..24)
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or(InvalidInstruction)?;
        Ok(market_side)
    }

    // fn unpack_result(input: &[u8]) -> Result<bool, ProgramError> {
    //     let (tag, _rest) = input.split_last().ok_or(InvalidInstruction)?;

    //     if *tag == 1 {
    //         Ok(true)
    //     } else {
    //         Ok(false)
    //     }
    // }
}
