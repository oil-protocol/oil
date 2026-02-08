use oil_api::prelude::*;
use oil_api::consts::{AUCTION_FLOOR_PRICE, POOL_ADDRESS};
use oil_api::instruction::PlaceBid;
use solana_program::{log::sol_log, native_token::lamports_to_sol};
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

    let has_referral = referrer != Pubkey::default();
    // Account order: signer, authority, well, auction, treasury, treasury_tokens, mint, mint_authority, mint_program,
    // staking_pool, fee_collector, config, token_program, system_program, oil_program, bidder_miner, previous_owner_miner,
    // rig, micro, referral (optional)
    let expected_len = 19 + if has_referral { 1 } else { 0 };
    
    if accounts.len() < expected_len {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    let mut accounts_iter = accounts.iter();
    oil_api::extract_accounts!(accounts_iter, [s, a, w, au, t, tt, m, ma, mp, sp, fc, c, tp, sys, op, bm, pom, r, mic]);
    let ref_info = if has_referral { accounts_iter.next() } else { None };
    let (signer_info, authority_info, well_info, auction_info, 
         treasury_info, treasury_tokens_info, mint_info, mint_authority_info, mint_program, staking_pool_info, 
         fee_collector_info, config_info, token_program, system_program, oil_program, bidder_miner_info, 
         previous_owner_miner_info, rig_info, micro_info, referral_info_opt) = 
         (s, a, w, au, t, tt, m, ma, mp, sp, fc, c, tp, sys, op, bm, pom, r, mic, ref_info);

    signer_info.is_signer()?;
    let authority = *authority_info.key;
    
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
    
    let is_new_miner = bidder_miner_info.data_is_empty();
    if is_new_miner {
        bidder_miner_info.is_writable()?.has_seeds(&[MINER, &authority.to_bytes()], &oil_api::ID)?;

        create_program_account::<Miner>(
            bidder_miner_info,
            system_program,
            signer_info,
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
    
    // Create or load Rig account
    let rig = if rig_info.data_is_empty() {
        rig_info.is_writable()?.has_seeds(&[RIG, &authority.to_bytes()], &oil_api::ID)?;
        create_program_account::<Rig>(
            rig_info,
            system_program,
            signer_info,
            &oil_api::ID,
            &[RIG, &authority.to_bytes()],
        )?;
        let r = rig_info.as_account_mut::<Rig>(&oil_api::ID)?;
        r.initialize(authority);
        r
    } else {
        rig_info.is_writable()?.has_seeds(&[RIG, &authority.to_bytes()], &oil_api::ID)?;
        let r = rig_info.as_account_mut::<Rig>(&oil_api::ID)?;
        r.assert_mut(|r| r.authority == authority)?;
        r
    };
    
    // Checkpoint requirement: Must checkpoint previous epoch before placing bid
    // Similar to block-based mining: if miner.round_id != round.id, must have checkpointed
    if rig.current_epoch_id[well_id] != 0 && rig.current_epoch_id[well_id] < well.epoch_id {
        assert!(rig.checkpointed_epoch_id[well_id] >= rig.current_epoch_id[well_id], "Miner has not checkpointed previous epoch");
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

    treasury_info.collect(bid_amount, signer_info)?;
    
    treasury.balance += buyback_amount;
    treasury.liquidity += liquidity_amount;
    if has_previous_owner && previous_owner_amount > 0 {
        treasury.auction_rewards_sol += previous_owner_amount;
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
    
    // Create Micro account for previous epoch before incrementing epoch_id
    let previous_epoch_id = well.epoch_id;
    let was_pool_owner = previous_owner == POOL_ADDRESS;
    let pool_never_bid = well.total_contributed > 0 && !was_pool_owner;
    
    if was_pool_owner || pool_never_bid {
        // Create Micro account for previous epoch
        micro_info.is_writable()?.has_seeds(&[MICRO, &well_id.to_le_bytes(), &previous_epoch_id.to_le_bytes()], &oil_api::ID)?;
        
        if micro_info.data_is_empty() {
            create_program_account::<Micro>(
                micro_info,
                system_program,
                signer_info,
                &oil_api::ID,
                &[MICRO, &well_id.to_le_bytes(), &previous_epoch_id.to_le_bytes()],
            )?;
        }
        
        let micro = micro_info.as_account_mut::<Micro>(&oil_api::ID)?;
        
        if was_pool_owner {
            // Pool was owner: calculate original total_contributed
            let original_total = well.total_contributed
                .checked_add(well.pool_bid_cost)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            
            micro.well_id = well_id as u64;
            micro.epoch_id = previous_epoch_id;
            micro.total_contributed = original_total;
            micro.total_oil_mined = accumulated_oil;
            micro.total_refund = previous_owner_amount
                .checked_add(well.total_contributed)
                .ok_or(ProgramError::ArithmeticOverflow)?;
            micro.pool_members = 0; // Optional: can be calculated from Share accounts if needed
            
            // FOGO is already in Treasury, so leftover goes to auction_rewards_sol for distribution
            let leftover = well.total_contributed;
            if leftover > 0 {
                treasury.auction_rewards_sol = treasury.auction_rewards_sol
                    .checked_add(leftover)
                    .ok_or(ProgramError::ArithmeticOverflow)?;
            }
            
            // Decrement Treasury auction_total_pooled by original_total (all pool funds for this epoch)
            treasury.auction_total_pooled = treasury.auction_total_pooled
                .checked_sub(original_total)
                .ok_or(ProgramError::InsufficientFunds)?;
            
            // Reset pool fields
            well.total_contributed = 0;
            well.pool_bid_cost = 0;
        } else if pool_never_bid {
            // Pool existed but never bid: only pool contributions
            micro.well_id = well_id as u64;
            micro.epoch_id = previous_epoch_id;
            micro.total_contributed = well.total_contributed;
            micro.total_oil_mined = 0; // Pool never mined OIL
            micro.total_refund = well.total_contributed; // Only pool contributions
            micro.pool_members = 0;
            
            // FOGO is already in Treasury, so leftover goes to auction_rewards_sol for distribution
            let leftover = well.total_contributed;
            if leftover > 0 {
                treasury.auction_rewards_sol = treasury.auction_rewards_sol
                    .checked_add(leftover)
                    .ok_or(ProgramError::ArithmeticOverflow)?;
            }
            
            // Decrement Treasury auction_total_pooled by pool contributions
            treasury.auction_total_pooled = treasury.auction_total_pooled
                .checked_sub(well.total_contributed)
                .ok_or(ProgramError::InsufficientFunds)?;
            
            // Reset pool field
            well.total_contributed = 0;
        }
    }
    
    well.epoch_id += 1;
    
    // Update Rig current_epoch_id
    rig.current_epoch_id[well_id] = well.epoch_id;
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
