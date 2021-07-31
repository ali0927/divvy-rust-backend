use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};


pub struct HpLiquidity {
    pub is_initialized: bool,
    pub locked_liquidity: u64,
    pub live_liquidity: u64,
    pub ht_mint: Pubkey,
    pub betting_usdt: Pubkey,
    pub pool_usdt: Pubkey,
    pub insurance_fund_usdt: Pubkey,
    pub divvy_foundation_proceeds_usdt: Pubkey,
    pub frozen_pool: bool,
}


impl IsInitialized for HpLiquidity {
    fn is_initialized(&self) -> bool {
        self.is_initialized
    }
}

// Trait Seal implemented for HpLiquidity
impl Sealed for HpLiquidity {}

impl Pack for HpLiquidity {
    const LEN: usize = 178;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, HpLiquidity::LEN];
        let (
            is_initialized,
            locked_liquidity,
            live_liquidity,
            ht_mint,
            betting_usdt,
            pool_usdt,
            insurance_fund_usdt,
            divvy_foundation_proceeds_usdt,
            frozen_pool,
        ) = array_refs![src,1, 8, 8, 32, 32, 32, 32, 32, 1];

        Ok(HpLiquidity {
            is_initialized: is_initialized[0] != 0,
            locked_liquidity: u64::from_le_bytes(*locked_liquidity),
            live_liquidity: u64::from_le_bytes(*live_liquidity),
            ht_mint: Pubkey::new_from_array(*ht_mint),
            betting_usdt: Pubkey::new_from_array(*betting_usdt),
            pool_usdt: Pubkey::new_from_array(*pool_usdt),
            insurance_fund_usdt: Pubkey::new_from_array(*insurance_fund_usdt),
            divvy_foundation_proceeds_usdt: Pubkey::new_from_array(*divvy_foundation_proceeds_usdt),
            frozen_pool: frozen_pool[0] != 0,
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, HpLiquidity::LEN];
        let (
            is_initialized_dst,
            locked_liquidity_dst,
            live_liquidity_dst,
            ht_mint_dst,
            betting_usdt_dst,
            pool_usdt_dst,
            insurance_fund_usdt_dst,
            divvy_foundation_proceeds_usdt_dst,
            frozen_pool_dst,
        ) = mut_array_refs![dst, 1, 8, 8, 32, 32, 32, 32, 32, 1];

        let HpLiquidity {
            is_initialized,
            locked_liquidity,
            live_liquidity,
            ht_mint,
            betting_usdt,
            pool_usdt,
            insurance_fund_usdt,
            divvy_foundation_proceeds_usdt,
            frozen_pool,
        } = self;
        is_initialized_dst[0] = *is_initialized as u8;
        *locked_liquidity_dst = locked_liquidity.to_le_bytes();
        *live_liquidity_dst = live_liquidity.to_le_bytes();
        ht_mint_dst.copy_from_slice(ht_mint.as_ref());
        betting_usdt_dst.copy_from_slice(betting_usdt.as_ref());
        pool_usdt_dst.copy_from_slice(pool_usdt.as_ref());
        insurance_fund_usdt_dst.copy_from_slice(insurance_fund_usdt.as_ref());
        divvy_foundation_proceeds_usdt_dst.copy_from_slice(divvy_foundation_proceeds_usdt.as_ref());
        frozen_pool_dst[0] = *frozen_pool as u8;
    }
}
