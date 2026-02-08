use oil_api::prelude::*;
use solana_program::{
    log::sol_log,
    program::invoke,
    system_instruction,
    rent::Rent,
    sysvar::Sysvar,
};
use steel::*;

/// Migrate: Extend Treasury struct with 2 u64 fields (buffer_b and auction_total_pooled)
pub fn process_migrate(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    let [signer_info, config_info, treasury_info, system_program_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    
    signer_info.is_signer()?;
    config_info.is_writable()?;
    treasury_info.is_writable()?;
    system_program_info.is_program(&system_program::ID)?;
    
    // Validate config is owned by our program
    config_info.has_owner(&oil_api::ID)?;
    
    // Validate treasury is owned by our program and has correct seeds
    treasury_info.has_owner(&oil_api::ID)?;
    treasury_info.has_seeds(&[TREASURY], &oil_api::ID)?;
    
    // Read admin from config to validate signer
    let config = config_info.as_account::<Config>(&oil_api::ID)?;
    if config.admin != *signer_info.key {
        return Err(ProgramError::InvalidArgument);
    }
    
    // Calculate sizes - current Treasury is 128 bytes (8 discriminator + 120 data), new is 144 bytes (8 discriminator + 136 data)
    let old_size = treasury_info.data_len();
    let new_struct_size = 144; // New Treasury: 8 discriminator + 136 bytes of data (adding 2 u64 fields = 16 bytes)
    
    // Check if already migrated
    if old_size >= new_struct_size {
        sol_log("Treasury already migrated");
        return Ok(());
    }
    
    sol_log("Migrating Treasury: Adding 2 u64 fields (buffer_b and auction_total_pooled)");
    sol_log(&format!("Reallocating Treasury: {} -> {} bytes", old_size, new_struct_size));
    
    // Reallocate account to new size
    let rent = Rent::get()?;
    let new_rent = rent.minimum_balance(new_struct_size);
    let current_rent = treasury_info.lamports();
    let additional_rent = new_rent.saturating_sub(current_rent);
    
    // Transfer additional rent if needed
    if additional_rent > 0 {
        invoke(
            &system_instruction::transfer(signer_info.key, treasury_info.key, additional_rent),
            &[signer_info.clone(), treasury_info.clone(), system_program_info.clone()],
        )?;
    }
    
    // Reallocate the account (new bytes are automatically zero-initialized)
    treasury_info.realloc(new_struct_size, false)?;
    
    sol_log("Treasury migration complete: 2 u64 fields added (initialized to 0)");
    
    Ok(())
}
