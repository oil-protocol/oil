use oil_api::prelude::*;
use solana_program::log::sol_log;
use steel::*;

/// Creates a referral account for a user to become a referrer.
pub fn process_create_referral(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let [signer_info, authority_info, referral_info, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    
    // Use the authority account (user's wallet public key) for PDA derivation
    let user = *authority_info.key;
    
    referral_info
        .is_writable()?
        .has_seeds(&[REFERRAL, &user.to_bytes()], &oil_api::ID)?;
    system_program.is_program(&system_program::ID)?;

    // Create referral account if it doesn't exist.
    if referral_info.data_is_empty() {
        create_program_account::<Referral>(
            referral_info,
            system_program,
            signer_info,
            &oil_api::ID,
            &[REFERRAL, &user.to_bytes()],
        )?;
        let referral = referral_info.as_account_mut::<Referral>(&oil_api::ID)?;
        referral.authority = user; // Store the user's wallet public key as authority
        referral.total_referred = 0;
        referral.total_sol_earned = 0;
        referral.total_oil_earned = 0;
        referral.pending_sol = 0;
        referral.pending_oil = 0;

        sol_log("Created referral account");
    } else {
        sol_log("Referral account already exists");
    }

    Ok(())
}

