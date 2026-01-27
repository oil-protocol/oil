use oil_api::prelude::*;
use solana_program::{log::sol_log, native_token::lamports_to_sol};
use spl_token::amount_to_ui_amount;
use steel::*;

/// Claims pending referral rewards (both SOL and OIL).
pub fn process_claim_referral(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let [signer_info, authority_info, referral_info, referral_tokens_info, mint_info, recipient_info, system_program, token_program, associated_token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    
    // Use the authority account (user's wallet public key) for PDA derivation
    // This allows Fogo sessions to work - the payer signs, but the authority is the user's wallet
    let authority = *authority_info.key;
    
    let referral = referral_info
        .as_account_mut::<Referral>(&oil_api::ID)?
        .assert_mut(|r| r.authority == authority)?;
    referral_tokens_info.as_associated_token_account(&referral_info.key, &mint_info.key)?;
    mint_info.has_address(&MINT_ADDRESS)?.as_mint()?;
    recipient_info.is_writable()?;
    system_program.is_program(&system_program::ID)?;
    token_program.is_program(&spl_token::ID)?;
    associated_token_program.is_program(&spl_associated_token_account::ID)?;

    // Create recipient OIL ATA if it doesn't exist (signer pays for creation)
    if recipient_info.data_is_empty() {
            create_associated_token_account(
                signer_info,
                signer_info,
            recipient_info,
                mint_info,
                system_program,
                token_program,
                associated_token_program,
            )?;
        } else {
        // Validate existing recipient OIL ATA (owned by authority, not signer)
        recipient_info.as_associated_token_account(&authority, mint_info.key)?;
        }

    // Normalize amount
    let pending_oil = referral.claim_oil();
    let pending_sol = referral.claim_sol();

    sol_log(
        &format!(
            "Claiming {} OIL",
            amount_to_ui_amount(pending_oil, TOKEN_DECIMALS)
        )
        .as_str(),
    );

    sol_log(&format!("Claiming {} SOL", lamports_to_sol(pending_sol)).as_str());

    // Do CPI first (token transfer), then direct lamport modification (SOL transfer).
        transfer_signed(
            referral_info,
            referral_tokens_info,
            recipient_info,
            token_program,
            pending_oil,
            &[REFERRAL, &authority.to_bytes()],
        )?;

    referral_info.send(pending_sol, authority_info);

    Ok(())
}

