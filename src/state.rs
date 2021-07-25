use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};

use crate::error::ExchangeError::InvalidInstruction;

impl MoneylineMarketOutcome {
    pub fn unpack(input: &u8) -> Result<Self, ProgramError> {
        Ok(match input {
            0 => Self::MarketSide0Won,
            1 => Self::MarketSide1Won,
            2 => Self::MarketSide2Won,
            3 => Self::NotYetCommenced,
            4 => Self::Commenced,
            _ => return Err(InvalidInstruction.into()),
        })
    }

    pub fn pack(&self) -> u8 {
        match *self {
            MoneylineMarketOutcome::MarketSide0Won => 0,
            MoneylineMarketOutcome::MarketSide1Won => 1,
            MoneylineMarketOutcome::MarketSide2Won => 2,
            MoneylineMarketOutcome::NotYetCommenced => 3,
            MoneylineMarketOutcome::Commenced => 4,
        }
    }
}
impl From<MoneylineMarketOutcome> for &str {
    fn from(val: MoneylineMarketOutcome) -> Self {
        match val {
            MoneylineMarketOutcome::MarketSide0Won => "Market side 0 won",
            MoneylineMarketOutcome::MarketSide1Won => "Market side 1 won",
            MoneylineMarketOutcome::MarketSide2Won => "Market side 2 won",
            MoneylineMarketOutcome::NotYetCommenced => "Not yet commenced",
            MoneylineMarketOutcome::Commenced => "Commenced",
        }
    }
}

pub struct Market {
    pub is_initialized: bool,
    pub market_sides: [MarketSide; 3],
    pub locked_liquidity: u64,
    pub result_feed: Pubkey,
    pub result: MoneylineMarketOutcome,
    /// The amount of risk the bettors have entered into the market.
    /// When the market settles, this equals to the winning sides unsettled risk and payout
    pub bettor_balance: u64,
    pub pending_bets: u64,
}

pub struct MarketSide {
    pub feed_account: Pubkey,
    pub payout: u64,
    pub risk: u64,
}

pub struct HpLiquidity {
    pub is_initialized: bool,
    pub locked_liquidity: u64,
    pub live_liquidity: u64,
    pub bettor_balance: u64,
    pub pending_bets: u64,
    pub ht_mint: Pubkey,
    pub pool_usdt: Pubkey,
    pub insurance_fund_usdt: Pubkey,
    pub divvy_foundation_proceeds_usdt: Pubkey,
    pub frozen_pool: bool,
    pub frozen_betting: bool,
}

pub struct Bet {
    pub is_initialized: bool,
    pub market: Pubkey,
    pub user_usdt_account: Pubkey,
    pub user_main_account: Pubkey,
    pub user_risk: u64,
    pub user_payout: u64,
    pub user_market_side: u8,
    pub outcome: u8,
}

#[derive(PartialEq, Clone, Copy)]
pub enum MoneylineMarketOutcome {
    MarketSide0Won,
    MarketSide1Won,
    MarketSide2Won,
    NotYetCommenced,
    Commenced,
}

impl Sealed for Market {}

impl Sealed for HpLiquidity {}

impl Sealed for Bet {}

