use oil_api::prelude::*;
use oil_api::fogo;
use solana_program::{log::sol_log, native_token::lamports_to_sol};
use steel::*;

/// Claim auction-based SOL rewards (FOGO session)
pub fn process_claim_auction_sol_with_session<'a>(accounts: &'a [AccountInfo<'a>], _data: &[u8]) -> ProgramResult {
    let clock = Clock::get()?;

    if accounts.len() < 8 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let [signer_info, authority_info, program_signer_info, miner_info, treasury_info, auction_info, system_program, oil_program] =
        &accounts[0..8]
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

    signer_info.is_signer()?;
    authority_info.is_writable()?;
    
    fogo::validate_session(signer_info)?;
    fogo::validate_program_signer(program_signer_info)?;
    
    let authority = *authority_info.key;
    
    let miner = miner_info
        .as_account_mut::<Miner>(&oil_api::ID)?
        .assert_mut(|d| d.authority == authority)?;
    let treasury = treasury_info.as_account_mut::<Treasury>(&oil_api::ID)?;
    system_program.is_program(&system_program::ID)?;
    oil_program.is_program(&oil_api::ID)?;

    let total_sol_claimed = miner.auction_rewards_sol;
    let rewards_sol = miner.auction_rewards_sol;
    miner.auction_rewards_sol = 0;

    let referral_amount = if miner.referrer != Pubkey::default() {
        if accounts.len() < 10 {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        let miner_referrer_idx = 8;
        let miner_referrer_info = &accounts[miner_referrer_idx];
        miner_referrer_info
            .has_seeds(&[MINER, &miner.referrer.to_bytes()], &oil_api::ID)?;

        let referral_referrer_idx = 9;
        let referral_referrer_info = &accounts[referral_referrer_idx];
        referral_referrer_info
            .has_seeds(&[REFERRAL, &miner.referrer.to_bytes()], &oil_api::ID)?;
        
        let referral_referrer = referral_referrer_info
            .as_account_mut::<Referral>(&oil_api::ID)?;
        
        referral_referrer.credit_sol_referral(total_sol_claimed)
    } else {
        0
    };

    let authority_amount = total_sol_claimed.saturating_sub(referral_amount);

    if authority_amount > 0 {
        treasury_info.send(authority_amount, authority_info);
        treasury.auction_rewards_sol = treasury.auction_rewards_sol.saturating_sub(authority_amount);
    }
    
    if referral_amount > 0 {
        let referral_referrer_info = &accounts[9];
        
        treasury_info.send(referral_amount, referral_referrer_info);
        treasury.auction_rewards_sol = treasury.auction_rewards_sol.saturating_sub(referral_amount);
        
        sol_log(&format!(
            "Referral bonus: {} SOL to {}",
            lamports_to_sol(referral_amount),
            miner.referrer
        ));
    }

    if total_sol_claimed > 0 {
        miner.lifetime_rewards_sol += total_sol_claimed;
        miner.last_claim_auction_sol_at = clock.unix_timestamp;
    }

    auction_info
        .is_writable()?
        .has_seeds(&[AUCTION], &oil_api::ID)?;
    auction_program_log(
        &[auction_info.clone(), oil_program.clone()],
        ClaimAuctionSOLEvent {
            disc: 7,
            authority: authority,
            sol_claimed: total_sol_claimed,
            rewards_sol,
            refunds_sol: 0,
            ts: clock.unix_timestamp as u64,
        }
        .to_bytes(),
    )?;

    sol_log(
        &format!(
            "Claiming {} SOL",
            lamports_to_sol(authority_amount),
        )
    );
    Ok(())
}
