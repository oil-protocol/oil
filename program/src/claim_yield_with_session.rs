use oil_api::prelude::*;
use oil_api::fogo;
use solana_program::log::sol_log;
use solana_program::native_token::lamports_to_sol;
use steel::*;

/// Claims SOL yield from the staking contract (FOGO session). Stakers earn SOL rewards (2% of round winnings), not OIL.
pub fn process_claim_yield_with_session<'a>(accounts: &'a [AccountInfo<'a>], data: &[u8]) -> ProgramResult {
    let args = ClaimYield::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);

    let clock = Clock::get()?;
    let [signer_info, authority_info, program_signer_info, stake_info, pool_info, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    
    fogo::validate_session(signer_info)?;
    fogo::validate_program_signer(program_signer_info)?;
    
    let authority = *authority_info.key;
    authority_info.is_writable()?;
    
    let stake = stake_info
        .as_account_mut::<Stake>(&oil_api::ID)?
        .assert_mut(|s| s.authority == authority)?;
    let pool = pool_info.as_account_mut::<Pool>(&oil_api::ID)?;
    system_program.is_program(&system_program::ID)?;

    stake.update_rewards(pool);
    let available_rewards = stake.rewards;
    let requested_amount = amount.min(available_rewards);
    let claimable_amount = requested_amount.min(pool.balance);
    
    stake.rewards -= claimable_amount;
    stake.last_claim_at = clock.unix_timestamp;
    
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
