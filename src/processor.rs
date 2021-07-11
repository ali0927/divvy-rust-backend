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
    error::ExchangeError,
    initprogram,
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
            ExchangeInstruction::Deposit { usdt_amount, bump_seed } => {
                msg!("Instruction: Deposit");
                Self::process_deposit(accounts, usdt_amount, bump_seed, program_id)
            }
            ExchangeInstruction::Withdraw { ht_amount, bump_seed } => {
                msg!("Instruction: Withdraw");
                Self::process_withdraw(accounts, ht_amount, bump_seed, program_id)
            }
            ExchangeInstruction::Initbet { risk, odds, market_side } => {
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
                initprogram::process_ownership(accounts, bump_seed, program_id)
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
        let hp_mint_account = next_account_info(accounts_iter)?;
        let token_program = next_account_info(accounts_iter)?;
        let token_account = next_account_info(accounts_iter)?;
        let pda_account = next_account_info(accounts_iter)?;
        let user_usdt_account = next_account_info(accounts_iter)?;
        let hp_usdt_account = next_account_info(accounts_iter)?;
        let hp_state_account = next_account_info(accounts_iter)?;

        let mut hp_state = HpLiquidity::unpack(&hp_state_account.data.borrow())?;
        let hp_mint_state = TokenMint::unpack(&hp_mint_account.data.borrow())?;
    
        let conversion_ratio = hp_mint_state.supply as f64 / hp_state.available_liquidity as f64;
        let ht_amount = (usdt_amount as f64 * conversion_ratio).floor() as u64;
        
        msg!("- USDT amount");
        msg!(0, 0, 0, 0, usdt_amount);
        msg!("- HT amount");
        msg!(0, 0, 0, 0, ht_amount);

        //let (_pda, bump_seed) = Pubkey::find_program_address(&[b"divvyexchange"], program_id);
        let transfer_instruction = transfer(
            &token_program.key,
            &user_usdt_account.key,
            &hp_usdt_account.key,
            &user_account.key,
            &[&user_account.key],
            usdt_amount.clone(),
        )?;
        msg!("Calling the token program to transfer tokens...");
        invoke(
            &transfer_instruction,
            &[
                user_usdt_account.clone(),
                hp_usdt_account.clone(),
                user_account.clone(),
                token_program.clone(),
            ],
        )?;

        msg!("Creating mint instruction");
        let mint_ix = mint_to(
            &token_program.key,
            &hp_mint_account.key,
            &token_account.key,
            &pda_account.key,
            &[&pda_account.key],
            ht_amount,
        )?;

        invoke_signed(
            &mint_ix,
            &[hp_mint_account.clone(), token_account.clone(), pda_account.clone()],
            &[&[b"divvyexchange", &[bump_seed]]],
        )?;

        msg!("Adding deposit to liquidity");
        hp_state.available_liquidity = hp_state.available_liquidity + usdt_amount;
        HpLiquidity::pack(hp_state, &mut hp_state_account.data.borrow_mut())?;

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
        let hp_mint_account = next_account_info(accounts_iter)?;
        let token_program = next_account_info(accounts_iter)?;
        let user_hp_account = next_account_info(accounts_iter)?;
        let pda_account = next_account_info(accounts_iter)?;
        let user_usdt_account = next_account_info(accounts_iter)?;
        let hp_usdt_account = next_account_info(accounts_iter)?;
        let hp_state_account = next_account_info(accounts_iter)?;

        let mut hp_state = HpLiquidity::unpack(&hp_state_account.data.borrow())?;
        let hp_mint_state = TokenMint::unpack(&hp_mint_account.data.borrow())?;
    
        let conversion_ratio = hp_state.available_liquidity as f64 / hp_mint_state.supply as f64;
        let usdt_amount = (ht_amount as f64 * conversion_ratio).floor() as u64;

        msg!("- USDT amount");
        msg!(0, 0, 0, 0, usdt_amount);
        msg!("- HT amount");
        msg!(0, 0, 0, 0, ht_amount);

        //Burn the transfers
        let burn_tx = burn(
            &token_program.key,
            &user_hp_account.key,
            &hp_mint_account.key,
            &user_account.key,
            &[&user_account.key],
            ht_amount,
        )?;

        invoke(
            &burn_tx,
            &[
                token_program.clone(),
                user_hp_account.clone(),
                hp_mint_account.clone(),
                user_account.clone(),
            ],
        )?;

        //Transfer Withdraw
        let transfer_instruction = transfer(
            &token_program.key,
            &hp_usdt_account.key,
            &user_usdt_account.key,
            &pda_account.key,
            &[&pda_account.key],
            usdt_amount.clone(),
        )?;
        msg!("Calling the token program to transfer tokens...");
        invoke_signed(
            &transfer_instruction,
            &[
                hp_usdt_account.clone(),
                user_usdt_account.clone(),
                pda_account.clone(),
                token_program.clone(),
            ],
            &[&[b"divvyexchange", &[bump_seed]]],
        )?;

        msg!("Subtracting withdrawal from liquidity");
        hp_state.available_liquidity = hp_state.available_liquidity - usdt_amount;
        HpLiquidity::pack(hp_state, &mut hp_state_account.data.borrow_mut())?;

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
        let hp_state_account = next_account_info(accounts_iter)?;
        let hp_usdt_account = next_account_info(accounts_iter)?;
        let user_usdt_account = next_account_info(accounts_iter)?;
        let token_program = next_account_info(accounts_iter)?;
        
        msg!("Validating accounts");
        //Checking if market is initialized
        let mut market_state = Market::unpack(&market_state_account.data.borrow())
            .map_err(|_| Into::<ProgramError>::into(ExchangeError::MarketNotInitialized))?;
        
        let mut hp_state = HpLiquidity::unpack_unchecked(&hp_state_account.data.borrow())?;
        
        let mut bet_state = Bet::unpack_unchecked(&bet_account.data.borrow())?;
        if bet_state.is_initialized {
            return Err(ExchangeError::BetAlreadyInitialized.into())
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

        //Getting odds from the Switchboard
        let aggregator: AggregatorState = get_aggregator(feed_account)?;
        let round_result: RoundResult = get_aggregator_result(&aggregator)?;
        let feed_odds: f64 = round_result.result.ok_or(ExchangeError::FeedNotInitialized)?;
        msg!("Odds from feed are {}", feed_odds as u64);

        //To Do comparison of provided odds & feed odds.

        //Calculate payout
        let payout = Self::calculate_payout(feed_odds, risk);
        msg!("Bet payout is {}", payout);

        // Increment pending bets
        msg!("Incrementing market pending bets.");
        market_state.pending_bets = market_state.pending_bets
            .checked_add(1)
            .ok_or(ExchangeError::AmountOverflow)?;

        msg!("Incrementing house pool pending bets.");
        hp_state.pending_bets = hp_state.pending_bets
            .checked_add(1)
            .ok_or(ExchangeError::AmountOverflow)?;
        
        // Increment market bettor balance
        market_state.bettor_balance = market_state.bettor_balance
        .checked_add(risk)
        .ok_or(ExchangeError::AmountOverflow)?;

        hp_state.bettor_balance = hp_state.bettor_balance
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
        let new_locked_liquidity = Self::calculate_locked_liquidity(&market_state)?;
        let current_available_liquidity = hp_state.available_liquidity;
        let current_locked_liquidity = market_state.locked_liquidity;
        market_state.locked_liquidity = new_locked_liquidity;
        hp_state.available_liquidity = current_available_liquidity
            .checked_add(current_locked_liquidity)
            .ok_or(ExchangeError::AmountOverflow)?
            .checked_sub(new_locked_liquidity)
            .ok_or(ExchangeError::NotEnoughLiquidity)?;
        msg!("Market locked liquidity from {} to {}", current_locked_liquidity, new_locked_liquidity);
        msg!("Pool available liquidity from {} to {}", current_available_liquidity, hp_state.available_liquidity);

        //Transfer token from user account to hp account
        let transfer_instruction = transfer(
            &token_program.key,
            &user_usdt_account.key,
            &hp_usdt_account.key,
            &initializer.key,
            &[&initializer.key],
            risk.clone(),
        )?;
        msg!("Transferring risk from user account to divvy account");
        invoke(
            &transfer_instruction,
            &[
                user_usdt_account.clone(),
                hp_usdt_account.clone(),
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
        HpLiquidity::pack(hp_state, &mut hp_state_account.data.borrow_mut())?;
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
        let hp_usdt_account = next_account_info(accounts_iter)?;
        let user_usdt_account = next_account_info(accounts_iter)?;
        let user_main_account = next_account_info(accounts_iter)?;
        let hp_state_account = next_account_info(accounts_iter)?;

        let mut hp_state = HpLiquidity::unpack(&hp_state_account.data.borrow())?;
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
        market_state.pending_bets = market_state.pending_bets
            .checked_sub(1)
            .ok_or(ExchangeError::AmountOverflow)?;

        msg!("Decrementing house pool pending bets.");
        hp_state.pending_bets = hp_state.pending_bets
            .checked_sub(1)
            .ok_or(ExchangeError::AmountOverflow)?;

        if bet_state.user_market_side != market_state.result.pack() {
            bet_state.outcome = 2; //User have lost
        } else {
            bet_state.outcome = 1; //User have won
            let bet_balance = bet_state.user_risk
                .checked_add(bet_state.user_payout)
                .ok_or(ExchangeError::AmountOverflow)?;

            // Subtract bettor balance in the market and house pool
            // Only for winning bets, as when the market settles,
            // the balance is changed to only include the winning sides risk and payout
            market_state.bettor_balance = market_state.bettor_balance
                .checked_sub(bet_balance)
                .ok_or(ExchangeError::AmountOverflow)?;
            hp_state.bettor_balance = hp_state.bettor_balance
                .checked_sub(bet_balance)
                .ok_or(ExchangeError::AmountOverflow)?;

            //Remove risk & payout in market side. Only for winning bets, as locked 
            // liquidity was already calculated for losers.
            let current_market_side_risk = market_state.market_sides[bet_state.user_market_side as usize].risk;
            let current_market_side_payout = market_state.market_sides[bet_state.user_market_side as usize].payout;
            market_state.market_sides[bet_state.user_market_side as usize].risk = current_market_side_risk
                .checked_sub(bet_state.user_risk)
                .ok_or(ExchangeError::MarketSideRiskUnderflow)?;
            market_state.market_sides[bet_state.user_market_side as usize].payout = current_market_side_payout
                .checked_sub(bet_state.user_payout)
                .ok_or(ExchangeError::MarketSidePayoutUnderflow)?;

            let transfer_instruction = transfer(
                &token_program.key,
                &hp_usdt_account.key,
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
                    hp_usdt_account.clone(),
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
        if hp_state.pending_bets == 0 {
            msg!("House pool pending bets are settled. Asserting.");
            if hp_state.bettor_balance != 0 {
                return Err(ExchangeError::HousePoolBettorBalanceRemaining.into());
            }
            let hp_usdt = TokenAccount::unpack(&hp_usdt_account.data.borrow())?;
            if hp_state.available_liquidity != hp_usdt.amount {
                return Err(ExchangeError::UnexpectedAvailableLiquidity.into());
            }
        }

        HpLiquidity::pack(hp_state, &mut hp_state_account.data.borrow_mut())?;
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
        let hp_state_account = next_account_info(accounts_iter)?;
        let result_account = next_account_info(accounts_iter)?;

        let mut market_state = Market::unpack_unchecked(&market_state_account.data.borrow())?;
        let mut hp_state = HpLiquidity::unpack_unchecked(&hp_state_account.data.borrow())?;

        //Verifying result account
        if result_account.key != &market_state.result_feed {
            return Err(ExchangeError::NotValidAuthority.into());
        }
        //Checking if market is not settled yet
        if market_state.result != MoneylineMarketOutcome::NotYetSettled {
            return Err(ExchangeError::MarketAlreadySettled.into());
        }
        //Getting results from Switchboard
        let aggregator: AggregatorState = get_aggregator(result_account)?;
        let round_result: RoundResult = get_aggregator_result(&aggregator)?;
        let result_u8 = round_result.result.ok_or(ExchangeError::FeedNotInitialized)? as u8;
        msg!("Result feed is {}", result_u8);
        if result_u8 > 2 {
            return Err(ExchangeError::NotValidResult.into());
        }
        market_state.result = MoneylineMarketOutcome::unpack(&result_u8)?;
        
        // Loosing market sides get 0 payout
        let winner_payout = market_state.market_sides[market_state.result as usize].payout;
        let winner_risk = market_state.market_sides[market_state.result as usize].risk;
        let loser_risk = match market_state.result {
            MoneylineMarketOutcome::MarketSide0Won => market_state.market_sides[1].risk
                .checked_add(market_state.market_sides[2].risk)
                .ok_or(ExchangeError::AmountOverflow)?,
            MoneylineMarketOutcome::MarketSide1Won => market_state.market_sides[0].risk
                .checked_add(market_state.market_sides[2].risk)
                .ok_or(ExchangeError::AmountOverflow)?,
            MoneylineMarketOutcome::MarketSide2Won => market_state.market_sides[0].risk
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
        hp_state.bettor_balance = hp_state.bettor_balance
            .checked_add(new_bettor_balance)
            .ok_or(ExchangeError::AmountOverflow)?
            .checked_sub(current_bettor_balance)
            .ok_or(ExchangeError::AmountOverflow)?;
        
        // Calculate locked liquidity after losers lose payout
        let new_locked_liquidity = 0u64;
        let current_locked_liquidity = market_state.locked_liquidity;
        let current_available_liquidity = hp_state.available_liquidity;
        market_state.locked_liquidity = 0u64;
        hp_state.available_liquidity = current_available_liquidity
            .checked_add(current_locked_liquidity)
            .ok_or(ExchangeError::AmountOverflow)?
            .checked_add(loser_risk)
            .ok_or(ExchangeError::AmountOverflow)?
            .checked_sub(winner_payout)
            .ok_or(ExchangeError::AmountOverflow)?;

        msg!("Market locked liquidity from {} to {}", current_locked_liquidity, new_locked_liquidity);
        msg!("Pool available liquidity from {} to {}", current_available_liquidity, hp_state.available_liquidity);

        Market::pack(market_state, &mut market_state_account.data.borrow_mut())?;
        HpLiquidity::pack(hp_state, &mut hp_state_account.data.borrow_mut())?;
        
        Ok(())
    }

    // Helpers
    fn calculate_payout(odds: f64, risk: u64) -> u64 {
        if odds < 0.0 {
            return 100 / ((-odds as u64) * risk);
        } else {
            return odds as u64 * risk / 100;
        }
    }

    fn calculate_locked_liquidity(market_state: &Market) -> Result<u64, ExchangeError> {
        //Calculating max loss
        let mut locked_side_0 = 0u64;
        let mut locked_side_1 = 0u64;
        let mut locked_side_2 = 0u64;

        if market_state.market_sides[0].payout >
            market_state.market_sides[1].risk
            .checked_add(market_state.market_sides[2].risk)
            .ok_or(ExchangeError::AmountOverflow)?
        {
            locked_side_0 = market_state.market_sides[0].payout
                .checked_sub(market_state.market_sides[1].risk)
                .ok_or(ExchangeError::AmountOverflow)?
                .checked_sub(market_state.market_sides[2].risk)
                .ok_or(ExchangeError::AmountOverflow)?;
        };
        if market_state.market_sides[1].payout >
            market_state.market_sides[0].risk
            .checked_add(market_state.market_sides[2].risk)
            .ok_or(ExchangeError::AmountOverflow)?
        {
            locked_side_1 = market_state.market_sides[1].payout
                .checked_sub(market_state.market_sides[0].risk)
                .ok_or(ExchangeError::AmountOverflow)?
                .checked_sub(market_state.market_sides[2].risk)
                .ok_or(ExchangeError::AmountOverflow)?;
        };

        if market_state.market_sides[2].payout >
            market_state.market_sides[0].risk
            .checked_add(market_state.market_sides[1].risk)
            .ok_or(ExchangeError::AmountOverflow)?
        {
            locked_side_2 = market_state.market_sides[2].payout
                .checked_sub(market_state.market_sides[0].risk)
                .ok_or(ExchangeError::AmountOverflow)?
                .checked_sub(market_state.market_sides[1].risk)
                .ok_or(ExchangeError::AmountOverflow)?;
        };

        let locked_liquidity = *[locked_side_0, locked_side_1, locked_side_2]
            .iter().max()
            .ok_or(ExchangeError::InvalidInstruction)?;
        
        return Ok(locked_liquidity);
    }
}