impl IsInitialized for Market {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl IsInitialized for HpLiquidity {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl IsInitialized for Bet {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

impl Pack for Market {
    const LEN: usize = 202;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, Market::LEN];
        let (
            is_initialized,
            option_0_pubkey,
            option_0_loss,
            option_0_win,
            option_1_pubkey,
            option_1_loss,
            option_1_win,
            option_2_pubkey,
            option_2_loss,
            option_2_win,
            locked_liquidity,
            result_feed,
            result,
            user_risk,
            pending_bets,
        ) = array_refs![src, 1, 32, 8, 8, 32, 8, 8, 32, 8, 8, 8, 32, 1, 8, 8];
        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };
        Ok(Market {
            is_initialized,
            market_sides: [
                MarketSide {
                    feed_account: Pubkey::new_from_array(*option_0_pubkey),
                    payout: u64::from_le_bytes(*option_0_loss),
                    risk: u64::from_le_bytes(*option_0_win),
                },
                MarketSide {
                    feed_account: Pubkey::new_from_array(*option_1_pubkey),
                    payout: u64::from_le_bytes(*option_1_loss),
                    risk: u64::from_le_bytes(*option_1_win),
                },
                MarketSide {
                    feed_account: Pubkey::new_from_array(*option_2_pubkey),
                    payout: u64::from_le_bytes(*option_2_loss),
                    risk: u64::from_le_bytes(*option_2_win),
                },
            ],
            locked_liquidity: u64::from_le_bytes(*locked_liquidity),
            result_feed: Pubkey::new_from_array(*result_feed),
            result: MoneylineMarketOutcome::unpack(&(u8::from_le_bytes(*result))).unwrap(),
            bettor_balance: u64::from_le_bytes(*user_risk),
            pending_bets: u64::from_le_bytes(*pending_bets),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, Market::LEN];
        let (
            is_initialized_dst,
            option_0_pubkey_dst,
            option_0_loss_dst,
            option_0_win_dst,
            option_1_pubkey_dst,
            option_1_loss_dst,
            option_1_win_dst,
            option_2_pubkey_dst,
            option_2_loss_dst,
            option_2_win_dst,
            locked_liquidity_dst,
            result_feed_dst,
            result_dst,
            user_risk_dst,
            pending_bets_dst,
        ) = mut_array_refs![dst, 1, 32, 8, 8, 32, 8, 8, 32, 8, 8, 8, 32, 1, 8, 8];

        let Market {
            is_initialized,
            market_sides,
            locked_liquidity,
            result_feed,
            result,
            bettor_balance: user_risk,
            pending_bets,
        } = self;

        is_initialized_dst[0] = *is_initialized as u8;
        option_0_pubkey_dst.copy_from_slice(market_sides[0].feed_account.as_ref());
        *option_0_loss_dst = market_sides[0].payout.to_le_bytes();
        *option_0_win_dst = market_sides[0].risk.to_le_bytes();
        option_1_pubkey_dst.copy_from_slice(market_sides[1].feed_account.as_ref());
        *option_1_loss_dst = market_sides[1].payout.to_le_bytes();
        *option_1_win_dst = market_sides[1].risk.to_le_bytes();
        option_2_pubkey_dst.copy_from_slice(market_sides[2].feed_account.as_ref());
        *option_2_loss_dst = market_sides[2].payout.to_le_bytes();
        *option_2_win_dst = market_sides[2].risk.to_le_bytes();
        *locked_liquidity_dst = locked_liquidity.to_le_bytes();
        result_feed_dst.copy_from_slice(result_feed.as_ref());
        *result_dst = result.pack().to_le_bytes();
        *user_risk_dst = user_risk.to_le_bytes();
        *pending_bets_dst = pending_bets.to_le_bytes();
    }
}

impl Pack for HpLiquidity {
    const LEN: usize = 163;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, HpLiquidity::LEN];
        let (
            is_initialized,
            locked_liquidity,
            live_liquidity,
            bettor_balance,
            pending_bets,
            ht_mint,
            pool_usdt,
            insurance_fund_usdt,
            divvy_foundation_proceeds_usdt,
            frozen_pool,
            frozen_betting,
        ) = array_refs![src, 1, 8, 8, 8, 8, 32, 32, 32, 32, 1, 1];

