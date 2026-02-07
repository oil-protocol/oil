use oil_api::prelude::*;
use oil_api::fogo;
use solana_program::log::sol_log;
use spl_token::amount_to_ui_amount;
use steel::*;

/// Claims OIL rewards with single-tier referral system (FOGO session)
pub fn process_claim_oil_with_session<'a>(accounts: &'a [AccountInfo<'a>], _data: &[u8]) -> ProgramResult {
    let clock = Clock::get()?;
    
    if accounts.len() < 11 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    let program_signer_info = &accounts[0];
    let payer_info = &accounts[1];
    let authority_info = &accounts[2];
    let miner_info = &accounts[3];
    let mint_info = &accounts[4];
    let recipient_info = &accounts[5];
    let treasury_info = &accounts[6];
    let treasury_tokens_info = &accounts[7];
    let system_program = &accounts[8];
    let token_program = &accounts[9];
    let associated_token_program = &accounts[10];
    
    program_signer_info.is_signer()?;
    payer_info.is_signer()?;
    
    fogo::validate_program_signer(program_signer_info)?;
    
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

    let total_amount = miner.claim_oil(&clock, treasury);

    let referral_amount = if miner.referrer != Pubkey::default() {
        if accounts.len() < 14 {
            return Err(ProgramError::NotEnoughAccountKeys);
        }
        
        let miner_referrer_idx = 11;
        let miner_referrer_info = &accounts[miner_referrer_idx];
        miner_referrer_info
            .has_seeds(&[MINER, &miner.referrer.to_bytes()], &oil_api::ID)?;
        
        let referral_referrer_idx = 12;
        let referral_referrer_info = &accounts[referral_referrer_idx];
        referral_referrer_info
            .has_seeds(&[REFERRAL, &miner.referrer.to_bytes()], &oil_api::ID)?;
        
        let referral_referrer = referral_referrer_info
            .as_account_mut::<Referral>(&oil_api::ID)?;
        
        referral_referrer.credit_oil_referral(total_amount)
    } else {
        0
    };

    let signer_amount = total_amount.saturating_sub(referral_amount);

    sol_log(
        &format!(
            "Claiming {} OIL",
            amount_to_ui_amount(total_amount, TOKEN_DECIMALS)
        )
        .as_str(),
    );

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
        let referral_referrer_info = &accounts[12];
        let referral_referrer_oil_ata_info = &accounts[13];
                    
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
        )
        .as_str(),
    );
    }

    Ok(())
}
