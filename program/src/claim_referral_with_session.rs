use oil_api::prelude::*;
use oil_api::fogo;
use solana_program::{log::sol_log, native_token::lamports_to_sol};
use spl_token::amount_to_ui_amount;
use steel::*;

/// Claims pending referral rewards (both SOL and OIL) (FOGO session).
pub fn process_claim_referral_with_session<'a>(accounts: &'a [AccountInfo<'a>], _data: &[u8]) -> ProgramResult {
    let [signer_info, authority_info, program_signer_info, payer_info, referral_info, referral_tokens_info, mint_info, recipient_info, system_program, token_program, associated_token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    
    fogo::validate_session(signer_info)?;
    fogo::validate_program_signer(program_signer_info)?;
    
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

    if recipient_info.data_is_empty() {
        create_associated_token_account(
            payer_info,
            authority_info,
            recipient_info,
            mint_info,
            system_program,
            token_program,
            associated_token_program,
        )?;
    } else {
        recipient_info.as_associated_token_account(authority_info.key, mint_info.key)?;
    }

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