        Ok(HpLiquidity {
            is_initialized: is_initialized[0] != 0,
            locked_liquidity: u64::from_le_bytes(*locked_liquidity),
            live_liquidity: u64::from_le_bytes(*live_liquidity),
            bettor_balance: u64::from_le_bytes(*bettor_balance),
            pending_bets: u64::from_le_bytes(*pending_bets),
            ht_mint: Pubkey::new_from_array(*ht_mint),
            pool_usdt: Pubkey::new_from_array(*pool_usdt),
            insurance_fund_usdt: Pubkey::new_from_array(*insurance_fund_usdt),
            divvy_foundation_proceeds_usdt: Pubkey::new_from_array(*divvy_foundation_proceeds_usdt),
            frozen_pool: frozen_pool[0] != 0,
            frozen_betting: frozen_betting[0] != 0,
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, HpLiquidity::LEN];
        let (
            is_initialized_dst,
            locked_liquidity_dst,
            live_liquidity_dst,
            bettor_balance_dst,
            pending_bets_dst,
            ht_mint_dst,
            pool_usdt_dst,
            insurance_fund_usdt_dst,
            divvy_foundation_proceeds_usdt_dst,
            frozen_pool_dst,
            frozen_betting_dst,
            
        ) = mut_array_refs![dst, 1, 8, 8, 8, 8, 32, 32, 32, 32, 1, 1];

        let HpLiquidity {
            is_initialized,
            locked_liquidity,
            live_liquidity,
            bettor_balance,
            pending_bets,
            ht_mint,
            pool_usdt,
            insurance_fund_usdt,
            divvy_foundation_proceeds_usdt,
            frozen_pool,
            frozen_betting,
        } = self;
        is_initialized_dst[0] = *is_initialized as u8;
        *locked_liquidity_dst = locked_liquidity.to_le_bytes();
        *live_liquidity_dst = live_liquidity.to_le_bytes();
        *bettor_balance_dst = bettor_balance.to_le_bytes();
        *pending_bets_dst = pending_bets.to_le_bytes();
        ht_mint_dst.copy_from_slice(ht_mint.as_ref());
        pool_usdt_dst.copy_from_slice(pool_usdt.as_ref());
        insurance_fund_usdt_dst.copy_from_slice(insurance_fund_usdt.as_ref());
        divvy_foundation_proceeds_usdt_dst.copy_from_slice(divvy_foundation_proceeds_usdt.as_ref());
        frozen_pool_dst[0] = *frozen_pool as u8;
        frozen_betting_dst[0] = *frozen_betting as u8;
    }
}

impl Pack for Bet {
    const LEN: usize = 115;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, Bet::LEN];
        let (
            is_initialized,
            market,
            user_usdt_account,
            user_main_account,
            user_risk,
            user_payout,
            user_market_side,
            outcome,
        ) = array_refs![src, 1, 32, 32, 32, 8, 8, 1, 1];
        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };
        Ok(Bet {
            is_initialized,
            market: Pubkey::new_from_array(*market),
            user_usdt_account: Pubkey::new_from_array(*user_usdt_account),
            user_main_account: Pubkey::new_from_array(*user_main_account),
            user_risk: u64::from_le_bytes(*user_risk),
            user_payout: u64::from_le_bytes(*user_payout),
            user_market_side: u8::from_le_bytes(*user_market_side),
            outcome: u8::from_le_bytes(*outcome),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, Bet::LEN];
        let (
            is_initialized_dst,
            market_dst,
            user_usdt_account_dst,
            user_main_account_dst,
            user_risk_dst,
            user_payout_dst,
            user_market_side_dst,
            outcome_dst,
        ) = mut_array_refs![dst, 1, 32, 32, 32, 8, 8, 1, 1];

        let Bet {
            is_initialized,
            market,
            user_usdt_account,
            user_main_account,
            user_risk,
            user_payout,
            user_market_side,
            outcome,
        } = self;

        is_initialized_dst[0] = *is_initialized as u8;
        market_dst.copy_from_slice(market.as_ref());
        user_usdt_account_dst.copy_from_slice(user_usdt_account.as_ref());
        user_main_account_dst.copy_from_slice(user_main_account.as_ref());
        *user_risk_dst = user_risk.to_le_bytes();
        *user_payout_dst = user_payout.to_le_bytes();
        *user_market_side_dst = user_market_side.to_le_bytes();
        *outcome_dst = outcome.to_le_bytes();
    }
}
