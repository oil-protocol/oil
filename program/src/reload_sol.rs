use oil_api::prelude::*;
use solana_program::{log::sol_log, native_token::lamports_to_sol};
use steel::*;

/// Reloads SOL from miner account to automation balance with single-tier referral system.
pub fn process_reload_sol(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let clock = Clock::get()?;
    
    if accounts.len() < 4 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    let signer_info = &accounts[0];
    let automation_info = &accounts[1];
    let miner_info = &accounts[2];
    let system_program = &accounts[3];
    
    signer_info.is_signer()?;
    let automation = automation_info
        .as_account_mut::<Automation>(&oil_api::ID)?
        .assert_mut(|a| a.executor == *signer_info.key)?
        .assert_mut(|a| a.reload > 0)?;
    let miner = miner_info
        .as_account_mut::<Miner>(&oil_api::ID)?
        .assert_mut(|m| m.authority == automation.authority)?;
    system_program.is_program(&system_program::ID)?;

    // Get claimable amount (includes both regular SOL and gusher SOL).
    let total_amount = miner.claim_sol(&clock);

    let referral_amount = if miner.referrer != Pubkey::default() {
        if accounts.len() < 6 {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        // Validate referrer's miner account
        let miner_referrer_idx = 4;
        let miner_referrer_info = &accounts[miner_referrer_idx];
        miner_referrer_info
            .has_seeds(&[MINER, &miner.referrer.to_bytes()], &oil_api::ID)?;

        // Validate referrer's referral account
        let referral_referrer_idx = 5;
        let referral_referrer_info = &accounts[referral_referrer_idx];
        referral_referrer_info
            .has_seeds(&[REFERRAL, &miner.referrer.to_bytes()], &oil_api::ID)?;
        
        // Get referral account and calculate/credit referral bonus
        let referral_referrer_info = &accounts[referral_referrer_idx];
        let referral_referrer = referral_referrer_info
            .as_account_mut::<Referral>(&oil_api::ID)?;
        
        // Calculate and credit referral bonus (0.5% of total claim)
        referral_referrer.credit_sol_referral(total_amount)
    } else {
        0
    };

    // Calculate amount to add to automation balance (after referral deduction).
    let automation_amount = total_amount.saturating_sub(referral_amount);

    sol_log(&format!("Reloading {} SOL", lamports_to_sol(total_amount)).as_str());

    // Increment automation balance with claimed amount (after referral deduction).
    automation.balance += automation_amount;

    // Transfer automation's portion from miner account to automation.
    if automation_amount > 0 {
        miner_info.send(automation_amount, automation_info);
    }
    
    // Transfer referral SOL directly to referral account PDA from miner account.
    if referral_amount > 0 {
        let referral_referrer_info = &accounts[5];
        
        // Transfer SOL from miner to referral account
        miner_info.send(referral_amount, referral_referrer_info);
        
        sol_log(&format!(
            "Referral bonus: {} SOL to {}",
            lamports_to_sol(referral_amount),
            miner.referrer
        ));
    }

    Ok(())
}
