use solana_program::program_error::ProgramError;
use std::convert::TryInto;

use crate::{
    error::ExchangeError::{self, InvalidInstruction},
    state::BetType,
};

pub enum ExchangeInstruction {
    Initbet {
        risk: u64,
        odds: u64,
        market_side: u8,
    },
    SettleBet {
        bump_seed: u8,
    },
    InitMoneylineMarket {
        bet_type: BetType,
    },
    SettleMoneylineMarket {
        bump_seed: u8,
    },
    Ownership {
        bump_seed: u8,
    },
    CommenceMarket {
        hp_bump_seed: u8,
        bump_seed: u8
    },
    Freeze {
        freeze_betting: bool,
    },
}

impl ExchangeInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (tag, rest) = input.split_first().ok_or(InvalidInstruction)?;
        Ok(match tag {
            0 => Self::Initbet {
                risk: Self::unpack_amount(rest)?,
                odds: Self::unpack_odds(rest)?,
                market_side: Self::unpack_market_side(rest)?,
            },
            1 => Self::SettleBet {
                bump_seed: Self::unpack_last(rest)?,
            },
            2 => {
                let (bet_type, _rest) = rest
                    .split_first()
                    .ok_or(ExchangeError::InvalidInstruction)?;
                Self::InitMoneylineMarket {
                    bet_type: BetType::unpack(bet_type)?,
                }
            }
            3 => Self::SettleMoneylineMarket {
                bump_seed: Self::unpack_last(rest)?,
            },
            4 => Self::Ownership {
                bump_seed: Self::unpack_last(rest)?,
            },
            5 => Self::CommenceMarket {
                hp_bump_seed: Self::unpack_last(rest)?,
                bump_seed: Self::unpack_last(rest)?,
            },
            6 => {
                let (freeze_betting, _rest) = rest
                    .split_first()
                    .ok_or(ExchangeError::InvalidInstruction)?;
                Self::Freeze {
                    freeze_betting: *freeze_betting != 0,
                }
            }
            _ => return Err(InvalidInstruction.into()),
        })
    }

    // Todo: delete these 4 methods and use split_first, like in spl-token/instruction.rs
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
    fn unpack_market_side(input: &[u8]) -> Result<u8, ProgramError> {
        let market_side = input
            .get(16..17)
            .and_then(|slice| slice.try_into().ok())
            .map(u8::from_le_bytes)
            .ok_or(InvalidInstruction)?;
        Ok(market_side)
    }
}
