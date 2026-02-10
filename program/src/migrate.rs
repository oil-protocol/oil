use oil_api::prelude::*;
use solana_program::{
    log::sol_log,
    program::invoke,
    system_instruction,
    rent::Rent,
    sysvar::Sysvar,
};
use steel::*;

/// Migrate Miner: Extend with 2 [u64; 4] arrays
/// This moves current_epoch_id and checkpointed_epoch_id from Rig to Miner
pub fn process_migrate(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    let [signer_info, config_info, miner_info, system_program_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    
    signer_info.is_signer()?;
    config_info.is_writable()?;
    miner_info.is_writable()?;
    system_program_info.is_program(&system_program::ID)?;
    
    // Validate config is owned by our program
    config_info.has_owner(&oil_api::ID)?;
    
    // Validate miner is owned by our program
    miner_info.has_owner(&oil_api::ID)?;
    
    // Read admin from config to validate signer
    let config = config_info.as_account::<Config>(&oil_api::ID)?;
    if config.admin != *signer_info.key {
        return Err(ProgramError::InvalidArgument);
    }
    
    // Calculate sizes
    // Current Miner: 664 bytes data + 8 discriminator = 672 bytes total
    // New Miner: 664 + 64 (2 [u64; 4]) = 728 bytes data + 8 discriminator = 736 bytes total
    let old_size = miner_info.data_len();
    let new_struct_size = 736; // New Miner: 8 discriminator + 728 bytes of data
    
    // Check if already migrated
    if old_size >= new_struct_size {
        sol_log("Miner already migrated");
        return Ok(());
    }
    
    // Verify current size matches expected old size
    if old_size != 672 {
        sol_log(&format!("Warning: Miner size is {} bytes, expected 672 bytes", old_size));
        // Continue anyway - might be a different version
    }
    
    sol_log("Migrating Miner: Adding 2 [u64; 4] arrays (current_epoch_id, checkpointed_epoch_id)");
    sol_log(&format!("Reallocating Miner: {} -> {} bytes", old_size, new_struct_size));
    
    // Read miner to check existing balance requirements
    let miner = miner_info.as_account::<Miner>(&oil_api::ID)?;
    let rent = Rent::get()?;
    let old_rent_requirement = rent.minimum_balance(old_size);
    let new_rent_requirement = rent.minimum_balance(new_struct_size);
    let current_balance = miner_info.lamports();
    
    // Calculate what the account needs:
    // - New rent requirement (for larger account size)
    // - Existing checkpoint_fee (preserved)
    // - Existing block_rewards_sol (preserved)
    let required_balance = new_rent_requirement
        .saturating_add(miner.checkpoint_fee)
        .saturating_add(miner.block_rewards_sol);
    
    // Calculate additional rent needed
    // We need to ensure the account has enough for rent + checkpoint_fee + block_rewards_sol
    let additional_rent = if required_balance > current_balance {
        required_balance.saturating_sub(current_balance)
    } else {
        0
    };
    
    sol_log(&format!("Current balance: {} lamports", current_balance));
    sol_log(&format!("Old rent requirement: {} lamports", old_rent_requirement));
    sol_log(&format!("New rent requirement: {} lamports", new_rent_requirement));
    sol_log(&format!("Required balance (rent + checkpoint_fee + block_rewards_sol): {} lamports", required_balance));
    sol_log(&format!("Additional rent needed: {} lamports", additional_rent));
    
    // Transfer additional rent if needed
    if additional_rent > 0 {
        invoke(
            &system_instruction::transfer(signer_info.key, miner_info.key, additional_rent),
            &[signer_info.clone(), miner_info.clone(), system_program_info.clone()],
        )?;
    }
    
    // Reallocate the account (new bytes are automatically zero-initialized)
    miner_info.realloc(new_struct_size, false)?;
    
    sol_log("Miner migration complete: 2 [u64; 4] arrays added (initialized to 0)");
    
    Ok(())
}
