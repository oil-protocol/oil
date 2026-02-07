use oil_api::prelude::*;
use oil_api::fogo;
use solana_program::{log::sol_log, native_token::lamports_to_sol};
use steel::*;

/// Claims SOL rewards with single-tier referral system (FOGO session)
pub fn process_claim_sol_with_session<'a>(accounts: &'a [AccountInfo<'a>], _data: &[u8]) -> ProgramResult {
    let clock = Clock::get()?;
    
    if accounts.len() < 5 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    let program_signer_info = &accounts[0];
    let payer_info = &accounts[1];
    let authority_info = &accounts[2];
    let miner_info = &accounts[3];
    let system_program = &accounts[4];
    
    program_signer_info.is_signer()?;
    payer_info.is_signer()?;
    authority_info.is_writable()?;
    
    fogo::validate_program_signer(program_signer_info)?;
    
    let authority = *authority_info.key;
    
    let miner = miner_info
        .as_account_mut::<Miner>(&oil_api::ID)?
        .assert_mut(|m| m.authority == authority)?;
    system_program.is_program(&system_program::ID)?;

    let total_amount = miner.claim_sol(&clock);

    let referral_amount = if miner.referrer != Pubkey::default() {
        if accounts.len() < 7 {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        let miner_referrer_idx = 5;
        let miner_referrer_info = &accounts[miner_referrer_idx];
        miner_referrer_info
            .has_seeds(&[MINER, &miner.referrer.to_bytes()], &oil_api::ID)?;

        let referral_referrer_idx = 6;
        let referral_referrer_info = &accounts[referral_referrer_idx];
        referral_referrer_info
            .has_seeds(&[REFERRAL, &miner.referrer.to_bytes()], &oil_api::ID)?;
        
        let referral_referrer = referral_referrer_info
            .as_account_mut::<Referral>(&oil_api::ID)?;
        
        referral_referrer.credit_sol_referral(total_amount)
    } else {
        0
    };

    let authority_amount = total_amount.saturating_sub(referral_amount);

    sol_log(&format!("Claiming {} SOL", lamports_to_sol(total_amount)).as_str());

    if authority_amount > 0 {
        miner_info.send(authority_amount, authority_info);
    }
    
    if referral_amount > 0 {
        let referral_referrer_info = &accounts[6];
        
        miner_info.send(referral_amount, referral_referrer_info);
        
        sol_log(&format!(
            "Referral bonus: {} SOL to {}",
            lamports_to_sol(referral_amount),
            miner.referrer
        ));
    }
    
    Ok(())
}
