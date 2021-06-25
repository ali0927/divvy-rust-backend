use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    //program_pack::{IsInitialized, Pack},
    program_pack::Pack,
    pubkey::Pubkey,
    //sysvar::{rent::Rent, Sysvar},
};

//use spl_associated_token_account::get_associated_token_address;
use spl_token::instruction::{burn, mint_to, transfer};
//use spl_token::state::Account as TokenAccount;

//Switchboard dependencies
use switchboard_program::{get_aggregator, get_aggregator_result, AggregatorState, RoundResult};

use crate::{
    error::ExchangeError::{
        BetAlreadySettled, ExpectedDataMismatch, InvalidFeedAccount, InvalidInstruction,
        MarketAlreadySettled, MarketNotInitialized, MarketNotSettled, NotEnoughLiquidity,
        NotValidAuthority, NotValidResult,
    },
    instruction::ExchangeInstruction,
    schema::authority,
    state::{Bet, HpLiquidity, Market},
};

pub struct Processor;
impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = ExchangeInstruction::unpack(instruction_data)?;

        match instruction {
            ExchangeInstruction::Deposit { amount, bump_seed } => {
                msg!("Instruction: Deposit");
                Self::process_deposit(accounts, amount, bump_seed, program_id)
            }
            ExchangeInstruction::Withdraw { amount, bump_seed } => {
                msg!("Instruction: Withdraw");
                Self::process_withdraw(accounts, amount, bump_seed, program_id)
            }
            ExchangeInstruction::Initbet {
                amount,
                odds,
                market_side,
            } => {
                msg!("Instruction: Initbet");
                Self::process_initbet(accounts, amount, odds, market_side, program_id)
            }
            ExchangeInstruction::Settle { bump_seed } => {
                msg!("Instruction: Settle");
                Self::process_settle(accounts, bump_seed, program_id)
            }
            ExchangeInstruction::InitMoneylineMarket {} => {
                msg!("Instruction: Init Moneyline Market");
                Self::process_initmoneylinebet(accounts, program_id)
            }
            ExchangeInstruction::SettleMoneylineMarket {} => {
                msg!("Instruction: Settle Moneyline Market");
                Self::process_settle_moneyline_market(accounts, program_id)
            }
            ExchangeInstruction::Ownership { bump_seed } => {
                msg!("Instruction: Ownership");
                Self::process_ownership(accounts, bump_seed, program_id)
            }
        }
    }

    fn process_deposit(
        accounts: &[AccountInfo],
        amount: u64,
        bump_seed: u8,
        _program_id: &Pubkey,
    ) -> ProgramResult {
        msg!("Divvy program deposit");

        // Iterating accounts is safer then indexing
        let accounts_iter = &mut accounts.iter();

        // Get the account to say hello to
        let account = next_account_info(accounts_iter)?;
        let mint = next_account_info(accounts_iter)?;
        let token_program = next_account_info(accounts_iter)?;
        //let token_owner = next_account_info(accounts_iter)?;
        let token_account = next_account_info(accounts_iter)?;
        let pda_account = next_account_info(accounts_iter)?;
        let user_account = next_account_info(accounts_iter)?;
        let hp_usdt_account = next_account_info(accounts_iter)?;
        let hp_state_account = next_account_info(accounts_iter)?;

        msg!("Amount is {} ", amount);
        //let (_pda, bump_seed) = Pubkey::find_program_address(&[b"divvyexchange"], program_id);
        let transfer_instruction = transfer(
            &token_program.key,
            &user_account.key,
            &hp_usdt_account.key,
            &account.key,
            &[&account.key],
            amount.clone(),
        )?;
        msg!("Calling the token program to transfer tokens...");
        invoke(
            &transfer_instruction,
            &[
                user_account.clone(),
                hp_usdt_account.clone(),
                account.clone(),
                token_program.clone(),
            ],
        )?;

        msg!("Creating mint instruction");
        let mint_ix = mint_to(
            &token_program.key,
            &mint.key,
            &token_account.key,
            &pda_account.key,
            &[&pda_account.key],
            amount,
        )?;

        invoke_signed(
            &mint_ix,
            &[mint.clone(), token_account.clone(), pda_account.clone()],
            &[&[&b"divvyexchange"[..], &[bump_seed]]],
        )?;

        msg!("Adding deposit to liquidity");
        let mut hp_state = HpLiquidity::unpack_unchecked(&hp_state_account.data.borrow())?;
        hp_state.available_liquidity = hp_state.available_liquidity + amount;
        HpLiquidity::pack(hp_state, &mut hp_state_account.data.borrow_mut())?;

        Ok(())
    }

    fn process_withdraw(
        accounts: &[AccountInfo],
        amount: u64,
        bump_seed: u8,
        _program_id: &Pubkey,
    ) -> ProgramResult {
        // Iterating accounts is safer then indexing
        let accounts_iter = &mut accounts.iter();

        // Get the account to say hello to
        let account = next_account_info(accounts_iter)?;
        let mint = next_account_info(accounts_iter)?;
        let token_program = next_account_info(accounts_iter)?;
        //let token_owner = next_account_info(accounts_iter)?;
        let token_account = next_account_info(accounts_iter)?;
        let pda_account = next_account_info(accounts_iter)?;
        let user_account = next_account_info(accounts_iter)?;
        let hp_usdt_account = next_account_info(accounts_iter)?;
        let hp_state_account = next_account_info(accounts_iter)?;

        //Burn the transfers
        let burn_tx = burn(
            &token_program.key,
            &token_account.key,
            &mint.key,
            &account.key,
            &[&account.key],
            amount,
        )?;

        invoke(
            &burn_tx,
            &[
                token_program.clone(),
                token_account.clone(),
                mint.clone(),
                account.clone(),
            ],
        )?;

        //Transfer Withdraw
        let transfer_instruction = transfer(
            &token_program.key,
            &hp_usdt_account.key,
            &user_account.key,
            &pda_account.key,
            &[&pda_account.key],
            amount.clone(),
        )?;
        msg!("Calling the token program to transfer tokens...");
        invoke_signed(
            &transfer_instruction,
            &[
                hp_usdt_account.clone(),
                user_account.clone(),
                pda_account.clone(),
                token_program.clone(),
            ],
            &[&[&b"divvyexchange"[..], &[bump_seed]]],
        )?;

        msg!("Subtracting withdrawal from liquidity");
        let mut hp_state = HpLiquidity::unpack_unchecked(&hp_state_account.data.borrow())?;
        hp_state.available_liquidity = hp_state.available_liquidity - amount;
        HpLiquidity::pack(hp_state, &mut hp_state_account.data.borrow_mut())?;

        Ok(())
    }

    fn process_initbet(
        accounts: &[AccountInfo],
        amount: u64,
        odds: u64,
        market_side: usize,
        _program_id: &Pubkey,
    ) -> ProgramResult {
        msg!(
            "Divvy program initbet with amount {} and odds {}",
            amount,
            odds
        );
        let accounts_iter = &mut accounts.iter();
        let initializer = next_account_info(accounts_iter)?;
        if !initializer.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        let feed_account = next_account_info(accounts_iter)?;
        let bet_account = next_account_info(accounts_iter)?;
        let market_state_account = next_account_info(accounts_iter)?;
        let hp_state_account = next_account_info(accounts_iter)?;
        let hp_usdt_account = next_account_info(accounts_iter)?;
        let user_usdt_account = next_account_info(accounts_iter)?;
        let token_program = next_account_info(accounts_iter)?;

        msg!("Checking if data is correct");
        //Checking if market is initialized
        let mut market_state = Market::unpack_unchecked(&market_state_account.data.borrow())?;
        if !market_state.is_initialized {
            return Err(MarketNotInitialized.into());
        }
        //Checking if market is not settled yet
        if market_state.result == 3 {
            return Err(MarketAlreadySettled.into());
        }
        //Checking if feed account is right
        if market_state.options_data[market_side].0 != feed_account.key.clone() {
            return Err(InvalidFeedAccount.into());
        }

        //Getting odds from the Switchboard
        let aggregator: AggregatorState = get_aggregator(feed_account)?;
        let round_result: RoundResult = get_aggregator_result(&aggregator)?;
        let feed_odds: f64 = round_result.result.expect("Invalid feed");

        //To Do comparison of provided odds & feed odds.

        //Calculate risk
        let risk = Self::calculate_risk(feed_odds, amount);
        //See if we have available liquidity
        let mut hp_state = HpLiquidity::unpack_unchecked(&hp_state_account.data.borrow())?;
        //Calculate impact of the bet on market risk
        let current_max_loss = market_state.max_loss;
        //Add risk & gain in market
        let current_market_side_loss = market_state.options_data[market_side].1;
        let current_market_side_win = market_state.options_data[market_side].2;
        market_state.options_data[market_side].1 = current_market_side_loss + risk;
        market_state.options_data[market_side].2 = current_market_side_win + amount;

        //Calculating max loss
        let mut loss_side_0 = 0;
        if market_state.options_data[0].1
            - (market_state.options_data[1].2 + market_state.options_data[2].2)
            > 0
        {
            loss_side_0 = market_state.options_data[0].1
                - (market_state.options_data[1].2 + market_state.options_data[2].2)
        };
        let mut loss_side_1 = 0;
        if market_state.options_data[1].1
            - (market_state.options_data[0].2 + market_state.options_data[2].2)
            > 0
        {
            loss_side_1 = market_state.options_data[1].1
                - (market_state.options_data[0].2 + market_state.options_data[2].2);
        }
        let mut loss_side_2 = 0;
        if market_state.options_data[2].1
            - (market_state.options_data[0].2 + market_state.options_data[1].2)
            > 0
        {
            loss_side_2 = market_state.options_data[2].1
                - (market_state.options_data[0].2 + market_state.options_data[1].2);
        }
        let loss_arr = [loss_side_0, loss_side_1, loss_side_2];
        let new_max_loss = loss_arr.iter().max().unwrap();

        let current_hp_liquidity = hp_state.available_liquidity;
        hp_state.available_liquidity = current_hp_liquidity + current_max_loss - new_max_loss;
        if hp_state.available_liquidity >= current_hp_liquidity {
            //Transfer token from user account to hp account
            let transfer_instruction = transfer(
                &token_program.key,
                &user_usdt_account.key,
                &hp_usdt_account.key,
                &initializer.key,
                &[&initializer.key],
                amount.clone(),
            )?;
            msg!("Calling the token program to transfer tokens from user account to divvy account");
            invoke(
                &transfer_instruction,
                &[
                    user_usdt_account.clone(),
                    hp_usdt_account.clone(),
                    initializer.clone(),
                    token_program.clone(),
                ],
            )?;

            //Create a bet state
            let mut bet_state = Bet::unpack_unchecked(&bet_account.data.borrow())?;
            bet_state.is_initialized = true;
            bet_state.market = market_state_account.key.clone();
            bet_state.user_usdt_account = user_usdt_account.key.clone();
            bet_state.user_main_account = initializer.key.clone();
            bet_state.user_risk = amount;
            bet_state.user_potential_win = risk;
            bet_state.user_market_side = market_side as u8;
            bet_state.outcome = 0; //Outcome 0 as market not settled.
            Bet::pack(bet_state, &mut bet_account.data.borrow_mut())?;
            //Write the Accounts
            HpLiquidity::pack(hp_state, &mut hp_state_account.data.borrow_mut())?;
            //Write the market account
            Market::pack(market_state, &mut market_state_account.data.borrow_mut())?;

            Ok(())
        } else {
            return Err(NotEnoughLiquidity.into());
        }
    }

    fn calculate_risk(odds: f64, amount: u64) -> u64 {
        if odds < 0.0 {
            return ((odds / 100.0).round() as u64) * amount;
        } else {
            return ((100.0 / odds).round() as u64) * amount;
        }
    }

    fn process_settle(
        accounts: &[AccountInfo],
        bump_seed: u8,
        _program_id: &Pubkey,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let _initializer = next_account_info(accounts_iter)?;
        let token_program = next_account_info(accounts_iter)?;
        let market_state_account = next_account_info(accounts_iter)?;
        //let hp_state_account = next_account_info(accounts_iter)?;
        let bet_state_account = next_account_info(accounts_iter)?;
        let pda_account = next_account_info(accounts_iter)?;
        let hp_usdt_account = next_account_info(accounts_iter)?;
        let user_usdt_account = next_account_info(accounts_iter)?;
        let user_main_account = next_account_info(accounts_iter)?;

        let market_state = Market::unpack(&market_state_account.data.borrow())?;
        let mut bet_state = Bet::unpack_unchecked(&bet_state_account.data.borrow())?;

        if bet_state.market != market_state_account.key.clone() {
            return Err(ExpectedDataMismatch.into());
        }

        if market_state.result != 3 {
            return Err(MarketNotSettled.into());
        }

        if bet_state.user_usdt_account != user_usdt_account.key.clone() {
            return Err(ExpectedDataMismatch.into());
        }

        if bet_state.user_main_account != user_main_account.key.clone() {
            return Err(ExpectedDataMismatch.into());
        }

        if bet_state.outcome != 0 {
            return Err(BetAlreadySettled.into());
        }

        if bet_state.user_market_side == market_state.result {
            bet_state.outcome = 1; //User have won
            let amount_to_transfer = bet_state.user_risk + bet_state.user_potential_win;
            let transfer_instruction = transfer(
                &token_program.key,
                &hp_usdt_account.key,
                &user_usdt_account.key,
                &pda_account.key,
                &[&pda_account.key],
                amount_to_transfer,
            )?;
            msg!("Calling the token program to transfer tokens to user");
            invoke_signed(
                &transfer_instruction,
                &[
                    user_usdt_account.clone(),
                    hp_usdt_account.clone(),
                    pda_account.clone(),
                    token_program.clone(),
                ],
                &[&[&b"divvyexchange"[..], &[bump_seed]]],
            )?;
        } else {
            bet_state.outcome = 2; //User have lost
        }

        //Take rent from source
        //Todo find how I can transfer whole of the balance
        let balance = bet_state_account.lamports();
        **bet_state_account.try_borrow_mut_lamports()? -= balance;
        **user_main_account.try_borrow_mut_lamports()? += balance;
        Bet::pack(bet_state, &mut bet_state_account.data.borrow_mut())?;
        Ok(())
    }

    fn process_initmoneylinebet(accounts: &[AccountInfo], _program_id: &Pubkey) -> ProgramResult {
        msg!("Divvy program init market");
        let accounts_iter = &mut accounts.iter();
        let initializer = next_account_info(accounts_iter)?;
        let market_state_account = next_account_info(accounts_iter)?;
        let market_side_0_feed_account = next_account_info(accounts_iter)?;
        let market_side_1_feed_account = next_account_info(accounts_iter)?;
        let market_side_2_feed_account = next_account_info(accounts_iter)?;
        let result_feed_account = next_account_info(accounts_iter)?;

        if !initializer.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        msg!("Initializing market as required. Result as 3 means market is not settled yet");
        let mut market_state = Market::unpack_unchecked(&market_state_account.data.borrow())?;
        market_state.is_initialized = true;
        market_state.options_data[0].0 = market_side_0_feed_account.key.clone();
        market_state.options_data[0].1 = 0;
        market_state.options_data[0].2 = 0;
        market_state.options_data[1].0 = market_side_1_feed_account.key.clone();
        market_state.options_data[1].1 = 0;
        market_state.options_data[1].2 = 0;
        market_state.options_data[2].0 = market_side_2_feed_account.key.clone();
        market_state.options_data[2].1 = 0;
        market_state.options_data[2].2 = 0;
        market_state.max_loss = 0;
        market_state.result_feed = result_feed_account.key.clone();
        market_state.result = 3;
        Market::pack(market_state, &mut market_state_account.data.borrow_mut())?;

        Ok(())
    }

    fn process_settle_moneyline_market(
        accounts: &[AccountInfo],
        _program_id: &Pubkey,
    ) -> ProgramResult {
        msg!("Divvy program settle money line market");
        let accounts_iter = &mut accounts.iter();
        let initializer = next_account_info(accounts_iter)?;
        let market_state_account = next_account_info(accounts_iter)?;
        let hp_state_account = next_account_info(accounts_iter)?;
        let result_account = next_account_info(accounts_iter)?;

        if initializer.key != &authority::ID {
            return Err(NotValidAuthority.into());
        }

        let mut market_state = Market::unpack_unchecked(&market_state_account.data.borrow())?;
        let mut hp_state = HpLiquidity::unpack_unchecked(&hp_state_account.data.borrow())?;
        //Verifying result account
        if result_account.key != &market_state.result_feed {
            return Err(NotValidAuthority.into());
        }
        //Checking if market is not settled yet
        if market_state.result == 3 {
            return Err(MarketAlreadySettled.into());
        }
        //Getting odds from the Switchboard
        let aggregator: AggregatorState = get_aggregator(result_account)?;
        let round_result: RoundResult = get_aggregator_result(&aggregator)?;
        let result = round_result.result.expect("Invalid feed") as i32;
        if result > 3 || result < 0 {
            return Err(NotValidResult.into());
        } else {
            let net_value = match result {
                0 => {
                    market_state.options_data[0].1
                        - (market_state.options_data[1].2 + market_state.options_data[2].2)
                }
                1 => {
                    market_state.options_data[1].1
                        - (market_state.options_data[0].2 + market_state.options_data[2].2)
                }
                2 => {
                    market_state.options_data[2].1
                        - (market_state.options_data[0].2 + market_state.options_data[1].2)
                }
                _ => {
                    return Err(InvalidInstruction.into());
                }
            };
            market_state.result = result as u8;
            let current_amount_blocked = market_state.max_loss;
            let current_liquidity = hp_state.available_liquidity;
            hp_state.available_liquidity = current_liquidity + current_amount_blocked - net_value;
            Market::pack(market_state, &mut market_state_account.data.borrow_mut())?;
            HpLiquidity::pack(hp_state, &mut hp_state_account.data.borrow_mut())?;
        }
        Ok(())
    }

    fn process_ownership(
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
}
