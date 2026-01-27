use oil_api::prelude::*;
use solana_program::log::sol_log;
use steel::*;

/// Sets the TGE (Token Generation Event) timestamp.
pub fn process_set_tge_timestamp(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse data.
    let args = SetTgeTimestamp::try_from_bytes(data)?;
    let new_tge_timestamp = i64::from_le_bytes(args.tge_timestamp);

    // Load accounts.
    let [signer_info, config_info, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    config_info.is_writable()?;
    system_program.is_program(&system_program::ID)?;

    // Verify signer is admin
    let config = config_info
        .as_account_mut::<Config>(&oil_api::ID)?
        .assert_mut_err(
            |c| c.admin == *signer_info.key,
            OilError::NotAuthorized.into(),
        )?;

    // Set TGE timestamp
    let old_tge_timestamp = config.tge_timestamp;
    config.tge_timestamp = new_tge_timestamp;

    if new_tge_timestamp == 0 {
        sol_log("TGE timestamp set to 0: pre-mine disabled");
    } else {
        sol_log(&format!(
            "TGE timestamp updated: {} -> {} (pre-mine {} until TGE)",
            old_tge_timestamp,
            new_tge_timestamp,
            if new_tge_timestamp > old_tge_timestamp { "active" } else { "disabled" }
        ));
    }

    Ok(())
}
