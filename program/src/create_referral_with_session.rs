use oil_api::prelude::*;
use oil_api::fogo;
use solana_program::log::sol_log;
use steel::*;

/// Creates a referral account for a user to become a referrer (FOGO session).
pub fn process_create_referral_with_session<'a>(accounts: &'a [AccountInfo<'a>], _data: &[u8]) -> ProgramResult {
    let [signer_info, authority_info, program_signer_info, payer_info, referral_info, system_program] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    
    fogo::validate_session(signer_info)?;
    fogo::validate_program_signer(program_signer_info)?;
    
    let authority = *authority_info.key;
    
    referral_info
        .is_writable()?
        .has_seeds(&[REFERRAL, &authority.to_bytes()], &oil_api::ID)?;
    system_program.is_program(&system_program::ID)?;

    if referral_info.data_is_empty() {
        create_program_account::<Referral>(
            referral_info,
            system_program,
            payer_info,
            &oil_api::ID,
            &[REFERRAL, &authority.to_bytes()],
        )?;
        let referral = referral_info.as_account_mut::<Referral>(&oil_api::ID)?;
        referral.authority = authority;
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
