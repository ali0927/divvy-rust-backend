use crate::{error::ExchangeError::NotValidAuthority, schema::authority, state::HpLiquidity};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::invoke,
    program_pack::Pack,
    pubkey::Pubkey,
    //sysvar::{rent::Rent, Sysvar},
};

pub fn process_ownership(
    accounts: &[AccountInfo],
    _bump_seed: u8,
    _program_id: &Pubkey,
) -> ProgramResult {
    msg!("Divvy program ownership");
    let accounts_iter = &mut accounts.iter();
    let initializer = next_account_info(accounts_iter)?;
    let mint = next_account_info(accounts_iter)?;
    let token_program = next_account_info(accounts_iter)?;
    let pda_account = next_account_info(accounts_iter)?;
    let hp_usdt_account = next_account_info(accounts_iter)?;
    let hp_state_account = next_account_info(accounts_iter)?;

    if initializer.key != &authority::ID {
        return Err(NotValidAuthority.into());
    }

    msg!("Setting mint authority to PDA");
    let set_mint_authority = spl_token::instruction::set_authority(
        &token_program.key,
        &mint.key,
        Some(&pda_account.key),
        spl_token::instruction::AuthorityType::MintTokens,
        &initializer.key,
        &[&initializer.key],
    )?;
    msg!("Calling the token program to transfer token mint...");
    invoke(
        &set_mint_authority,
        &[mint.clone(), initializer.clone(), token_program.clone()],
    )?;
    msg!("Setting HP token account for program");
    let owner_change_ix = spl_token::instruction::set_authority(
        token_program.key,
        hp_usdt_account.key,
        Some(&pda_account.key),
        spl_token::instruction::AuthorityType::AccountOwner,
        initializer.key,
        &[&initializer.key],
    )?;

    msg!("Calling the token program to transfer token account ownership...");
    invoke(
        &owner_change_ix,
        &[
            hp_usdt_account.clone(),
            initializer.clone(),
            token_program.clone(),
        ],
    )?;

    msg!("Initalizing HP State account liquidity as 0");
    let mut hp_state = HpLiquidity::unpack_unchecked(&hp_state_account.data.borrow())?;
    hp_state.available_liquidity = 0;
    HpLiquidity::pack(hp_state, &mut hp_state_account.data.borrow_mut())?;
    Ok(())
}
