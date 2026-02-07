use oil_api::prelude::*;
use oil_api::fogo;
use solana_program::log::sol_log;
use spl_token::amount_to_ui_amount;
use steel::*;

/// Claim auction-based OIL rewards (FOGO session)
pub fn process_claim_auction_oil_with_session<'a>(accounts: &'a [AccountInfo<'a>], data: &[u8]) -> ProgramResult {
    let clock = Clock::get()?;
    let args = ClaimAuctionOIL::try_from_bytes(data)?;
    let well_mask = args.well_mask;

    if accounts.len() < 19 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }

    let [program_signer_info, payer_info, authority_info, miner_info, well_0_info, well_1_info, well_2_info, well_3_info, auction_info, treasury_info, treasury_tokens_info, mint_info, mint_authority_info, mint_program, recipient_info, token_program, associated_token_program, system_program, oil_program] =
        &accounts[0..19]
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    program_signer_info.is_signer()?;
    payer_info.is_signer()?;
    
    fogo::validate_program_signer(program_signer_info)?;
    
    let authority = *authority_info.key;
    
    let miner = miner_info
        .as_account_mut::<Miner>(&oil_api::ID)?
        .assert_mut(|d| d.authority == authority)?;
    
    if miner.last_claim_auction_oil_at > 0 {
        let time_since_last_claim = clock.unix_timestamp.saturating_sub(miner.last_claim_auction_oil_at);
        if time_since_last_claim < oil_api::consts::CLAIM_AUCTION_OIL_COOLDOWN_SECONDS {
            sol_log("Claim cooldown: Please wait before claiming again");
            return Err(ProgramError::Custom(6000));
        }
    }
    
    let auction = auction_info.as_account_mut::<Auction>(&oil_api::ID)?;
    let treasury = treasury_info.as_account_mut::<Treasury>(&oil_api::ID)?;
    mint_info.has_address(&MINT_ADDRESS)?.as_mint()?;
    treasury_tokens_info.as_associated_token_account(&treasury_info.key, &mint_info.key)?;
    mint_authority_info.as_account::<oil_mint_api::state::Authority>(&oil_mint_api::ID)?;
    mint_program.is_program(&oil_mint_api::ID)?;
    recipient_info.is_writable()?;
    token_program.is_program(&spl_token::ID)?;
    associated_token_program.is_program(&spl_associated_token_account::ID)?;
    system_program.is_program(&system_program::ID)?;
    oil_program.is_program(&oil_api::ID)?;

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

    miner.update_auction_rewards(treasury);
    
    let mut total_auction_oil = miner.auction_rewards_oil + miner.auction_refined_oil;

    let well_accounts = [well_0_info, well_1_info, well_2_info, well_3_info];
    
    for well_id in 0usize..4 {
        if (well_mask & (1 << well_id)) == 0 {
            continue;
        }

        let well_info = well_accounts[well_id];
        
        let well_id_u64: u64 = well_id.try_into().unwrap_or(0);
        let well = well_info
            .is_writable()?
            .has_seeds(&[WELL, &well_id_u64.to_le_bytes()], &oil_api::ID)?
            .as_account_mut::<Well>(&oil_api::ID)?;

        well.update_accumulated_oil(&clock);
        well.check_and_apply_halving(auction, &clock);

        let is_solo_owner = well.current_bidder == authority;

        if is_solo_owner {
            let accumulated_oil = well.accumulated_oil;
            total_auction_oil += accumulated_oil;
            well.accumulated_oil = 0;
            well.last_update_time = clock.unix_timestamp as u64;
        }
    }

    let mut signer_amount = 0u64;
    let mut refining_fee = 0u64;
    if total_auction_oil > 0 {
        let mut claimable_oil;
        let pre_minted_oil = miner.auction_rewards_oil;
        let current_ownership_oil = total_auction_oil.saturating_sub(pre_minted_oil);
        
        if current_ownership_oil > 0 {
            invoke_signed(
                &oil_mint_api::sdk::mint_oil(current_ownership_oil),
                &[
                    treasury_info.clone(),
                    mint_authority_info.clone(),
                    mint_info.clone(),
                    treasury_tokens_info.clone(),
                    token_program.clone(),
                ],
                &oil_api::ID,
                &[TREASURY],
            )?;
            
            treasury.auction_total_unclaimed += current_ownership_oil;
        }

        claimable_oil = total_auction_oil;
        if treasury.auction_total_unclaimed > 0 && total_auction_oil > 0 {
            refining_fee = total_auction_oil / 10;
            claimable_oil -= refining_fee;
            treasury.auction_rewards_factor += Numeric::from_fraction(refining_fee, treasury.auction_total_unclaimed);
            treasury.auction_total_refined += refining_fee;
        }

        treasury.auction_total_unclaimed = treasury.auction_total_unclaimed.saturating_sub(total_auction_oil);
        treasury.auction_total_refined = treasury.auction_total_refined.saturating_sub(miner.auction_refined_oil);

        miner.lifetime_rewards_oil += total_auction_oil;

        miner.auction_rewards_oil = 0;
        miner.auction_refined_oil = 0;
        miner.last_claim_auction_oil_at = clock.unix_timestamp;
        
        miner.auction_rewards_factor = treasury.auction_rewards_factor;

        let referral_amount = if miner.referrer != Pubkey::default() {
            if accounts.len() < 22 {
                return Err(ProgramError::NotEnoughAccountKeys);
            }
            
            let miner_referrer_idx = 19;
            let miner_referrer_info = &accounts[miner_referrer_idx];
            miner_referrer_info
                .has_seeds(&[MINER, &miner.referrer.to_bytes()], &oil_api::ID)?;
            
            let referral_referrer_idx = 20;
            let referral_referrer_info = &accounts[referral_referrer_idx];
            referral_referrer_info
                .has_seeds(&[REFERRAL, &miner.referrer.to_bytes()], &oil_api::ID)?;
            
            let referral_referrer = referral_referrer_info
                .as_account_mut::<Referral>(&oil_api::ID)?;
            
            referral_referrer.credit_oil_referral(claimable_oil)
        } else {
            0
        };

        signer_amount = claimable_oil.saturating_sub(referral_amount);

        if signer_amount > 0 {
            transfer_signed(
                treasury_info,
                treasury_tokens_info,
                recipient_info,
                token_program,
                signer_amount,
                &[TREASURY],
            )?;
        }
        
        if referral_amount > 0 {
            let referral_referrer_info = &accounts[20];
            let referral_referrer_oil_ata_info = &accounts[21];
                        
            if referral_referrer_oil_ata_info.data_is_empty() {
                create_associated_token_account(
                    payer_info,
                    referral_referrer_info,
                    referral_referrer_oil_ata_info,
                    mint_info,
                    system_program,
                    token_program,
                    associated_token_program,
                )?;
            } else {
                referral_referrer_oil_ata_info.as_associated_token_account(referral_referrer_info.key, mint_info.key)?;
            }
                        
            transfer_signed(
                treasury_info,
                treasury_tokens_info,
                referral_referrer_oil_ata_info,
                token_program,
                referral_amount,
                &[TREASURY],
            )?;
        
            sol_log(&format!(
                "Referral bonus: {} OIL to {}",
                amount_to_ui_amount(referral_amount, TOKEN_DECIMALS),
                miner.referrer
            ));
        }
    }

    auction_program_log(
        &[auction_info.clone(), oil_program.clone()],
        ClaimAuctionOILEvent {
            disc: 6,
            authority: authority,
            oil_claimed: signer_amount,
            refining_fee,
            ts: clock.unix_timestamp as u64,
        }
        .to_bytes(),
    )?;

    sol_log(
        &format!(
            "Claiming {} OIL",
            amount_to_ui_amount(signer_amount, TOKEN_DECIMALS),
        )
    );

    Ok(())
}
