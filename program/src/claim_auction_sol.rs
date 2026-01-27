use oil_api::prelude::*;
use solana_program::{log::sol_log, native_token::lamports_to_sol};
use steel::*;

/// Claim auction-based SOL rewards
pub fn process_claim_auction_sol(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    let clock = Clock::get()?;
    let _args = ClaimAuctionSOL::try_from_bytes(data)?;

    // Minimum accounts required (without referral)
    if accounts.len() < 7 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    // Destructure base accounts (like checkpoint and deploy)
    let [signer_info, authority_info, miner_info, treasury_info, auction_info, system_program, oil_program] =
        &accounts[0..7]
        else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

    signer_info.is_signer()?;
    signer_info.is_writable()?; // Signer must be writable to receive SOL
    
    // Use the authority account (user's wallet public key) for PDA derivation
    // This allows Fogo sessions to work - the payer signs, but the authority is the user's wallet
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

    // ENFORCE referral rewards: If miner has a referrer, require referral accounts to be provided.
    let referral_amount = if miner.referrer != Pubkey::default() {
        // Require at least 9 accounts (base 7 + miner_referrer + referral_referrer) if miner has referrer
        if accounts.len() < 9 {
            return Err(ProgramError::NotEnoughAccountKeys);
        }

        // Validate referrer's miner account
        let miner_referrer_idx = 7;
        let miner_referrer_info = &accounts[miner_referrer_idx];
        miner_referrer_info
            .has_seeds(&[MINER, &miner.referrer.to_bytes()], &oil_api::ID)?;

        // Validate referrer's referral account
        let referral_referrer_idx = 8;
        let referral_referrer_info = &accounts[referral_referrer_idx];
        referral_referrer_info
            .has_seeds(&[REFERRAL, &miner.referrer.to_bytes()], &oil_api::ID)?;
        
        // Get referral account and calculate/credit referral bonus
        let referral_referrer = referral_referrer_info
            .as_account_mut::<Referral>(&oil_api::ID)?;
        
        // Calculate and credit referral bonus (0.5% of total claim)
        referral_referrer.credit_sol_referral(total_sol_claimed)
    } else {
        0
    };

    // Calculate amount to send to authority (after referral deduction)
    let authority_amount = total_sol_claimed.saturating_sub(referral_amount);

    // Transfer authority's portion from treasury to authority (user's wallet)
    if authority_amount > 0 {
        treasury_info.send(authority_amount, authority_info);
        // Subtract from treasury.auction_rewards_sol (tracked separately from treasury.balance)
        treasury.auction_rewards_sol = treasury.auction_rewards_sol.saturating_sub(authority_amount);
    }
    
    // Transfer referral SOL directly to referral account PDA from treasury
    if referral_amount > 0 {
        let referral_referrer_info = &accounts[8];
        
        // Transfer SOL from treasury to referral account
        treasury_info.send(referral_amount, referral_referrer_info);
        // Subtract from treasury.auction_rewards_sol
        treasury.auction_rewards_sol = treasury.auction_rewards_sol.saturating_sub(referral_amount);
        
        sol_log(&format!(
            "Referral bonus: {} SOL to {}",
            lamports_to_sol(referral_amount),
            miner.referrer
        ));
    }

    // Update miner timestamps and lifetime stats
    if total_sol_claimed > 0 {
        miner.lifetime_rewards_sol += total_sol_claimed;
        miner.last_claim_auction_sol_at = clock.unix_timestamp;
    }

    // Emit event
    auction_info
        .is_writable()?
        .has_seeds(&[AUCTION], &oil_api::ID)?;
    auction_program_log(
        &[auction_info.clone(), oil_program.clone()],
        ClaimAuctionSOLEvent {
            disc: 7,
            authority: authority, // Use authority (user's wallet) for event
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

