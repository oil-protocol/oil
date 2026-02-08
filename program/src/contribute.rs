use oil_api::prelude::*;
use oil_api::consts::{POOL_ADDRESS, SOL_MINT};
use oil_api::instruction::Contribute;
use oil_api::utils::create_or_validate_wrapped_sol_ata;
use solana_program::{log::sol_log, native_token::lamports_to_sol, program::invoke};
use steel::*;

/// User contributes FOGO to the pool for a specific well
pub fn process_contribute(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    let clock = Clock::get()?;
    let args = Contribute::try_from_bytes(data)?;
    let well_id = u64::from_le_bytes(args.well_id) as usize;
    let amount = u64::from_le_bytes(args.amount);
    
    if well_id >= 4 {
        return Err(ProgramError::InvalidArgument);
    }
    
    if amount == 0 {
        return Err(ProgramError::InvalidArgument);
    }
    
    // Account order: signer, authority, well, auction, treasury, rig, share, 
    // treasury_wrapped_sol_ata, user_wrapped_sol_ata, token_program, mint, associated_token_program, system_program, oil_program
    let expected_len = 14;
    if accounts.len() < expected_len {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    let mut accounts_iter = accounts.iter();
    oil_api::extract_accounts!(accounts_iter, [s, a, w, au, t, r, sh, tws, uws, tp, m, atap, sys, op]);
    let (signer_info, authority_info, well_info, auction_info, treasury_info, 
         rig_info, share_info, treasury_wrapped_sol_info, user_wrapped_sol_info, 
         token_program_info, mint_info, ata_program_info, system_program, oil_program) = 
         (s, a, w, au, t, r, sh, tws, uws, tp, m, atap, sys, op);
    
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
    token_program_info.is_program(&spl_token::ID)?;
    mint_info.has_address(&SOL_MINT)?.as_mint()?;
    ata_program_info.is_program(&spl_associated_token_account::ID)?;
    system_program.is_program(&system_program::ID)?;
    oil_program.is_program(&oil_api::ID)?;
    
    // Validate pool is not already owner
    if well.current_bidder == POOL_ADDRESS {
        return Err(ProgramError::InvalidArgument); // Can't contribute when pool already owns
    }
    
    well.update_accumulated_oil(&clock);
    well.check_and_apply_halving(auction, &clock);
    
    let current_price = well.current_price(auction, &clock);
    let bid_amount = current_price.checked_add(1).ok_or(ProgramError::ArithmeticOverflow)?;
    
    // Calculate actual amount to take (partial contribution logic)
    let required_funds = bid_amount; // Fees are deducted from bid, not added
    let current_total = well.total_contributed;
    let needed = required_funds.saturating_sub(current_total);
    let actual_amount = amount.min(needed);
    
    if actual_amount == 0 {
        return Err(ProgramError::InvalidArgument); // No contribution needed
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
    
    // Checkpoint requirement: Must checkpoint previous epoch before contributing to new epoch
    // Similar to block-based mining: if miner.round_id != round.id, must have checkpointed
    if rig.current_epoch_id[well_id] != 0 && rig.current_epoch_id[well_id] < well.epoch_id {
        assert!(rig.checkpointed_epoch_id[well_id] >= rig.current_epoch_id[well_id], "Miner has not checkpointed previous epoch");
    }
    
    // Create or load Share account
    let share = if share_info.data_is_empty() {
        share_info.is_writable()?.has_seeds(&[SHARE, &authority.to_bytes(), &well_id.to_le_bytes(), &well.epoch_id.to_le_bytes()], &oil_api::ID)?;
        create_program_account::<Share>(
            share_info,
            system_program,
            signer_info,
            &oil_api::ID,
            &[SHARE, &authority.to_bytes(), &well_id.to_le_bytes(), &well.epoch_id.to_le_bytes()],
        )?;
        let s = share_info.as_account_mut::<Share>(&oil_api::ID)?;
        s.initialize(authority, well_id as u64, well.epoch_id, &clock);
        s
    } else {
        share_info.is_writable()?.has_seeds(&[SHARE, &authority.to_bytes(), &well_id.to_le_bytes(), &well.epoch_id.to_le_bytes()], &oil_api::ID)?;
        let s = share_info.as_account_mut::<Share>(&oil_api::ID)?;
        s.assert_mut(|s| s.authority == authority && s.well_id == well_id as u64 && s.epoch_id == well.epoch_id)?;
        s
    };
    
    // Validate user wrapped SOL ATA
    if user_wrapped_sol_info.data_is_empty() {
        return Err(ProgramError::InvalidAccountData);
    }
    user_wrapped_sol_info.as_associated_token_account(authority_info.key, mint_info.key)?;
    
    // Create or validate Treasury wrapped SOL ATA
    create_or_validate_wrapped_sol_ata(
        treasury_wrapped_sol_info,
        treasury_info,
        mint_info,
        signer_info,
        system_program,
        token_program_info,
        ata_program_info,
        None,
    )?;
    
    // Transfer wrapped SOL from user to Treasury ATA
    let user_token_account = user_wrapped_sol_info.as_token_account()?;
    if user_token_account.amount() < actual_amount {
        return Err(ProgramError::InsufficientFunds);
    }
    
    invoke(
        &spl_token::instruction::transfer(
            token_program_info.key,
            user_wrapped_sol_info.key,
            treasury_wrapped_sol_info.key,
            signer_info.key,
            &[],
            actual_amount,
        )?,
        &[
            user_wrapped_sol_info.clone(),
            treasury_wrapped_sol_info.clone(),
            signer_info.clone(),
            token_program_info.clone(),
        ],
    )?;
    
    // Close Treasury's ATA to unwrap to native SOL in Treasury's system account
    let treasury_seeds: &[&[u8]] = &[TREASURY];
    let close_ix = spl_token::instruction::close_account(
        token_program_info.key,
        treasury_wrapped_sol_info.key,
        treasury_info.key,
        treasury_info.key,
        &[],
    )?;
    invoke_signed(
        &close_ix,
        &[
            treasury_wrapped_sol_info.clone(),
            treasury_info.clone(),
            treasury_info.clone(),
            token_program_info.clone(),
        ],
        &oil_api::ID,
        treasury_seeds,
    )?;
    
    // Update Share contribution
    share.contribution = share.contribution
        .checked_add(actual_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    // Update Well total_contributed (for backward compatibility/well-specific tracking)
    well.total_contributed = well.total_contributed
        .checked_add(actual_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    // Update Treasury auction_total_pooled (centralized tracking)
    treasury.auction_total_pooled = treasury.auction_total_pooled
        .checked_add(actual_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    // Update Rig current_epoch_id
    rig.current_epoch_id[well_id] = well.epoch_id;
    
    // Check if pool can bid now (immediate path)
    if well.current_bidder != POOL_ADDRESS && well.total_contributed >= bid_amount {
        // Execute pool bid
        // FOGO is already in Treasury system account, so no transfer needed
        // Just deduct from tracking fields
        
        // Store pool bid cost
        well.pool_bid_cost = bid_amount;
        
        // Decrement Well total_contributed (for backward compatibility)
        well.total_contributed = well.total_contributed
            .checked_sub(bid_amount)
            .ok_or(ProgramError::InsufficientFunds)?;
        
        // Decrement Treasury auction_total_pooled (centralized tracking)
        treasury.auction_total_pooled = treasury.auction_total_pooled
            .checked_sub(bid_amount)
            .ok_or(ProgramError::InsufficientFunds)?;
        
        // Distribute fees (same as regular bid)
        let buyback_amount = (bid_amount * 7) / 100;
        let liquidity_amount = (bid_amount * 3) / 100;
        let staking_amount = (bid_amount * 3) / 100;
        let dev_fee_rate = 1; // Regular dev fee (not premine)
        let dev_fee_amount = (bid_amount * dev_fee_rate) / 100;
        let total_protocol_revenue = buyback_amount + liquidity_amount + staking_amount + dev_fee_amount;
        let previous_owner_amount = bid_amount.saturating_sub(total_protocol_revenue);
        
        treasury.balance += buyback_amount;
        treasury.liquidity += liquidity_amount;
        
        // Update previous owner miner if exists
        if well.current_bidder != Pubkey::default() {
            treasury.auction_rewards_sol += previous_owner_amount;
            // Note: Previous owner miner account not in this instruction, will be updated when they checkpoint
        }
        
        // Pool becomes owner
        well.current_bidder = POOL_ADDRESS;
        
        sol_log(&format!(
            "Pool bid: well_id={}, epoch_id={}, bid_amount={} SOL",
            well_id,
            well.epoch_id,
            lamports_to_sol(bid_amount)
        ));
    }
    
    sol_log(&format!(
        "Contribute: well_id={}, epoch_id={}, amount={} SOL, actual={} SOL",
        well_id,
        well.epoch_id,
        lamports_to_sol(amount),
        lamports_to_sol(actual_amount)
    ));
    
    Ok(())
}
