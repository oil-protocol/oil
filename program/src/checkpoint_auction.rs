use oil_api::prelude::*;
use oil_api::instruction::CheckpointAuction;
use solana_program::{log::sol_log, native_token::lamports_to_sol};
use steel::*;

/// User checkpoints their auction rewards for a specific epoch
pub fn process_checkpoint_auction(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    let args = CheckpointAuction::try_from_bytes(data)?;
    let well_id = u64::from_le_bytes(args.well_id) as usize;
    let epoch_id = u64::from_le_bytes(args.epoch_id);
    
    if well_id >= 4 {
        return Err(ProgramError::InvalidArgument);
    }
    
    // Account order: signer, authority, rig, miner, share, micro, well, oil_program
    let expected_len = 8;
    if accounts.len() < expected_len {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    let mut accounts_iter = accounts.iter();
    oil_api::extract_accounts!(accounts_iter, [s, a, r, m, sh, mic, w, op]);
    let (signer_info, authority_info, rig_info, miner_info, share_info, micro_info, well_info, oil_program) = 
        (s, a, r, m, sh, mic, w, op);
    
    signer_info.is_signer()?;
    let authority = *authority_info.key;
    
    // Validate accounts
    rig_info.is_writable()?.has_seeds(&[RIG, &authority.to_bytes()], &oil_api::ID)?;
    let rig = rig_info.as_account_mut::<Rig>(&oil_api::ID)?;
    rig.assert_mut(|r| r.authority == authority)?;
    
    miner_info.is_writable()?.has_seeds(&[MINER, &authority.to_bytes()], &oil_api::ID)?;
    let miner = miner_info.as_account_mut::<Miner>(&oil_api::ID)?;
    miner.assert_mut(|m| m.authority == authority)?;
    
    share_info.is_writable()?.has_seeds(&[SHARE, &authority.to_bytes(), &well_id.to_le_bytes(), &epoch_id.to_le_bytes()], &oil_api::ID)?;
    let share = share_info.as_account_mut::<Share>(&oil_api::ID)?;
    share.assert_mut(|s| s.authority == authority && s.well_id == well_id as u64 && s.epoch_id == epoch_id)?;
    
    micro_info.has_seeds(&[MICRO, &well_id.to_le_bytes(), &epoch_id.to_le_bytes()], &oil_api::ID)?;
    let micro = micro_info.as_account::<Micro>(&oil_api::ID)?;
    
    well_info.has_seeds(&[WELL, &well_id.to_le_bytes()], &oil_api::ID)?;
    let well = well_info.as_account::<Well>(&oil_api::ID)?;
    
    oil_program.is_program(&oil_api::ID)?;
    
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
    if rig.current_epoch_id[well_id] != 0 && share.epoch_id != rig.current_epoch_id[well_id] {
        return Err(ProgramError::InvalidArgument); // InvalidEpoch
    }
    
    // Can't checkpoint current/active epoch
    if share.epoch_id >= well.epoch_id {
        return Err(ProgramError::InvalidArgument); // CannotCheckpointCurrentEpoch
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
    
    // Update rig checkpointed epoch
    rig.checkpointed_epoch_id[well_id] = epoch_id;
    
    sol_log(&format!(
        "CheckpointAuction: well_id={}, epoch_id={}, oil={}, refund={} SOL",
        well_id,
        epoch_id,
        user_oil,
        lamports_to_sol(user_refund)
    ));
    
    Ok(())
}
