use oil_api::prelude::*;
use solana_program::log::sol_log;
use solana_program::native_token::lamports_to_sol;
use steel::*;

/// Claims SOL yield from the staking contract. Stakers earn SOL rewards (2% of round winnings), not OIL.
pub fn process_claim_yield(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse data.
    let args = ClaimYield::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);

    let clock = Clock::get()?;
    let [signer_info, authority_info, stake_info, pool_info, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    
    // Use the authority account (user's wallet public key) for PDA derivation
    // This allows Fogo sessions to work - the payer signs, but the authority is the user's wallet
    let authority = *authority_info.key;
    
    authority_info.is_writable()?; // Authority receives SOL
    let stake = stake_info
        .as_account_mut::<Stake>(&oil_api::ID)?
        .assert_mut(|s| s.authority == authority)?;
    let pool = pool_info.as_account_mut::<Pool>(&oil_api::ID)?;
    system_program.is_program(&system_program::ID)?;

    // Claim SOL yield from stake account (rewards field now stores SOL).
    stake.update_rewards(pool);
    let available_rewards = stake.rewards;
    let requested_amount = amount.min(available_rewards);
    let claimable_amount = requested_amount.min(pool.balance);
    
    // Deduct from stake.rewards and update last_claim_at
    stake.rewards -= claimable_amount;
    stake.last_claim_at = clock.unix_timestamp;
    
    // Transfer SOL from pool to authority (user's wallet)
    pool.balance -= claimable_amount;
    pool_info.send(claimable_amount, authority_info);
    sol_log(
        &format!(
            "Claiming {} SOL",
            lamports_to_sol(claimable_amount)
        )
        .as_str(),
    );

    Ok(())
}
