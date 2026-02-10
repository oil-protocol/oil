use oil_api::prelude::*;
use oil_api::instruction::CheckpointAuction;
use solana_program::{log::sol_log, native_token::lamports_to_sol};
use steel::*;

/// User checkpoints their auction rewards for multiple wells/epochs
/// Supports batch checkpointing: well_mask allows checkpointing multiple wells in a single instruction
pub fn process_checkpoint_auction(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    let args = CheckpointAuction::try_from_bytes(data)?;
    let well_mask = args.well_mask;
    
    // Count how many wells are being checkpointed
    let num_wells = (well_mask & 0x0F).count_ones() as usize;
    if num_wells == 0 {
        return Err(ProgramError::InvalidArgument); // No wells selected
    }
    if num_wells > 4 {
        return Err(ProgramError::InvalidArgument); // Invalid well_mask
    }
    
    // Account order: signer, authority, miner, [share, micro, well for each well], oil_program
    let expected_len = 3 + (num_wells * 3) + 1; // 3 base + 3 per well + 1 program
    if accounts.len() < expected_len {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    let mut accounts_iter = accounts.iter();
    oil_api::extract_accounts!(accounts_iter, [s, a, m]);
    let (signer_info, authority_info, miner_info) = (s, a, m);
    
    signer_info.is_signer()?;
    let authority = *authority_info.key;
    
    // Validate miner account
    miner_info.is_writable()?.has_seeds(&[MINER, &authority.to_bytes()], &oil_api::ID)?;
    let miner = miner_info.as_account_mut::<Miner>(&oil_api::ID)?;
    miner.assert_mut(|m| m.authority == authority)?;
    
    // Get oil_program (last account)
    let oil_program = accounts.last().ok_or(ProgramError::NotEnoughAccountKeys)?;
    oil_program.is_program(&oil_api::ID)?;
    
    // Process each well in the mask
    for well_id in 0..4 {
        if (well_mask & (1 << well_id)) == 0 {
            continue; // Skip this well
        }
        
        let epoch_id = u64::from_le_bytes(args.epoch_ids[well_id]);
        
        // Get accounts for this well: share, micro, well
        let share_info = accounts_iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?;
        let micro_info = accounts_iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?;
        let well_info = accounts_iter.next().ok_or(ProgramError::NotEnoughAccountKeys)?;
        
        well_info.has_seeds(&[WELL, &(well_id as u64).to_le_bytes()], &oil_api::ID)?;
        let well = well_info.as_account::<Well>(&oil_api::ID)?;
        
        // Can't checkpoint current/active epoch
        if epoch_id >= well.epoch_id {
            return Err(ProgramError::InvalidArgument); // CannotCheckpointCurrentEpoch
        }
        
        // Check if user was a contributor (has Share account) or operator (no Share account)
        let is_contributor = !share_info.data_is_empty();
        
        if is_contributor {
            // User was a contributor - process Share rewards
            // Micro account must exist for contributors (it's created when pool activity occurs)
            if micro_info.data_is_empty() {
                return Err(ProgramError::InvalidAccountData); // Micro account doesn't exist - no pool activity for this epoch
            }
            
            micro_info.has_seeds(&[MICRO, &(well_id as u64).to_le_bytes(), &epoch_id.to_le_bytes()], &oil_api::ID)?;
            let micro = micro_info.as_account::<Micro>(&oil_api::ID)?;
            
            share_info.is_writable()?.has_seeds(&[SHARE, &authority.to_bytes(), &(well_id as u64).to_le_bytes(), &epoch_id.to_le_bytes()], &oil_api::ID)?;
            let share = share_info.as_account_mut::<Share>(&oil_api::ID)?;
            share.assert_mut(|s| s.authority == authority && s.well_id == well_id as u64 && s.epoch_id == epoch_id)?;
    
    // Validations
    if share.well_id != micro.well_id {
        return Err(ProgramError::InvalidAccountData);
    }
    if share.epoch_id != micro.epoch_id {
        return Err(ProgramError::InvalidAccountData);
    }
    if share.contribution == 0 {
        return Err(ProgramError::InvalidArgument); // NoContribution
    }
    if share.claimed_oil != 0 {
        return Err(ProgramError::InvalidArgument); // AlreadyClaimed
    }
    
    // Can only checkpoint the epoch user participated in
    if miner.current_epoch_id[well_id] != 0 && share.epoch_id != miner.current_epoch_id[well_id] {
        return Err(ProgramError::InvalidArgument); // InvalidEpoch
    }
    
    // Calculate user's share of OIL and refund
    let user_oil = if micro.total_contributed > 0 {
        ((share.contribution as u128 * micro.total_oil_mined as u128) 
         / micro.total_contributed as u128) as u64
    } else {
        0
    };
    
    let user_refund = if micro.total_contributed > 0 {
        ((share.contribution as u128 * micro.total_refund as u128) 
         / micro.total_contributed as u128) as u64
    } else {
        0
    };
    
    // Update miner rewards
    miner.auction_rewards_oil = miner.auction_rewards_oil
        .checked_add(user_oil)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    miner.auction_rewards_sol = miner.auction_rewards_sol
        .checked_add(user_refund)
        .ok_or(ProgramError::ArithmeticOverflow)?;
    
    // Mark share as claimed
    share.claimed_oil = user_oil;
    share.claimed_sol = user_refund;
    
    sol_log(&format!(
                "CheckpointAuction (Contributor): well_id={}, epoch_id={}, oil={}, refund={} SOL",
        well_id,
        epoch_id,
        user_oil,
        lamports_to_sol(user_refund)
    ));
        } else {
            // User was an operator (no Share account) - just update checkpointed_epoch_id
            // Operators already received their rewards when someone else bid (in place_bid)
            // We just need to allow them to place another bid by updating checkpointed_epoch_id
            // Micro account may not exist if there was no pool activity - that's fine
            
            // Validate that user participated in this epoch or later
            if miner.current_epoch_id[well_id] != 0 && miner.current_epoch_id[well_id] < epoch_id {
                return Err(ProgramError::InvalidArgument); // User didn't participate in this epoch as operator
            }
            
            sol_log(&format!(
                "CheckpointAuction (Operator): well_id={}, epoch_id={}, current_epoch_id={}",
                well_id,
                epoch_id,
                miner.current_epoch_id[well_id]
            ));
        }
        
        // Update miner checkpointed epoch (for both contributors and operators)
        // Important: If user has already moved to a later epoch (current_epoch_id > epoch_id),
        // we should set checkpointed_epoch_id to at least current_epoch_id to satisfy place_bid validation
        if miner.current_epoch_id[well_id] > epoch_id {
            // User has moved to a later epoch, so checkpointed_epoch_id should be at least current_epoch_id
            miner.checkpointed_epoch_id[well_id] = miner.current_epoch_id[well_id];
        } else {
            // User is checkpointing their current or previous epoch
            miner.checkpointed_epoch_id[well_id] = epoch_id;
        }
    }
    
    Ok(())
}
