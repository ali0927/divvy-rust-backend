use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};

pub struct Market {
    pub is_initialized: bool,
    pub options_data: [(Pubkey, u64, u64); 3],
    pub max_loss: u64,
    pub result_feed: Pubkey,
    pub result: u8,
}

pub struct HpLiquidity {
    pub is_initialized: bool,
    pub available_liquidity: u64,
}

pub struct Bet {
    pub is_initialized: bool,
    pub market: Pubkey,
    pub user_usdt_account: Pubkey,
    pub user_main_account: Pubkey,
    pub user_risk: u64,
    pub user_potential_win: u64,
    pub user_market_side: u8,
    pub outcome: u8,
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
    const LEN: usize = 186;
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
            max_loss,
            result_feed,
            result,
        ) = array_refs![src, 1, 32, 8, 8, 32, 8, 8, 32, 8, 8, 8, 32, 1];
        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };
        Ok(Market {
            is_initialized,
            options_data: [
                (
                    Pubkey::new_from_array(*option_0_pubkey),
                    u64::from_le_bytes(*option_0_loss),
                    u64::from_le_bytes(*option_0_win),
                ),
                (
                    Pubkey::new_from_array(*option_1_pubkey),
                    u64::from_le_bytes(*option_1_loss),
                    u64::from_le_bytes(*option_1_win),
                ),
                (
                    Pubkey::new_from_array(*option_2_pubkey),
                    u64::from_le_bytes(*option_2_loss),
                    u64::from_le_bytes(*option_2_win),
                ),
            ],
            max_loss: u64::from_le_bytes(*max_loss),
            result_feed: Pubkey::new_from_array(*result_feed),
            result: u8::from_le_bytes(*result),
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
            max_loss_dst,
            result_feed_dst,
            result_dst,
        ) = mut_array_refs![dst, 1, 32, 8, 8, 32, 8, 8, 32, 8, 8, 8, 32, 1];

        let Market {
            is_initialized,
            options_data,
            max_loss,
            result_feed,
            result,
        } = self;

        is_initialized_dst[0] = *is_initialized as u8;
        option_0_pubkey_dst.copy_from_slice(options_data[0].0.as_ref());
        *option_0_loss_dst = options_data[0].1.to_le_bytes();
        *option_0_win_dst = options_data[0].2.to_le_bytes();
        option_1_pubkey_dst.copy_from_slice(options_data[1].0.as_ref());
        *option_1_loss_dst = options_data[1].1.to_le_bytes();
        *option_1_win_dst = options_data[1].2.to_le_bytes();
        option_2_pubkey_dst.copy_from_slice(options_data[2].0.as_ref());
        *option_2_loss_dst = options_data[2].1.to_le_bytes();
        *option_2_win_dst = options_data[2].2.to_le_bytes();
        *max_loss_dst = max_loss.to_le_bytes();
        result_feed_dst.copy_from_slice(result_feed.as_ref());
        *result_dst = result.to_le_bytes();
    }
}

impl Pack for HpLiquidity {
    const LEN: usize = 9;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, HpLiquidity::LEN];
        let (is_initialized, available_liquidity) = array_refs![src, 1, 8];
        let is_initialized = match is_initialized {
            [0] => false,
            [1] => true,
            _ => return Err(ProgramError::InvalidAccountData),
        };
        Ok(HpLiquidity {
            is_initialized,
            available_liquidity: u64::from_le_bytes(*available_liquidity),
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, HpLiquidity::LEN];
        let (is_initialized_dst, available_liquidity_dst) = mut_array_refs![dst, 1, 8];

        let HpLiquidity {
            is_initialized,
            available_liquidity,
        } = self;
        is_initialized_dst[0] = *is_initialized as u8;
        *available_liquidity_dst = available_liquidity.to_le_bytes();
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
            user_potential_win,
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
            user_potential_win: u64::from_le_bytes(*user_potential_win),
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
            user_potential_win_dst,
            user_market_side_dst,
            outcome_dst,
        ) = mut_array_refs![dst, 1, 32, 32, 32, 8, 8, 1, 1];

        let Bet {
            is_initialized,
            market,
            user_usdt_account,
            user_main_account,
            user_risk,
            user_potential_win,
            user_market_side,
            outcome,
        } = self;

        is_initialized_dst[0] = *is_initialized as u8;
        market_dst.copy_from_slice(market.as_ref());
        user_usdt_account_dst.copy_from_slice(user_usdt_account.as_ref());
        user_main_account_dst.copy_from_slice(user_main_account.as_ref());
        *user_risk_dst = user_risk.to_le_bytes();
        *user_potential_win_dst = user_potential_win.to_le_bytes();
        *user_market_side_dst = user_market_side.to_le_bytes();
        *outcome_dst = outcome.to_le_bytes();
    }
}
