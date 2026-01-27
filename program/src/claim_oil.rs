use oil_api::prelude::*;
use solana_program::log::sol_log;
use spl_token::amount_to_ui_amount;
use steel::*;

/// Claims OIL rewards with single-tier referral system.
pub fn process_claim_oil(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let clock = Clock::get()?;
    
    if accounts.len() < 10 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    let signer_info = &accounts[0];
    let authority_info = &accounts[1];
    let miner_info = &accounts[2];
    let mint_info = &accounts[3];
    let recipient_info = &accounts[4];
    let treasury_info = &accounts[5];
    let treasury_tokens_info = &accounts[6];
    let system_program = &accounts[7];
    let token_program = &accounts[8];
    let associated_token_program = &accounts[9];
    
    signer_info.is_signer()?;
    
    // Use the authority account (user's wallet public key) for PDA derivation
    let authority = *authority_info.key;
    
    let miner = miner_info
        .as_account_mut::<Miner>(&oil_api::ID)?
        .assert_mut(|m| m.authority == authority)?;
    mint_info.has_address(&MINT_ADDRESS)?.as_mint()?;
    recipient_info.is_writable()?;
    let treasury = treasury_info.as_account_mut::<Treasury>(&oil_api::ID)?;
    treasury_tokens_info.as_associated_token_account(&treasury_info.key, &mint_info.key)?;
    system_program.is_program(&system_program::ID)?;
    token_program.is_program(&spl_token::ID)?;
    associated_token_program.is_program(&spl_associated_token_account::ID)?;

    // Load recipient.
    if recipient_info.data_is_empty() {
        create_associated_token_account(
            signer_info, // payer (session payer pays for creation)
            authority_info, // owner (user's wallet owns the ATA)
            recipient_info,
            mint_info,
            system_program,
            token_program,
            associated_token_program,
        )?;
    } else {
        recipient_info.as_associated_token_account(authority_info.key, mint_info.key)?;
    }

    // Calculate total claimable amount.
    let total_amount = miner.claim_oil(&clock, treasury);

    // ENFORCE referral rewards: If miner has a referrer, require referral accounts to be provided.
    let referral_amount = if miner.referrer != Pubkey::default() {
        // Require at least 13 accounts (base 10 + miner_referrer + referral_referrer + referral_referrer_oil_ata)
        if accounts.len() < 13 {
            return Err(ProgramError::NotEnoughAccountKeys);
        }
        
        // Validate referrer's miner account
        let miner_referrer_idx = 10;
        let miner_referrer_info = &accounts[miner_referrer_idx];
        miner_referrer_info
            .has_seeds(&[MINER, &miner.referrer.to_bytes()], &oil_api::ID)?;
        
        // Validate referrer's referral account
        let referral_referrer_idx = 11;
        let referral_referrer_info = &accounts[referral_referrer_idx];
        referral_referrer_info
            .has_seeds(&[REFERRAL, &miner.referrer.to_bytes()], &oil_api::ID)?;
        
        // Get referral account and calculate/credit referral bonus
        let referral_referrer = referral_referrer_info
            .as_account_mut::<Referral>(&oil_api::ID)?;
        
        // Calculate and credit referral bonus (0.5% of total claim)
        referral_referrer.credit_oil_referral(total_amount)
    } else {
        0
    };

    // Calculate amount to send to signer (after referral deduction).
    let signer_amount = total_amount.saturating_sub(referral_amount);

    sol_log(
        &format!(
            "Claiming {} OIL",
            amount_to_ui_amount(total_amount, TOKEN_DECIMALS)
        )
        .as_str(),
    );

    // Transfer only the user's portion to recipient.
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
    
    // Transfer referral OIL directly to referral account's OIL ATA.
    if referral_amount > 0 {
        let referral_referrer_info = &accounts[11];
        let referral_referrer_oil_ata_info = &accounts[12];
                    
        // Create referral OIL ATA if it doesn't exist
        if referral_referrer_oil_ata_info.data_is_empty() {
            create_associated_token_account(
                signer_info, // payer (session payer pays for creation)
                referral_referrer_info, // owner (referral account PDA)
                referral_referrer_oil_ata_info,
                mint_info,
                system_program,
                token_program,
                associated_token_program,
            )?;
            } else {
                referral_referrer_oil_ata_info.as_associated_token_account(referral_referrer_info.key, mint_info.key)?;
            }
                    
            // Transfer OIL from treasury to referral account's OIL ATA
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
            )
            .as_str(),
        );
    }

    Ok(())
}
