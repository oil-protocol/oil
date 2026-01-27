use oil_api::prelude::*;
use oil_api::state::Whitelist;
use solana_program::log::sol_log;
use steel::*;

/// Creates a Whitelist account for a shared access code.
pub fn process_create_whitelist(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    let args = CreateWhitelist::try_from_bytes(data)?;
    let code_hash = args.code_hash;

    // Load accounts: [signer, config, whitelist, system_program]
    let [signer_info, config_info, whitelist_info, system_program_info] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    signer_info.is_signer()?;
    whitelist_info.is_writable()?;
    system_program_info.is_program(&system_program::ID)?;

    // Verify signer is admin
    config_info
        .as_account::<Config>(&oil_api::ID)?
        .assert(|c| c.admin == *signer_info.key)?;

    // Derive expected PDA (no authority needed - codes are shared)
    let (whitelist_pda, _) = Whitelist::pda(code_hash);
    whitelist_info.has_address(&whitelist_pda)?;

    // Check if whitelist entry already exists
    if !whitelist_info.data_is_empty() {
        sol_log("Whitelist entry already exists");
        return Err(ProgramError::InvalidArgument);
    }

    // Create the Whitelist account
    create_program_account::<Whitelist>(
        whitelist_info,
        system_program_info,
        signer_info,
        &oil_api::ID,
        &[b"whitelist", &code_hash],
    )?;

    // Initialize the Whitelist account
    let whitelist = whitelist_info.as_account_mut::<Whitelist>(&oil_api::ID)?;
    whitelist.code_hash = code_hash;
    whitelist.usage_count = 0;

    Ok(())
}
