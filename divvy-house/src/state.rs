use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};


pub struct HpLiquidity {
    pub is_initialized: bool,
    pub ht_mint: Pubkey,
    pub betting_usdt: Pubkey,
    pub pool_usdt: Pubkey,
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
    const LEN: usize = 98;
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let src = array_ref![src, 0, HpLiquidity::LEN];
        let (
            is_initialized,
            ht_mint,
            betting_usdt,
            pool_usdt,
            frozen_pool,
        ) = array_refs![src,1, 32, 32, 32, 1];

        Ok(HpLiquidity {
            is_initialized: is_initialized[0] != 0,
            ht_mint: Pubkey::new_from_array(*ht_mint),
            betting_usdt: Pubkey::new_from_array(*betting_usdt),
            pool_usdt: Pubkey::new_from_array(*pool_usdt),
            frozen_pool: frozen_pool[0] != 0,
        })
    }

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let dst = array_mut_ref![dst, 0, HpLiquidity::LEN];
        let (
            is_initialized_dst,
            ht_mint_dst,
            betting_usdt_dst,
            pool_usdt_dst,
            frozen_pool_dst,
        ) = mut_array_refs![dst, 1, 32, 32, 32, 1];

        let HpLiquidity {
            is_initialized,
            ht_mint,
            betting_usdt,
            pool_usdt,
            frozen_pool,
        } = self;
        is_initialized_dst[0] = *is_initialized as u8;
        ht_mint_dst.copy_from_slice(ht_mint.as_ref());
        betting_usdt_dst.copy_from_slice(betting_usdt.as_ref());
        pool_usdt_dst.copy_from_slice(pool_usdt.as_ref());
        frozen_pool_dst[0] = *frozen_pool as u8;
    }
}
