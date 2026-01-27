use oil_api::prelude::*;
use oil_api::consts::{SOL_MINT, AUCTION_FLOOR_PRICE};
use oil_api::instruction::PlaceBid;
use oil_api::utils::create_or_validate_wrapped_sol_ata;
use solana_program::{log::sol_log, native_token::lamports_to_sol, program::invoke};
use solana_program::program::invoke_signed as solana_invoke_signed;
use fogo_sessions_sdk::token::instruction::transfer_checked;
use fogo_sessions_sdk::token::PROGRAM_SIGNER_SEED;
use spl_token::instruction::close_account;
use steel::*;

/// Direct solo bid on an auction well (seize ownership)
pub fn process_place_bid(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    let clock = Clock::get()?;
    let args = PlaceBid::try_from_bytes(data)?;
    let well_id = u64::from_le_bytes(args.square_id) as usize;
    let referrer = Pubkey::new_from_array(args.referrer);
    
    if well_id >= 4 {
        return Err(ProgramError::InvalidArgument);
    }

    let is_fogo_session = accounts.len() >= 2 && accounts[0].key != accounts[1].key;
    let base_accounts_count = if is_fogo_session { 20 } else { 18 };
    let has_referral = referrer != Pubkey::default();
    let referral_offset = if has_referral { 1 } else { 0 };
    let wrapped_token_accounts_count = 5;
    let min_accounts = base_accounts_count + referral_offset + wrapped_token_accounts_count;
    
    if accounts.len() < min_accounts {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // Parse base accounts
    let (signer_info, authority_info, program_signer_info_opt, payer_info_opt, well_info, auction_info, 
         treasury_info, treasury_tokens_info, mint_info, mint_authority_info, mint_program, staking_pool_info, 
         fee_collector_info, config_info, token_program, system_program, oil_program, bidder_miner_info, 
         previous_owner_miner_info, _previous_owner_info, referral_info_opt) = 
        if is_fogo_session {
            let expected_len = 20 + if has_referral { 1 } else { 0 };
            if accounts.len() < expected_len {
                return Err(ProgramError::NotEnoughAccountKeys);
            }
            let mut accounts_iter = accounts.iter();
            oil_api::extract_accounts!(accounts_iter, [s, a, ps, pay, w, au, t, tt, m, ma, mp, sp, fc, c, tp, sys, op, bm, pom, po]);
            let ref_info = if has_referral { accounts_iter.next() } else { None };
            (s, a, Some(ps), Some(pay), w, au, t, tt, m, ma, mp, sp, fc, c, tp, sys, op, bm, pom, po, ref_info)
        } else {
            let expected_len = 18 + if has_referral { 1 } else { 0 };
            if accounts.len() < expected_len {
                return Err(ProgramError::NotEnoughAccountKeys);
            }
            let mut accounts_iter = accounts.iter();
            oil_api::extract_accounts!(accounts_iter, [s, a, w, au, t, tt, m, ma, mp, sp, fc, c, tp, sys, op, bm, pom, po]);
            let ref_info = if has_referral { accounts_iter.next() } else { None };
            (s, a, None, None, w, au, t, tt, m, ma, mp, sp, fc, c, tp, sys, op, bm, pom, po, ref_info)
        };

    signer_info.is_signer()?;
    let authority = *authority_info.key;
    
    // Parse wrapped token accounts (both Fogo sessions and regular wallets use wrapped SOL)
    let wrapped_start = base_accounts_count + referral_offset;
    let wrapped_end = wrapped_start + wrapped_token_accounts_count;
    if accounts.len() < wrapped_end {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    let [user_wrapped_sol_info, treasury_wrapped_sol_info, token_program_wrapped, mint_info_wrapped, 
         associated_token_program_wrapped] = &accounts[wrapped_start..wrapped_end] else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    
    // Validate accounts
    let well = well_info.is_writable()?
        .has_seeds(&[WELL, &(well_id as u64).to_le_bytes()], &oil_api::ID)?
        .as_account_mut::<Well>(&oil_api::ID)?;
    auction_info.is_writable()?.has_seeds(&[AUCTION], &oil_api::ID)?;
    let auction = auction_info.as_account_mut::<Auction>(&oil_api::ID)?;
    treasury_info.is_writable()?.has_seeds(&[TREASURY], &oil_api::ID)?;
    let treasury = treasury_info.as_account_mut::<Treasury>(&oil_api::ID)?;
    let config = config_info.as_account::<Config>(&oil_api::ID)?;
    let is_premine = oil_api::utils::is_premine_active(&config, &clock);
    
    mint_info.has_address(&MINT_ADDRESS)?.as_mint()?;
    treasury_tokens_info.as_associated_token_account(&treasury_info.key, &mint_info.key)?;
    mint_authority_info.as_account::<oil_mint_api::state::Authority>(&oil_mint_api::ID)?;
    mint_program.is_program(&oil_mint_api::ID)?;
    staking_pool_info.is_writable()?;
    let staking_pool = staking_pool_info.as_account_mut::<Pool>(&oil_api::ID)?;
    fee_collector_info.is_writable()?.has_address(&config.fee_collector)?;
    token_program.is_program(&spl_token::ID)?;
    system_program.is_program(&system_program::ID)?;
    oil_program.is_program(&oil_api::ID)?;
    
    // Create miner if new
    let is_new_miner = bidder_miner_info.data_is_empty();
    if is_new_miner {
        bidder_miner_info.is_writable()?.has_seeds(&[MINER, &authority.to_bytes()], &oil_api::ID)?;

        let account_creation_payer = if is_fogo_session {
            payer_info_opt.unwrap_or(signer_info)
        } else {
            signer_info
        };
        create_program_account::<Miner>(
            bidder_miner_info,
            system_program,
            account_creation_payer,
            &oil_api::ID,
            &[MINER, &authority.to_bytes()],
        )?;
        let miner = bidder_miner_info.as_account_mut::<Miner>(&oil_api::ID)?;
        miner.initialize(authority);
        
        if referrer != Pubkey::default() && referrer != authority {
            miner.referrer = referrer;
            Referral::process_new_miner_referral(
                referral_info_opt,
                referrer,
                authority,
            )?;
        }
    }
    
    well.update_accumulated_oil(&clock);
    well.check_and_apply_halving(auction, &clock);

    let current_price = well.current_price(auction, &clock);
    let is_at_floor = current_price == AUCTION_FLOOR_PRICE;
    
    let bid_amount = current_price.checked_add(1).ok_or(ProgramError::ArithmeticOverflow)?;

    let previous_owner = well.current_bidder;
    let has_previous_owner = previous_owner != Pubkey::default();
    let accumulated_oil = if has_previous_owner { well.accumulated_oil } else { 0 };

    let buyback_amount = (bid_amount * 7) / 100; // 7% buyback & burn
    let liquidity_amount = (bid_amount * 3) / 100; // 3% liquidity
    let staking_amount = (bid_amount * 3) / 100; // 3% staking
    // During premine: 2% dev fee, otherwise 1% (extra 1% comes from previous owner's refund)
    let dev_fee_rate = if is_premine { 2 } else { 1 };
    let dev_fee_amount = (bid_amount * dev_fee_rate) / 100;
    let total_protocol_revenue = buyback_amount + liquidity_amount + staking_amount + dev_fee_amount;
    let previous_owner_amount = bid_amount.saturating_sub(total_protocol_revenue);

    // For Fogo sessions, validate program_signer
    let program_signer_with_bump_opt = if is_fogo_session {
        let program_signer_info = program_signer_info_opt.ok_or(ProgramError::InvalidAccountData)?;
        let (program_signer_pda, bump) = Pubkey::find_program_address(&[PROGRAM_SIGNER_SEED], &oil_api::ID);
        if program_signer_info.key != &program_signer_pda {
            return Err(ProgramError::InvalidArgument);
        }
        Some((program_signer_info, bump))
    } else {
        None
    };
        
        let payer = if is_fogo_session {
            payer_info_opt.unwrap_or(signer_info)
        } else {
            signer_info
        };
        create_or_validate_wrapped_sol_ata(
            user_wrapped_sol_info,
            authority_info,
            mint_info_wrapped,
            payer,
            system_program,
            token_program_wrapped,
            associated_token_program_wrapped,
            None,
        )?;
        
        // Validate token program and mint
        token_program_wrapped.is_program(&spl_token::ID)?;
        mint_info_wrapped.has_address(&SOL_MINT)?.as_mint()?;
        associated_token_program_wrapped.is_program(&spl_associated_token_account::ID)?;
        
        let mint_key = mint_info_wrapped.key;
        
        let payer = if is_fogo_session {
            payer_info_opt.unwrap_or(signer_info)
        } else {
            // For regular wallets, the signer (authority) pays for ATA creation
            signer_info
        };
        
        create_or_validate_wrapped_sol_ata(
            treasury_wrapped_sol_info,
            treasury_info,
            mint_info_wrapped,
            payer,
            system_program,
            token_program_wrapped,
            associated_token_program_wrapped,
            None,
        )?;
        
        // Calculate total amount to transfer (all wrapped SOL goes to single temp ATA)
        let total_wrapped_amount = total_protocol_revenue + previous_owner_amount;
        
        if total_wrapped_amount > 0 {
            let transfer_ix = transfer_checked(
                token_program_wrapped.key,
                user_wrapped_sol_info.key,
                mint_key,
                treasury_wrapped_sol_info.key,
                signer_info.key,
                program_signer_with_bump_opt.map(|(ps, _)| ps.key),
                total_wrapped_amount,
                9,
            )?;
            
            if let Some((program_signer_info, bump)) = program_signer_with_bump_opt {
                // Fogo session: use invoke_signed with program_signer
                solana_invoke_signed(
                    &transfer_ix,
                    &[
                        user_wrapped_sol_info.clone(),
                        mint_info_wrapped.clone(),
                        treasury_wrapped_sol_info.clone(),
                        signer_info.clone(),
                        token_program_wrapped.clone(),
                        program_signer_info.clone(),
                    ],
                    &[&[PROGRAM_SIGNER_SEED, &[bump]]],
                )?;
            } else {
                // Regular wallet: use regular invoke (signer is authority)
                invoke(
                    &transfer_ix,
                    &[
                        user_wrapped_sol_info.clone(),
                        mint_info_wrapped.clone(),
                        treasury_wrapped_sol_info.clone(),
                        signer_info.clone(),
                        token_program_wrapped.clone(),
                    ],
                )?;
            }
            
            // Close the temp ATA to unwrap all SOL into treasury account
            let close_ix = close_account(
                token_program_wrapped.key,
                treasury_wrapped_sol_info.key,
                treasury_info.key,      // Destination for native SOL from closing ATA
                treasury_info.key,       // Owner (treasury PDA)
                &[],
            )?;
            let treasury_seeds: &[&[u8]] = &[TREASURY];
            invoke_signed(
                &close_ix,
                &[
                    treasury_wrapped_sol_info.clone(),
                    treasury_info.clone(), // Destination for native SOL
                    treasury_info.clone(), // Owner
                    token_program_wrapped.clone(),
                ],
                &oil_api::ID,
                treasury_seeds,
            )?;
            
            treasury.balance += buyback_amount;
            treasury.liquidity += liquidity_amount;
            if has_previous_owner && previous_owner_amount > 0 {
                treasury.auction_rewards_sol += previous_owner_amount;
            }
    }
    
    // Update previous owner miner (if has previous owner)
    if has_previous_owner {
        previous_owner_miner_info.is_writable()?
            .has_seeds(&[MINER, &previous_owner.to_bytes()], &oil_api::ID)?;
        let previous_owner_miner = previous_owner_miner_info.as_account_mut::<Miner>(&oil_api::ID)?;
        previous_owner_miner.auction_rewards_sol += previous_owner_amount;
    }
    
    if has_previous_owner && accumulated_oil > 0 {
        invoke_signed(
            &oil_mint_api::sdk::mint_oil(accumulated_oil),
            &[
                treasury_info.clone(),
                mint_authority_info.clone(),
                mint_info.clone(),
                treasury_tokens_info.clone(),
                token_program.clone(),
            ],
            &oil_api::ID,
            &[TREASURY],
        )?;
        previous_owner_miner_info.is_writable()?
            .has_seeds(&[MINER, &previous_owner.to_bytes()], &oil_api::ID)?;
        let previous_owner_miner = previous_owner_miner_info.as_account_mut::<Miner>(&oil_api::ID)?;
        previous_owner_miner.auction_rewards_oil += accumulated_oil;
        treasury.auction_total_unclaimed += accumulated_oil;
    }
    
    well.accumulated_oil = 0;

    bidder_miner_info.is_writable()?.has_seeds(&[MINER, &authority.to_bytes()], &oil_api::ID)?;
    let bidder_miner = bidder_miner_info.as_account_mut::<Miner>(&oil_api::ID)?;
    bidder_miner.lifetime_bid = bidder_miner.lifetime_bid
        .checked_add(bid_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    well.epoch_id += 1;
    well.current_bidder = authority;
    well.init_price = if is_at_floor {
        auction.starting_prices[well_id]
    } else {
        current_price * 2
    };
    well.epoch_start_time = clock.unix_timestamp as u64;
    well.accumulated_oil = 0;
    well.operator_total_oil_mined = 0;
    well.last_update_time = clock.unix_timestamp as u64;
    well.mps = auction.base_mining_rates[well_id];
    well.check_and_apply_halving(auction, &clock);
    
    // Validate auction account right before auction_program_log (matching pattern from claim_auction_sol.rs)
    auction_info
        .is_writable()?
        .has_seeds(&[AUCTION], &oil_api::ID)?;
    
    auction_program_log(
        &[auction_info.clone(), oil_program.clone()],
        BidEvent {
            disc: 4,
            authority,
            square_id: well_id as u64,
            bid_amount,
            current_price,
            previous_owner,
            accumulated_oil_transferred: accumulated_oil,
            new_start_price: well.init_price,
            ts: clock.unix_timestamp as u64,
        }
        .to_bytes(),
    )?;

    if bid_amount > 0 {
        let pool_final_amount = if has_previous_owner {
            staking_amount
        } else {
            staking_amount + previous_owner_amount
        };
        
        if pool_final_amount > 0 {
            treasury_info.send(pool_final_amount, staking_pool_info);
            staking_pool.balance += pool_final_amount;
            if staking_pool.total_staked_score > 0 {
                staking_pool.stake_rewards_factor +=
                    Numeric::from_fraction(pool_final_amount, staking_pool.total_staked_score);
            }
        }
        
        if dev_fee_amount > 0 {
            treasury_info.send(dev_fee_amount, fee_collector_info);
        }
    }

    sol_log(&format!(
        "Bid: well_id={}, epoch_id={}, bid_amount={} SOL",
        well_id,
        well.epoch_id - 1,
        lamports_to_sol(bid_amount)
    ));

    Ok(())
}
