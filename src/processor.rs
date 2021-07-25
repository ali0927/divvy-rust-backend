use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    program_pack::Pack,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};

use spl_token::{
    instruction::{burn, mint_to, transfer},
    state::Account as TokenAccount,
    state::Mint as TokenMint,
};

//Switchboard dependencies
use switchboard_program::{get_aggregator, get_aggregator_result, AggregatorState, RoundResult};

use crate::{
    calculate_available_liquidity, calculate_locked_liquidity, calculate_payout,
    error::ExchangeError,
    instruction::ExchangeInstruction,
    schema::{authority, token_program_id},
    state::{Bet, HpLiquidity, Market, MarketSide, MoneylineMarketOutcome},
};

use fixed::types::U64F64;

pub struct Processor;
impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = ExchangeInstruction::unpack(instruction_data)?;

        match instruction {
            ExchangeInstruction::Deposit {
                usdt_amount,
                bump_seed,
            } => {
                msg!("Divvy - Deposit");
                Self::process_deposit(accounts, usdt_amount, bump_seed, program_id)
            }
            ExchangeInstruction::Withdraw {
                ht_amount,
                bump_seed,
            } => {
                msg!("Divvy - Withdraw");
                Self::process_withdraw(accounts, ht_amount, bump_seed, program_id)
            }
            ExchangeInstruction::Initbet {
                risk,
                odds,
                market_side,
            } => {
                msg!("Divvy - Init Bet");
                Self::process_init_bet(accounts, risk, odds, market_side, program_id)
            }
            ExchangeInstruction::SettleBet { bump_seed } => {
                msg!("Divvy - Settle");
                Self::process_settle_bet(accounts, bump_seed, program_id)
            }
            ExchangeInstruction::InitMoneylineMarket => {
                msg!("Divvy - Init Moneyline Market");
                Self::process_init_moneyline_market(accounts, program_id)
            }
            ExchangeInstruction::SettleMoneylineMarket { bump_seed } => {
                msg!("Divvy - Settle Moneyline Market");
                Self::process_settle_moneyline_market(accounts, program_id, bump_seed)
            }
            ExchangeInstruction::Ownership { bump_seed } => {
                msg!("Divvy - Ownership");
                Self::process_ownership(accounts, bump_seed, program_id)
            }
            ExchangeInstruction::CommenceMarket => {
                msg!("Divvy - Commence Market");
                Self::process_commence_market(accounts, program_id)
            }
            ExchangeInstruction::Freeze {
                freeze_pool,
                freeze_betting,
            } => {
                msg!("Divvy - Freeze");
                Self::process_freeze(accounts, program_id, freeze_pool, freeze_betting)
            }
        }
    }

    fn process_deposit(
        accounts: &[AccountInfo],
        usdt_amount: u64,
        bump_seed: u8,
        _program_id: &Pubkey,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let user_account = next_account_info(accounts_iter)?;
        let ht_mint_account = next_account_info(accounts_iter)?;
        let token_program = next_account_info(accounts_iter)?;
        let user_ht_account = next_account_info(accounts_iter)?;
        let pda_account = next_account_info(accounts_iter)?;
        let user_usdt_account = next_account_info(accounts_iter)?;
        let pool_usdt_account = next_account_info(accounts_iter)?;
        let pool_state_account = next_account_info(accounts_iter)?;

        msg!("- Unpacking pool state");
        let pool_state = HpLiquidity::unpack(&pool_state_account.data.borrow())?;
        msg!("- Unpacking ht mint");
        let ht_mint_state = TokenMint::unpack(&ht_mint_account.data.borrow())?;
        msg!("- Unpacking usdt pool");
        let pool_usdt_state = TokenAccount::unpack(&pool_usdt_account.data.borrow())?;

        // Checking house token ownership
        if *ht_mint_account.key != pool_state.ht_mint {
            return Err(ExchangeError::InvalidHtMintAccount.into());
        }
        if *pool_usdt_account.key != pool_state.pool_usdt {
            return Err(ExchangeError::InvalidPoolUsdtAccount.into());
        }
        if *token_program.key != token_program_id::ID {
            return Err(ExchangeError::InvalidInstruction.into());
        }

        msg!("- USDT amount deposited");
        msg!(0, 0, 0, 0, usdt_amount);
        msg!("- HT supply in circulation");
        msg!(0, 0, 0, 0, ht_mint_state.supply);
        msg!("- House pool balance");
        msg!(0, 0, 0, 0, pool_usdt_state.amount);
        msg!("- House pool locked liquidity");
        msg!(0, 0, 0, 0, pool_state.locked_liquidity);
        msg!("- House pool live liquidity");
        msg!(0, 0, 0, 0, pool_state.locked_liquidity);

        if pool_state.live_liquidity > 0 {
            return Err(ExchangeError::GamesAreLive.into());
        }
        if pool_state.frozen_pool {
            return Err(ExchangeError::PoolFrozen.into());
        }

        let ht_amount = match pool_usdt_state.amount {
            0 => usdt_amount,
            _ => (U64F64::from_num(ht_mint_state.supply)
                .checked_div(U64F64::from_num(
                    pool_usdt_state
                        .amount
                        .checked_sub(pool_state.bettor_balance)
                        .ok_or(ExchangeError::AmountOverflow)?,
                ))
                .ok_or(ExchangeError::AmountOverflow)?
                .checked_mul(U64F64::from_num(usdt_amount))
                .ok_or(ExchangeError::AmountOverflow)?)
            .to_num(),
        };

        msg!("- HT amount received");
        msg!(0, 0, 0, 0, ht_amount);

        //let (_pda, bump_seed) = Pubkey::find_program_address(&[b"divvyexchange"], program_id);
        let transfer_instruction = transfer(
            token_program.key,
            &user_usdt_account.key,
            &pool_usdt_account.key,
            &user_account.key,
            &[&user_account.key],
            usdt_amount.clone(),
        )?;
        msg!("Calling the token program to transfer tokens...");
        invoke(
            &transfer_instruction,
            &[
                user_usdt_account.clone(),
                pool_usdt_account.clone(),
                user_account.clone(),
                token_program.clone(),
            ],
        )?;

        msg!("Creating mint instruction");
        let mint_ix = mint_to(
            &token_program.key,
            &ht_mint_account.key,
            &user_ht_account.key,
            &pda_account.key,
            &[&pda_account.key],
            ht_amount,
        )?;

        invoke_signed(
            &mint_ix,
            &[
                ht_mint_account.clone(),
                user_ht_account.clone(),
                pda_account.clone(),
            ],
            &[&[b"divvyexchange", &[bump_seed]]],
        )?;

        Ok(())
    }

    fn process_withdraw(
        accounts: &[AccountInfo],
        ht_amount: u64,
        bump_seed: u8,
        _program_id: &Pubkey,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let user_account = next_account_info(accounts_iter)?;
        let ht_mint_account = next_account_info(accounts_iter)?;
        let token_program = next_account_info(accounts_iter)?;
        let user_ht_account = next_account_info(accounts_iter)?;
        let pda_account = next_account_info(accounts_iter)?;
        let user_usdt_account = next_account_info(accounts_iter)?;
        let pool_usdt_account = next_account_info(accounts_iter)?;
        let pool_state_account = next_account_info(accounts_iter)?;

        let pool_state = HpLiquidity::unpack(&pool_state_account.data.borrow())?;
        let ht_mint_state = TokenMint::unpack(&ht_mint_account.data.borrow())?;
        let pool_usdt_state = TokenAccount::unpack(&pool_usdt_account.data.borrow())?;

        // Checking house token ownership
        if *ht_mint_account.key != pool_state.ht_mint {
            return Err(ExchangeError::InvalidHtMintAccount.into());
        }
        if *pool_usdt_account.key != pool_state.pool_usdt {
            return Err(ExchangeError::InvalidPoolUsdtAccount.into());
        }
        if *token_program.key != token_program_id::ID {
            return Err(ExchangeError::InvalidInstruction.into());
        }

        msg!("- HT amount burned");
        msg!(0, 0, 0, 0, ht_amount);
        msg!("- HT supply in circulation");
        msg!(0, 0, 0, 0, ht_mint_state.supply);
        msg!("- House pool balance");
        msg!(0, 0, 0, 0, pool_usdt_state.amount);
        msg!("- House pool locked liquidity");
        msg!(0, 0, 0, 0, pool_state.locked_liquidity);
        msg!("- House pool bettor balance");
        msg!(0, 0, 0, 0, pool_state.bettor_balance);
        msg!("- House pool live liquidity");
        msg!(0, 0, 0, 0, pool_state.locked_liquidity);

        if pool_state.live_liquidity > 0 {
            return Err(ExchangeError::GamesAreLive.into());
        }
        if pool_state.frozen_pool {
            return Err(ExchangeError::PoolFrozen.into());
        }

        let usdt_amount: u64 = (U64F64::from_num(
            pool_usdt_state
                .amount
                .checked_sub(pool_state.bettor_balance)
                .ok_or(ExchangeError::AmountOverflow)?,
        )
        .checked_div(U64F64::from_num(ht_mint_state.supply))
        .ok_or(ExchangeError::AmountOverflow)?
        .checked_mul(U64F64::from_num(ht_amount))
        .ok_or(ExchangeError::AmountOverflow)?)
        .to_num();
        let available_liquidity = calculate_available_liquidity(&pool_usdt_state, &pool_state)?;

        msg!("- House pool available liquidity");
        msg!(0, 0, 0, 0, available_liquidity);
        msg!("- USDT amount received");
        msg!(0, 0, 0, 0, usdt_amount);

        if usdt_amount > available_liquidity {
            return Err(ExchangeError::NotEnoughAvailableLiquidityForWithdrawal.into());
        }

        msg!("Burning HT");
        let burn_tx = burn(
            &token_program.key,
            &user_ht_account.key,
            &ht_mint_account.key,
            &user_account.key,
            &[&user_account.key],
            ht_amount,
        )?;

        invoke(
            &burn_tx,
            &[
                token_program.clone(),
                user_ht_account.clone(),
                ht_mint_account.clone(),
                user_account.clone(),
            ],
        )?;

        msg!("Transfering USDT to the user");
        let transfer_instruction = transfer(
            &token_program.key,
            &pool_usdt_account.key,
            &user_usdt_account.key,
            &pda_account.key,
            &[&pda_account.key],
            usdt_amount.clone(),
        )?;
        invoke_signed(
            &transfer_instruction,
            &[
                pool_usdt_account.clone(),
                user_usdt_account.clone(),
                pda_account.clone(),
                token_program.clone(),
            ],
            &[&[b"divvyexchange", &[bump_seed]]],
        )?;

        Ok(())
    }

    fn process_init_bet(
        accounts: &[AccountInfo],
        risk: u64,
        _odds: u64,
        market_side: u8,
        program_id: &Pubkey,
    ) -> ProgramResult {
        msg!("- Risk");
        msg!(0, 0, 0, 0, risk);

        let accounts_iter = &mut accounts.iter();
        let initializer = next_account_info(accounts_iter)?;
        if !initializer.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        let feed_account = next_account_info(accounts_iter)?;
        let bet_account = next_account_info(accounts_iter)?;
        let market_state_account = next_account_info(accounts_iter)?;
        let pool_state_account = next_account_info(accounts_iter)?;
        let pool_usdt_account = next_account_info(accounts_iter)?;
        let user_usdt_account = next_account_info(accounts_iter)?;
        let token_program = next_account_info(accounts_iter)?;

        msg!("Validating accounts");
        //Checking if market is initialized
        let mut market_state = Market::unpack(&market_state_account.data.borrow())
            .map_err(|_| Into::<ProgramError>::into(ExchangeError::MarketNotInitialized))?;
        let mut pool_state = HpLiquidity::unpack(&pool_state_account.data.borrow())
            .map_err(|_| Into::<ProgramError>::into(ExchangeError::HpLiquidityNotInitialized))?;
        let mut bet_state = Bet::unpack_unchecked(&bet_account.data.borrow())?;
        if bet_state.is_initialized {
            return Err(ExchangeError::BetAlreadyInitialized.into());
        }
        if !Rent::get()?.is_exempt(**bet_account.lamports.borrow(), bet_account.data_len()) {
            return Err(ProgramError::AccountNotRentExempt);
        }
        let pool_usdt_state = TokenAccount::unpack(&pool_usdt_account.data.borrow())?;

        // Checking house token ownership
        if *pool_usdt_account.key != pool_state.pool_usdt {
            return Err(ExchangeError::InvalidPoolUsdtAccount.into());
        }
        if *market_state_account.owner != *program_id {
            return Err(ExchangeError::InvalidMarketAccount.into());
        }
        if *token_program.key != token_program_id::ID {
            return Err(ExchangeError::InvalidInstruction.into());
        }
        //Checking if betting is frozen
        if pool_state.frozen_betting {
            return Err(ExchangeError::BettingFrozen.into());
        }
        //Checking if market is not commenced or settled yet
        if market_state.result != MoneylineMarketOutcome::NotYetCommenced {
            return Err(ExchangeError::MarketCommenced.into());
        }
        //Checking if feed account is right
        if market_state.market_sides[market_side as usize].feed_account != *feed_account.key {
            return Err(ExchangeError::InvalidFeedAccount.into());
        }

        // Checking if risk is non zero
        if risk == 0 {
            return Err(ExchangeError::BetRiskZero.into());
        }

        let available_liquidty = calculate_available_liquidity(&pool_usdt_state, &pool_state)?;

        //Getting odds from the Switchboard
        let aggregator: AggregatorState = get_aggregator(feed_account)?;
        let round_result: RoundResult = get_aggregator_result(&aggregator)?;
        let feed_odds = round_result
            .result
            .ok_or(ExchangeError::FeedNotInitialized)?;
        if feed_odds >= 0f64 {
            msg!("- Odds from feed: Positive:");
            msg!(0, 0, 0, 0, feed_odds as u64);
        } else {
            msg!("- Odds from feed: Negative:");
            msg!(0, 0, 0, 0, -feed_odds as u64);
        }

        //To Do comparison of provided odds & feed odds.

        //Calculate payout
        let payout = calculate_payout(feed_odds, risk).ok_or(ExchangeError::InvalidInstruction)?;
        msg!("- Bet payout");
        msg!(0, 0, 0, 0, payout);

        // Increment pending bets
        msg!("Incrementing market pending bets.");
        market_state.pending_bets = market_state
            .pending_bets
            .checked_add(1)
            .ok_or(ExchangeError::AmountOverflow)?;

        msg!("Incrementing house pool pending bets.");
        pool_state.pending_bets = pool_state
            .pending_bets
            .checked_add(1)
            .ok_or(ExchangeError::AmountOverflow)?;

        // Increment market bettor balance
        market_state.bettor_balance = market_state
            .bettor_balance
            .checked_add(risk)
            .ok_or(ExchangeError::AmountOverflow)?;

        pool_state.bettor_balance = pool_state
            .bettor_balance
            .checked_add(risk)
            .ok_or(ExchangeError::AmountOverflow)?;

        //Add risk & payout in market side
        let current_market_side_risk = market_state.market_sides[market_side as usize].risk;
        let current_market_side_payout = market_state.market_sides[market_side as usize].payout;
        market_state.market_sides[market_side as usize].risk = current_market_side_risk
            .checked_add(risk)
            .ok_or(ExchangeError::AmountOverflow)?;
        market_state.market_sides[market_side as usize].payout = current_market_side_payout
            .checked_add(payout)
            .ok_or(ExchangeError::AmountOverflow)?;

        //Calculating locked liquidity
        let new_locked_liquidity = calculate_locked_liquidity(&market_state)?;
        let current_locked_liquidity = market_state.locked_liquidity;
        let current_pool_locked_liquidity = pool_state.locked_liquidity;

        //Confirm there is enough available liquidity
        if new_locked_liquidity > current_locked_liquidity {
            if new_locked_liquidity
                .checked_sub(current_locked_liquidity)
                .ok_or(ExchangeError::AmountOverflow)?
                > available_liquidty
            {
                return Err(ExchangeError::NotEnoughAvailableLiquidityForBet.into());
            }
        }

        market_state.locked_liquidity = new_locked_liquidity;
        pool_state.locked_liquidity = current_pool_locked_liquidity
            .checked_sub(current_locked_liquidity)
            .ok_or(ExchangeError::AmountOverflow)?
            .checked_add(new_locked_liquidity)
            .ok_or(ExchangeError::NotEnoughAvailableLiquidityForBet)?;

        msg!("- Market locked liquidity from");
        msg!(0, 0, 0, 0, current_locked_liquidity);
        msg!("- Market locked liquidity to");
        msg!(0, 0, 0, 0, new_locked_liquidity);
        msg!("- Pool locked liquidity from");
        msg!(0, 0, 0, 0, current_pool_locked_liquidity);
        msg!("- Pool locked liquidity to");
        msg!(0, 0, 0, 0, pool_state.locked_liquidity);

        //Transfer token from user account to hp account
        let transfer_instruction = transfer(
            &token_program.key,
            &user_usdt_account.key,
            &pool_usdt_account.key,
            &initializer.key,
            &[&initializer.key],
            risk,
        )?;
        msg!("Transferring risk from user account to divvy account");
        invoke(
            &transfer_instruction,
            &[
                user_usdt_account.clone(),
                pool_usdt_account.clone(),
                initializer.clone(),
                token_program.clone(),
            ],
        )?;

        // Initialize bet state
        bet_state = Bet {
            is_initialized: true,
            market: *market_state_account.key,
            user_usdt_account: *user_usdt_account.key,
            user_main_account: *initializer.key,
            user_risk: risk,
            user_payout: payout,
            user_market_side: market_side,
            outcome: 0, //Outcome 0 as market not settled.
        };

        //Write the accounts
        Bet::pack(bet_state, &mut bet_account.data.borrow_mut())?;
        HpLiquidity::pack(pool_state, &mut pool_state_account.data.borrow_mut())?;
        Market::pack(market_state, &mut market_state_account.data.borrow_mut())?;

        Ok(())
    }

    fn process_settle_bet(
        accounts: &[AccountInfo],
        bump_seed: u8,
        program_id: &Pubkey,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let _initializer = next_account_info(accounts_iter)?;
        let token_program = next_account_info(accounts_iter)?;
        let market_state_account = next_account_info(accounts_iter)?;
        let bet_state_account = next_account_info(accounts_iter)?;
        let pda_account = next_account_info(accounts_iter)?;
        let pool_usdt_account = next_account_info(accounts_iter)?;
        let user_usdt_account = next_account_info(accounts_iter)?;
        let user_main_account = next_account_info(accounts_iter)?;
        let pool_state_account = next_account_info(accounts_iter)?;

        let mut pool_state = HpLiquidity::unpack(&pool_state_account.data.borrow())?;
        let mut market_state = Market::unpack(&market_state_account.data.borrow())?;
        let mut bet_state = Bet::unpack(&bet_state_account.data.borrow())?;

        // Checking house token ownership
        if *pool_usdt_account.key != pool_state.pool_usdt {
            return Err(ExchangeError::InvalidPoolUsdtAccount.into());
        }
        if *market_state_account.owner != *program_id {
            return Err(ExchangeError::InvalidMarketAccount.into());
        }
        if *token_program.key != token_program_id::ID {
            return Err(ExchangeError::InvalidInstruction.into());
        }

        if bet_state.market != *market_state_account.key {
            return Err(ExchangeError::ExpectedDataMismatch.into());
        }

        //Checking if betting is frozen
        if pool_state.frozen_betting {
            return Err(ExchangeError::BettingFrozen.into());
        }
        if market_state.result == MoneylineMarketOutcome::NotYetCommenced {
            return Err(ExchangeError::MarketNotSettled.into());
        }

        if bet_state.user_usdt_account != *user_usdt_account.key {
            return Err(ExchangeError::ExpectedDataMismatch.into());
        }

        if bet_state.user_main_account != *user_main_account.key {
            return Err(ExchangeError::ExpectedDataMismatch.into());
        }

        if bet_state.outcome != 0 {
            return Err(ExchangeError::BetAlreadySettled.into());
        }

        // Decrement pending bets
        msg!("Decrementing market pending bets.");
        market_state.pending_bets = market_state
            .pending_bets
            .checked_sub(1)
            .ok_or(ExchangeError::AmountOverflow)?;

        msg!("Decrementing house pool pending bets.");
        pool_state.pending_bets = pool_state
            .pending_bets
            .checked_sub(1)
            .ok_or(ExchangeError::AmountOverflow)?;

        if bet_state.user_market_side != market_state.result.pack() {
            bet_state.outcome = 2; //User have lost
        } else {
            bet_state.outcome = 1; //User have won
            let bet_balance = bet_state
                .user_risk
                .checked_add(bet_state.user_payout)
                .ok_or(ExchangeError::AmountOverflow)?;

            // Subtract bettor balance in the market and house pool
            // Only for winning bets, as when the market settles,
            // the balance is changed to only include the winning sides risk and payout
            market_state.bettor_balance = market_state
                .bettor_balance
                .checked_sub(bet_balance)
                .ok_or(ExchangeError::AmountOverflow)?;
            pool_state.bettor_balance = pool_state
                .bettor_balance
                .checked_sub(bet_balance)
                .ok_or(ExchangeError::AmountOverflow)?;

            //Remove risk & payout in market side. Only for winning bets, as locked
            // liquidity was already calculated for losers.
            let current_market_side_risk =
                market_state.market_sides[bet_state.user_market_side as usize].risk;
            let current_market_side_payout =
                market_state.market_sides[bet_state.user_market_side as usize].payout;
            market_state.market_sides[bet_state.user_market_side as usize].risk =
                current_market_side_risk
                    .checked_sub(bet_state.user_risk)
                    .ok_or(ExchangeError::MarketSideRiskUnderflow)?;
            market_state.market_sides[bet_state.user_market_side as usize].payout =
                current_market_side_payout
                    .checked_sub(bet_state.user_payout)
                    .ok_or(ExchangeError::MarketSidePayoutUnderflow)?;

            let transfer_instruction = transfer(
                &token_program.key,
                &pool_usdt_account.key,
                &user_usdt_account.key,
                &pda_account.key,
                &[&pda_account.key],
                bet_balance,
            )?;
            msg!("Calling the token program to transfer winnings to user.");
            invoke_signed(
                &transfer_instruction,
                &[
                    user_usdt_account.clone(),
                    pool_usdt_account.clone(),
                    pda_account.clone(),
                    token_program.clone(),
                ],
                //To Do Please test bump seed thing
                &[&[b"divvyexchange", &[bump_seed]]],
            )?;
        }

        //Return rent to the user that placed the bet
        let balance = bet_state_account.lamports();
        **bet_state_account.try_borrow_mut_lamports()? -= balance;
        **user_main_account.try_borrow_mut_lamports()? += balance;

        //Assert that when all of the markets winning bets are settled there is
        //no remaining risk, payout and bettor balance in the winning market side.
        if market_state.pending_bets == 0 {
            msg!("Market pending bets are settled. Asserting.");
            if market_state.market_sides[market_state.result as usize].risk != 0 {
                return Err(ExchangeError::MarketSideRiskRemaining.into());
            }
            if market_state.market_sides[market_state.result as usize].payout != 0 {
                return Err(ExchangeError::MarketSidePayoutRemaining.into());
            }
            if market_state.bettor_balance != 0 {
                return Err(ExchangeError::MarketBettorBalanceRemaining.into());
            }
        }

        //Assert that when all of the house pool pending bets are settled there is
        //no remaining bettor balance in the house pool.
        if pool_state.pending_bets == 0 {
            msg!("House pool pending bets are settled. Asserting.");
            if pool_state.bettor_balance != 0 {
                return Err(ExchangeError::HousePoolBettorBalanceRemaining.into());
            }
            if pool_state.locked_liquidity != 0 {
                return Err(ExchangeError::HousePoolLockedLiquidityRemaining.into());
            }
            if pool_state.live_liquidity != 0 {
                return Err(ExchangeError::HousePoolLockedLiquidityRemaining.into());
            }
        }

        HpLiquidity::pack(pool_state, &mut pool_state_account.data.borrow_mut())?;
        Market::pack(market_state, &mut market_state_account.data.borrow_mut())?;
        Bet::pack(bet_state, &mut bet_state_account.data.borrow_mut())?;
        Ok(())
    }

    fn process_init_moneyline_market(
        accounts: &[AccountInfo],
        _program_id: &Pubkey,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let initializer = next_account_info(accounts_iter)?;
        let market_state_account = next_account_info(accounts_iter)?;
        let market_side_0_feed_account = next_account_info(accounts_iter)?;
        let market_side_1_feed_account = next_account_info(accounts_iter)?;
        let market_side_2_feed_account = next_account_info(accounts_iter)?;
        let result_feed_account = next_account_info(accounts_iter)?;
        let pool_state_account = next_account_info(accounts_iter)?;

        if !initializer.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if initializer.key != &authority::ID {
            return Err(ExchangeError::NotValidAuthority.into());
        }

        let pool_state = HpLiquidity::unpack(&pool_state_account.data.borrow())?;
        let mut market_state = Market::unpack_unchecked(&market_state_account.data.borrow())?;

        //Checking if betting is frozen
        if pool_state.frozen_betting {
            return Err(ExchangeError::BettingFrozen.into());
        }
        if market_state.is_initialized {
            return Err(ExchangeError::MarketAlreadyInitialized.into());
        }
        if !Rent::get()?.is_exempt(
            **market_state_account.lamports.borrow(),
            market_state_account.data_len(),
        ) {
            return Err(ProgramError::AccountNotRentExempt);
        }

        market_state = Market {
            is_initialized: true,
            market_sides: [
                MarketSide {
                    feed_account: *market_side_0_feed_account.key,
                    payout: 0,
                    risk: 0,
                },
                MarketSide {
                    feed_account: *market_side_1_feed_account.key,
                    payout: 0,
                    risk: 0,
                },
                MarketSide {
                    feed_account: *market_side_2_feed_account.key,
                    payout: 0,
                    risk: 0,
                },
            ],
            locked_liquidity: 0,
            result_feed: *result_feed_account.key,
            result: MoneylineMarketOutcome::NotYetCommenced,
            bettor_balance: 0,
            pending_bets: 0,
        };
        Market::pack(market_state, &mut market_state_account.data.borrow_mut())?;

        Ok(())
    }

    fn process_settle_moneyline_market(
        accounts: &[AccountInfo],
        program_id: &Pubkey,
        bump_seed: u8,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let _initializer = next_account_info(accounts_iter)?;
        let market_state_account = next_account_info(accounts_iter)?;
        let pool_state_account = next_account_info(accounts_iter)?;
        let result_account = next_account_info(accounts_iter)?;
        let pda_account = next_account_info(accounts_iter)?;
        let pool_usdt_account = next_account_info(accounts_iter)?;
        let insurance_fund_usdt_account = next_account_info(accounts_iter)?;
        let divvy_foundation_proceeds_usdt = next_account_info(accounts_iter)?;
        let token_program = next_account_info(accounts_iter)?;

        let mut market_state = Market::unpack(&market_state_account.data.borrow())?;
        let mut pool_state = HpLiquidity::unpack(&pool_state_account.data.borrow())?;

        // Checking house token ownership
        if *pool_usdt_account.key != pool_state.pool_usdt {
            return Err(ExchangeError::InvalidPoolUsdtAccount.into());
        }
        if *insurance_fund_usdt_account.key != pool_state.insurance_fund_usdt {
            return Err(ExchangeError::InvalidInsuranceFundUsdtAccount.into());
        }
        if *divvy_foundation_proceeds_usdt.key != pool_state.divvy_foundation_proceeds_usdt {
            return Err(ExchangeError::InvalidDivvyFoundationUsdtAccount.into());
        }
        if *token_program.key != token_program_id::ID {
            return Err(ExchangeError::InvalidInstruction.into());
        }
        if *market_state_account.owner != *program_id {
            return Err(ExchangeError::InvalidMarketAccount.into());
        }
        //Checking if betting is frozen
        if pool_state.frozen_betting {
            return Err(ExchangeError::BettingFrozen.into());
        }
        //Verifying result account
        if result_account.key != &market_state.result_feed {
            return Err(ExchangeError::NotValidAuthority.into());
        }
        //Checking if market is not settled yet
        if market_state.result != MoneylineMarketOutcome::NotYetCommenced
            || market_state.result != MoneylineMarketOutcome::Commenced
        {
            return Err(ExchangeError::MarketAlreadySettled.into());
        }
        //Getting results from Switchboard
        msg!("Unpacking switchboard aggregator.");
        let aggregator: AggregatorState = get_aggregator(result_account)?;
        msg!("Unpacking switchboard result.");
        let round_result: RoundResult = get_aggregator_result(&aggregator)?;
        msg!("Unpacking switchboard result option.");
        let result_u8 = round_result
            .result
            .ok_or(ExchangeError::FeedNotInitialized)? as u8;
        msg!("- Result feed");
        msg!(0, 0, 0, 0, result_u8);
        if result_u8 > 2 {
            return Err(ExchangeError::NotValidMarketResult.into());
        }

        msg!("- Market state");
        msg!(market_state.result.into());

        let new_market_result = MoneylineMarketOutcome::unpack(&result_u8)?;

        //When the market settles the bettor balance changes from the amount of risk the bettors
        //have entered into the market to the winning sides unsettled risk and payout.
        let current_bettor_balance = market_state.bettor_balance;
        let new_bettor_balance = market_state.market_sides[new_market_result as usize]
            .risk
            .checked_add(market_state.market_sides[new_market_result as usize].payout)
            .ok_or(ExchangeError::AmountOverflow)?;

        if new_bettor_balance < current_bettor_balance {
            // The house has made money
            let house_profit = current_bettor_balance
                .checked_sub(new_bettor_balance)
                .ok_or(ExchangeError::AmountOverflow)?;
            let house_profit_frac: U64F64 = U64F64::from_num(house_profit);

            let insurance_fund_fee: u64 = (house_profit_frac * U64F64::from_num(0.01))
                .checked_to_num()
                .ok_or(ExchangeError::AmountOverflow)?;
            let divvy_foundation_fee: u64 = (house_profit_frac * U64F64::from_num(0.05))
                .checked_to_num()
                .ok_or(ExchangeError::AmountOverflow)?;

            msg!("- House profit before fees");
            msg!(0, 0, 0, 0, house_profit);
            msg!("- Insurance fund fee");
            msg!(0, 0, 0, 0, insurance_fund_fee);
            msg!("- Divvy Foundation fee");
            msg!(0, 0, 0, 0, divvy_foundation_fee);

            msg!("Transfering USDT to the insurance fund");
            let transfer_instruction = transfer(
                &token_program.key,
                &pool_usdt_account.key,
                &insurance_fund_usdt_account.key,
                &pda_account.key,
                &[&pda_account.key],
                insurance_fund_fee.clone(),
            )?;
            invoke_signed(
                &transfer_instruction,
                &[
                    pool_usdt_account.clone(),
                    insurance_fund_usdt_account.clone(),
                    pda_account.clone(),
                    token_program.clone(),
                ],
                &[&[b"divvyexchange", &[bump_seed]]],
            )?;
            msg!("Transfering USDT to the Divvy foundation");
            let transfer_instruction = transfer(
                &token_program.key,
                &pool_usdt_account.key,
                &divvy_foundation_proceeds_usdt.key,
                &pda_account.key,
                &[&pda_account.key],
                divvy_foundation_fee.clone(),
            )?;
            invoke_signed(
                &transfer_instruction,
                &[
                    pool_usdt_account.clone(),
                    divvy_foundation_proceeds_usdt.clone(),
                    pda_account.clone(),
                    token_program.clone(),
                ],
                &[&[b"divvyexchange", &[bump_seed]]],
            )?;
        }

        market_state.bettor_balance = new_bettor_balance;
        pool_state.bettor_balance = pool_state
            .bettor_balance
            .checked_sub(current_bettor_balance)
            .ok_or(ExchangeError::AmountOverflow)?
            .checked_add(new_bettor_balance)
            .ok_or(ExchangeError::AmountOverflow)?;

        // Calculate locked liquidity after losers lose payout
        let new_locked_liquidity = 0u64;
        let current_locked_liquidity = market_state.locked_liquidity;
        let current_pool_locked_liquidity = pool_state.locked_liquidity;
        let current_pool_live_liquidity = pool_state.live_liquidity;
        market_state.locked_liquidity = new_locked_liquidity;
        match market_state.result {
            MoneylineMarketOutcome::NotYetCommenced => {
                pool_state.locked_liquidity = pool_state
                    .locked_liquidity
                    .checked_sub(current_locked_liquidity)
                    .ok_or(ExchangeError::AmountOverflow)?;
            }
            MoneylineMarketOutcome::Commenced => {
                pool_state.live_liquidity = pool_state
                    .live_liquidity
                    .checked_sub(current_locked_liquidity)
                    .ok_or(ExchangeError::AmountOverflow)?;
            }
            _ => return Err(ExchangeError::InvalidInstruction.into()),
        }

        market_state.result = new_market_result;

        msg!("- Market locked liquidity from");
        msg!(0, 0, 0, 0, current_locked_liquidity);
        msg!("- Market locked liquidity to");
        msg!(0, 0, 0, 0, new_locked_liquidity);
        msg!("- Pool locked liquidity from");
        msg!(0, 0, 0, 0, current_pool_locked_liquidity);
        msg!("- Pool locked liquidity to");
        msg!(0, 0, 0, 0, pool_state.locked_liquidity);
        msg!("- Pool live liquidity from");
        msg!(0, 0, 0, 0, current_pool_live_liquidity);
        msg!("- Pool live liquidity to");
        msg!(0, 0, 0, 0, pool_state.live_liquidity);

        Market::pack(market_state, &mut market_state_account.data.borrow_mut())?;
        HpLiquidity::pack(pool_state, &mut pool_state_account.data.borrow_mut())?;

        Ok(())
    }

    pub fn process_ownership(
        accounts: &[AccountInfo],
        _bump_seed: u8,
        _program_id: &Pubkey,
    ) -> ProgramResult {
        msg!("Divvy program ownership");
        let accounts_iter = &mut accounts.iter();
        let initializer = next_account_info(accounts_iter)?;
        let pool_state_account = next_account_info(accounts_iter)?;
        let ht_mint_account = next_account_info(accounts_iter)?;
        let pool_usdt_account = next_account_info(accounts_iter)?;
        let insurance_fund_usdt_account = next_account_info(accounts_iter)?;
        let divvy_foundation_proceeds_usdt = next_account_info(accounts_iter)?;

        let mut pool_state = HpLiquidity::unpack_unchecked(&pool_state_account.data.borrow())?;
        if pool_state.is_initialized {
            return Err(ExchangeError::HpLiquidityAlreadyInitialized.into());
        }
        if !Rent::get()?.is_exempt(
            **pool_state_account.lamports.borrow(),
            pool_state_account.data_len(),
        ) {
            return Err(ProgramError::AccountNotRentExempt);
        }
        // Unpack token accounts to verify their length
        TokenMint::unpack(&ht_mint_account.data.borrow())?;
        TokenAccount::unpack(&pool_usdt_account.data.borrow())?;
        TokenAccount::unpack(&insurance_fund_usdt_account.data.borrow())?;
        TokenAccount::unpack(&divvy_foundation_proceeds_usdt.data.borrow())?;

        if initializer.key != &authority::ID {
            return Err(ExchangeError::NotValidAuthority.into());
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
        pool_state = HpLiquidity {
            is_initialized: true,
            locked_liquidity: 0,
            live_liquidity: 0,
            bettor_balance: 0,
            pending_bets: 0,
            ht_mint: *ht_mint_account.key,
            pool_usdt: *pool_usdt_account.key,
            insurance_fund_usdt: *insurance_fund_usdt_account.key,
            divvy_foundation_proceeds_usdt: *divvy_foundation_proceeds_usdt.key,
            frozen_pool: false,
            frozen_betting: false,
        };
        HpLiquidity::pack(pool_state, &mut pool_state_account.data.borrow_mut())?;
        Ok(())
    }

    pub fn process_commence_market(
        accounts: &[AccountInfo],
        _program_id: &Pubkey,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let initializer = next_account_info(accounts_iter)?;
        if !initializer.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if initializer.key != &authority::ID {
            return Err(ExchangeError::NotValidAuthority.into());
        }
        let market_state_account = next_account_info(accounts_iter)?;
        let pool_state_account = next_account_info(accounts_iter)?;

        let mut market_state = Market::unpack(&market_state_account.data.borrow())?;
        let mut pool_state = HpLiquidity::unpack(&pool_state_account.data.borrow())?;

        //Checking if betting is frozen
        if pool_state.frozen_betting {
            return Err(ExchangeError::BettingFrozen.into());
        }
        if market_state.result != MoneylineMarketOutcome::NotYetCommenced {
            return Err(ExchangeError::MarketCommenced.into());
        }

        market_state.result = MoneylineMarketOutcome::Commenced;
        pool_state.locked_liquidity = pool_state
            .locked_liquidity
            .checked_sub(market_state.locked_liquidity)
            .ok_or(ExchangeError::AmountOverflow)?;
        pool_state.live_liquidity = pool_state
            .live_liquidity
            .checked_add(market_state.locked_liquidity)
            .ok_or(ExchangeError::AmountOverflow)?;

        Market::pack(market_state, &mut market_state_account.data.borrow_mut())?;
        HpLiquidity::pack(pool_state, &mut pool_state_account.data.borrow_mut())?;

        Ok(())
    }

    pub fn process_freeze(
        accounts: &[AccountInfo],
        _program_id: &Pubkey,
        freeze_pool: bool,
        freeze_betting: bool,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let initializer = next_account_info(accounts_iter)?;
        let pool_state_account = next_account_info(accounts_iter)?;

        if !initializer.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }
        if initializer.key != &authority::ID {
            return Err(ExchangeError::NotValidAuthority.into());
        }

        let mut pool_state = HpLiquidity::unpack(&pool_state_account.data.borrow())?;

        if freeze_pool && !pool_state.frozen_pool {
            msg!("Freezing pool");
        } else if !freeze_pool && pool_state.frozen_pool {
            msg!("Unfreezing pool");
        }
        if freeze_betting && !pool_state.frozen_betting {
            msg!("Freezing betting");
        } else if !freeze_betting && pool_state.frozen_betting {
            msg!("Unfreezing betting");
        }

        pool_state.frozen_pool = freeze_pool;
        pool_state.frozen_betting = freeze_betting;

        HpLiquidity::pack(pool_state, &mut pool_state_account.data.borrow_mut())?;

        Ok(())
    }
}
