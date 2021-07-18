use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    //program_pack::{IsInitialized, Pack},
    program_pack::Pack,
    pubkey::Pubkey,
};

//use spl_associated_token_account::get_associated_token_address;
use spl_token::{
    instruction::{burn, mint_to, transfer},
    state::Account as TokenAccount,
    state::Mint as TokenMint,
};

//Switchboard dependencies
use switchboard_program::{get_aggregator, get_aggregator_result, AggregatorState, RoundResult};

use crate::{
    calculate_locked_liquidity, calculate_payout,
    error::ExchangeError,
    instruction::ExchangeInstruction,
    schema::authority,
    state::{Bet, HpLiquidity, Market, MoneylineMarketOutcome},
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
            ExchangeInstruction::Deposit {
                usdt_amount,
                bump_seed,
            } => {
                msg!("Instruction: Deposit");
                Self::process_deposit(accounts, usdt_amount, bump_seed, program_id)
            }
            ExchangeInstruction::Withdraw {
                ht_amount,
                bump_seed,
            } => {
                msg!("Instruction: Withdraw");
                Self::process_withdraw(accounts, ht_amount, bump_seed, program_id)
            }
            ExchangeInstruction::Initbet {
                risk,
                odds,
                market_side,
            } => {
                msg!("Instruction: Initbet");
                Self::process_init_bet(accounts, risk, odds, market_side, program_id)
            }
            ExchangeInstruction::SettleBet { bump_seed } => {
                msg!("Instruction: Settle");
                Self::process_settle_bet(accounts, bump_seed, program_id)
            }
            ExchangeInstruction::InitMoneylineMarket {} => {
                msg!("Instruction: Init Moneyline Market");
                Self::process_init_moneyline_market(accounts, program_id)
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
        usdt_amount: u64,
        bump_seed: u8,
        _program_id: &Pubkey,
    ) -> ProgramResult {
        msg!("Divvy program deposit");

        let accounts_iter = &mut accounts.iter();

        let user_account = next_account_info(accounts_iter)?;
        let ht_mint_account = next_account_info(accounts_iter)?;
        let token_program = next_account_info(accounts_iter)?;
        let user_ht_account = next_account_info(accounts_iter)?;
        let pda_account = next_account_info(accounts_iter)?;
        let user_usdt_account = next_account_info(accounts_iter)?;
        let pool_usdt_account = next_account_info(accounts_iter)?;
        let pool_state_account = next_account_info(accounts_iter)?;

        let mut pool_state = HpLiquidity::unpack(&pool_state_account.data.borrow())?;
        let ht_mint_state = TokenMint::unpack(&ht_mint_account.data.borrow())?;

        msg!("- USDT amount deposited");
        msg!(0, 0, 0, 0, usdt_amount);
        msg!("- HT supply in circulation");
        msg!(0, 0, 0, 0, ht_mint_state.supply);
        msg!("- House pool balance");
        msg!(0, 0, 0, 0, pool_state.available_liquidity);

        let conversion_ratio = match pool_state.available_liquidity {
            0 => 1f64,
            _ => ht_mint_state.supply as f64 / pool_state.available_liquidity as f64,
        };
        let ht_amount = (usdt_amount as f64 * conversion_ratio) as u64;

        msg!("- HT amount received");
        msg!(0, 0, 0, 0, ht_amount);

        //let (_pda, bump_seed) = Pubkey::find_program_address(&[b"divvyexchange"], program_id);
        let transfer_instruction = transfer(
            &token_program.key,
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

        pool_state.available_liquidity = pool_state.available_liquidity + usdt_amount;
        msg!("Adding deposit to liquidity");
        HpLiquidity::pack(pool_state, &mut pool_state_account.data.borrow_mut())?;

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

        let mut pool_state = HpLiquidity::unpack(&pool_state_account.data.borrow())?;
        let ht_mint_state = TokenMint::unpack(&ht_mint_account.data.borrow())?;

        msg!("- HT amount burned");
        msg!(0, 0, 0, 0, ht_amount);
        msg!("- HT supply in circulation");
        msg!(0, 0, 0, 0, ht_mint_state.supply);
        msg!("- House pool balance");
        msg!(0, 0, 0, 0, pool_state.available_liquidity);

        let conversion_ratio = pool_state.available_liquidity as f64 / ht_mint_state.supply as f64;
        let usdt_amount = (ht_amount as f64 * conversion_ratio) as u64;

        msg!("- USDT amount received");
        msg!(0, 0, 0, 0, usdt_amount);

        msg!("Subtracting withdrawal from liquidity");
        pool_state.available_liquidity = pool_state.available_liquidity - usdt_amount;
        HpLiquidity::pack(pool_state, &mut pool_state_account.data.borrow_mut())?;

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
        odds: u64,
        market_side: u8,
        _program_id: &Pubkey,
    ) -> ProgramResult {
        msg!(
            "Divvy program initbet with amount {} and provided odds {}",
            risk,
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
        let pool_state_account = next_account_info(accounts_iter)?;
        let pool_usdt_account = next_account_info(accounts_iter)?;
        let user_usdt_account = next_account_info(accounts_iter)?;
        let token_program = next_account_info(accounts_iter)?;

        msg!("Validating accounts");
        //Checking if market is initialized
        let mut market_state = Market::unpack(&market_state_account.data.borrow())
            .map_err(|_| Into::<ProgramError>::into(ExchangeError::MarketNotInitialized))?;

        let mut pool_state = HpLiquidity::unpack_unchecked(&pool_state_account.data.borrow())?;

        let mut bet_state = Bet::unpack_unchecked(&bet_account.data.borrow())?;
        if bet_state.is_initialized {
            return Err(ExchangeError::BetAlreadyInitialized.into());
        }

        //Checking if market is not settled yet
        if market_state.result != MoneylineMarketOutcome::NotYetSettled {
            return Err(ExchangeError::MarketAlreadySettled.into());
        }
        //Checking if feed account is right
        if market_state.market_sides[market_side as usize].feed_account != feed_account.key.clone()
        {
            return Err(ExchangeError::InvalidFeedAccount.into());
        }

        // Checking if risk is non zero
        if risk == 0 {
            return Err(ExchangeError::BetRiskZero.into());
        }

        //Getting odds from the Switchboard
        let aggregator: AggregatorState = get_aggregator(feed_account)?;
        let round_result: RoundResult = get_aggregator_result(&aggregator)?;
        let feed_odds: f64 = round_result
            .result
            .ok_or(ExchangeError::FeedNotInitialized)?;
        msg!("Odds from feed are {}", feed_odds as u64);

        //To Do comparison of provided odds & feed odds.

        //Calculate payout
        let payout = calculate_payout(feed_odds, risk);
        msg!("Bet payout is {}", payout);

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
        let current_available_liquidity = pool_state.available_liquidity;
        let current_locked_liquidity = market_state.locked_liquidity;
        market_state.locked_liquidity = new_locked_liquidity;
        pool_state.available_liquidity = current_available_liquidity
            .checked_add(current_locked_liquidity)
            .ok_or(ExchangeError::AmountOverflow)?
            .checked_sub(new_locked_liquidity)
            .ok_or(ExchangeError::NotEnoughLiquidity)?;
        msg!(
            "Market locked liquidity from {} to {}",
            current_locked_liquidity,
            new_locked_liquidity
        );
        msg!(
            "Pool available liquidity from {} to {}",
            current_available_liquidity,
            pool_state.available_liquidity
        );

        //Transfer token from user account to hp account
        let transfer_instruction = transfer(
            &token_program.key,
            &user_usdt_account.key,
            &pool_usdt_account.key,
            &initializer.key,
            &[&initializer.key],
            risk.clone(),
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
        bet_state.is_initialized = true;
        bet_state.market = market_state_account.key.clone();
        bet_state.user_usdt_account = user_usdt_account.key.clone();
        bet_state.user_main_account = initializer.key.clone();
        bet_state.user_risk = risk;
        bet_state.user_payout = payout;
        bet_state.user_market_side = market_side;
        bet_state.outcome = 0; //Outcome 0 as market not settled.

        //Write the accounts
        Bet::pack(bet_state, &mut bet_account.data.borrow_mut())?;
        HpLiquidity::pack(pool_state, &mut pool_state_account.data.borrow_mut())?;
        Market::pack(market_state, &mut market_state_account.data.borrow_mut())?;

        Ok(())
    }

    fn process_settle_bet(
        accounts: &[AccountInfo],
        bump_seed: u8,
        _program_id: &Pubkey,
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

        if bet_state.market != market_state_account.key.clone() {
            return Err(ExchangeError::ExpectedDataMismatch.into());
        }

        if market_state.result == MoneylineMarketOutcome::NotYetSettled {
            return Err(ExchangeError::MarketNotSettled.into());
        }

        if bet_state.user_usdt_account != user_usdt_account.key.clone() {
            return Err(ExchangeError::ExpectedDataMismatch.into());
        }

        if bet_state.user_main_account != user_main_account.key.clone() {
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
            msg!("Calling the token program to transfer winnings to user");
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
            let hp_usdt = TokenAccount::unpack(&pool_usdt_account.data.borrow())?;
            if pool_state.available_liquidity != hp_usdt.amount {
                return Err(ExchangeError::UnexpectedAvailableLiquidity.into());
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

        if initializer.key != &authority::ID {
            return Err(ExchangeError::NotValidAuthority.into());
        }

        msg!("Initializing market as required. Result as 3 means market is not settled yet");
        let mut market_state = Market::unpack_unchecked(&market_state_account.data.borrow())?;
        market_state.is_initialized = true;
        market_state.market_sides[0].feed_account = market_side_0_feed_account.key.clone();
        market_state.market_sides[0].payout = 0;
        market_state.market_sides[0].risk = 0;
        market_state.market_sides[1].feed_account = market_side_1_feed_account.key.clone();
        market_state.market_sides[1].payout = 0;
        market_state.market_sides[1].risk = 0;
        market_state.market_sides[2].feed_account = market_side_2_feed_account.key.clone();
        market_state.market_sides[2].payout = 0;
        market_state.market_sides[2].risk = 0;
        market_state.locked_liquidity = 0;
        market_state.result_feed = result_feed_account.key.clone();
        market_state.result = MoneylineMarketOutcome::NotYetSettled;
        Market::pack(market_state, &mut market_state_account.data.borrow_mut())?;

        Ok(())
    }

    fn process_settle_moneyline_market(
        accounts: &[AccountInfo],
        _program_id: &Pubkey,
    ) -> ProgramResult {
        msg!("Divvy program settle money line market");
        let accounts_iter = &mut accounts.iter();
        let _initializer = next_account_info(accounts_iter)?;
        let market_state_account = next_account_info(accounts_iter)?;
        let pool_state_account = next_account_info(accounts_iter)?;
        let result_account = next_account_info(accounts_iter)?;

        let mut market_state = Market::unpack_unchecked(&market_state_account.data.borrow())?;
        let mut pool_state = HpLiquidity::unpack_unchecked(&pool_state_account.data.borrow())?;

        //Verifying result account
        if result_account.key != &market_state.result_feed {
            return Err(ExchangeError::NotValidAuthority.into());
        }
        //Checking if market is not settled yet
        if market_state.result != MoneylineMarketOutcome::NotYetSettled {
            return Err(ExchangeError::MarketAlreadySettled.into());
        }
        //Getting results from Switchboard
        msg!("Unpacking switchboard aggregator");
        let aggregator: AggregatorState = get_aggregator(result_account)?;
        msg!("Unpacking switchboard result");
        let round_result: RoundResult = get_aggregator_result(&aggregator)?;
        let result_u8 = round_result
            .result
            .ok_or(ExchangeError::FeedNotInitialized)? as u8;
        msg!("Result feed is {}", result_u8);
        if result_u8 > 2 {
            return Err(ExchangeError::NotValidResult.into());
        }
        market_state.result = MoneylineMarketOutcome::unpack(&result_u8)?;

        // Loosing market sides get 0 payout
        let winner_payout = market_state.market_sides[market_state.result as usize].payout;
        let winner_risk = market_state.market_sides[market_state.result as usize].risk;
        let loser_risk = match market_state.result {
            MoneylineMarketOutcome::MarketSide0Won => market_state.market_sides[1]
                .risk
                .checked_add(market_state.market_sides[2].risk)
                .ok_or(ExchangeError::AmountOverflow)?,
            MoneylineMarketOutcome::MarketSide1Won => market_state.market_sides[0]
                .risk
                .checked_add(market_state.market_sides[2].risk)
                .ok_or(ExchangeError::AmountOverflow)?,
            MoneylineMarketOutcome::MarketSide2Won => market_state.market_sides[0]
                .risk
                .checked_add(market_state.market_sides[1].risk)
                .ok_or(ExchangeError::AmountOverflow)?,
            _ => return Err(ExchangeError::NotValidResult.into()),
        };

        //When the market settles the bettor balance changes from the amount of risk the bettors
        //have entered into the market to the winning sides unsettled risk and payout.
        let current_bettor_balance = market_state.bettor_balance;
        let new_bettor_balance = winner_risk
            .checked_add(winner_payout)
            .ok_or(ExchangeError::AmountOverflow)?;
        market_state.bettor_balance = new_bettor_balance;
        pool_state.bettor_balance = pool_state
            .bettor_balance
            .checked_add(new_bettor_balance)
            .ok_or(ExchangeError::AmountOverflow)?
            .checked_sub(current_bettor_balance)
            .ok_or(ExchangeError::AmountOverflow)?;

        // Calculate locked liquidity after losers lose payout
        let new_locked_liquidity = 0u64;
        let current_locked_liquidity = market_state.locked_liquidity;
        let current_available_liquidity = pool_state.available_liquidity;
        market_state.locked_liquidity = 0u64;
        pool_state.available_liquidity = current_available_liquidity
            .checked_add(current_locked_liquidity)
            .ok_or(ExchangeError::AmountOverflow)?
            .checked_add(loser_risk)
            .ok_or(ExchangeError::AmountOverflow)?
            .checked_sub(winner_payout)
            .ok_or(ExchangeError::AmountOverflow)?;

        msg!(
            "Market locked liquidity from {} to {}",
            current_locked_liquidity,
            new_locked_liquidity
        );
        msg!(
            "Pool available liquidity from {} to {}",
            current_available_liquidity,
            pool_state.available_liquidity
        );

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
        //let mint = next_account_info(accounts_iter)?;
        //let token_program = next_account_info(accounts_iter)?;
        //let pda_account = next_account_info(accounts_iter)?;
        //let hp_usdt_account = next_account_info(accounts_iter)?;
        let hp_state_account = next_account_info(accounts_iter)?;

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
        let mut hp_state = HpLiquidity::unpack_unchecked(&hp_state_account.data.borrow())?;
        if hp_state.is_initialized {
            return Err(ExchangeError::HpLiquidityAlreadyInitialized.into());
        }
        hp_state.is_initialized = true;
        HpLiquidity::pack(hp_state, &mut hp_state_account.data.borrow_mut())?;
        Ok(())
    }
}
