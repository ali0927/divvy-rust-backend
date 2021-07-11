use crate::{error::ExchangeError::{self, NotValidAuthority}, schema::authority, state::HpLiquidity};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_pack::Pack,
    pubkey::Pubkey,
};

pub fn process_ownership(
    accounts: &[AccountInfo],
    _bump_seed: u8,
    _program_id: &Pubkey,
) -> ProgramResult {
    msg!("Divvy program ownership");
    let accounts_iter = &mut accounts.iter();
    let initializer = next_account_info(accounts_iter)?;
    //let mint = next_account_info(accounts_iter)?;
    //let token_program = next_account_info(accounts_iter)?;
    //let pda_account = next_account_info(accounts_iter)?;
    //let hp_usdt_account = next_account_info(accounts_iter)?;
    let hp_state_account = next_account_info(accounts_iter)?;

    if initializer.key != &authority::ID {
        return Err(NotValidAuthority.into());
    }

    // This initialization function should only be concerned with initializing the hp state account,
    // and perhaps validating the various mints, token and pda accounts. Those mints and tokens
    // could be stored in the hp state account, and validated in other functions.
    // Transferring ownership of mints, and token accounts can be done in web3 
    // as that requires only the authority of the owner, the initializer in this case.
    // When the init_program.ts script transfers the hp mint and usdt pool, delete these comments.

    // msg!("Setting mint authority to PDA");
    // let set_mint_authority = spl_token::instruction::set_authority(
    //     &token_program.key,
    //     &mint.key,
    //     Some(&pda_account.key),
    //     spl_token::instruction::AuthorityType::MintTokens,
    //     &initializer.key,
    //     &[&initializer.key],
    // )?;
    // msg!("Calling the token program to transfer token mint...");
    // invoke(
    //     &set_mint_authority,
    //     &[mint.clone(), initializer.clone(), token_program.clone()],
    // )?;
    // msg!("Setting HP token account owner to PDA");
    // let owner_change_ix = spl_token::instruction::set_authority(
    //     token_program.key,
    //     hp_usdt_account.key,
    //     Some(&pda_account.key),
    //     spl_token::instruction::AuthorityType::AccountOwner,
    //     initializer.key,
    //     &[&initializer.key],
    // )?;

    // msg!("Calling the token program to transfer token account ownership...");
    // invoke(
    //     &owner_change_ix,
    //     &[
    //         hp_usdt_account.clone(),
    //         initializer.clone(),
    //         token_program.clone(),
    //     ],
    // )?;

    msg!("Initalizing HP State account");
    let mut hp_state = HpLiquidity::unpack_unchecked(&hp_state_account.data.borrow())?;
    if hp_state.is_initialized {
        return Err(ExchangeError::HpLiquidityAlreadyInitialized.into());
    }
    hp_state.is_initialized = true;
    HpLiquidity::pack(hp_state, &mut hp_state_account.data.borrow_mut())?;
    Ok(())
}
