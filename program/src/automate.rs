use oil_api::prelude::*;
use oil_api::consts::SOL_MINT;
use oil_api::utils::create_or_validate_wrapped_sol_ata;
use solana_program::pubkey::Pubkey;
use solana_program::log::sol_log;
use solana_program::program::invoke_signed as solana_invoke_signed;
use fogo_sessions_sdk::token::instruction::transfer_checked;
use fogo_sessions_sdk::token::PROGRAM_SIGNER_SEED;
use spl_token::instruction::close_account;
use steel::*;

/// Sets the executor.
pub fn process_automate(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse data.
    let args = Automate::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);
    let deposit = u64::from_le_bytes(args.deposit);
    let fee = u64::from_le_bytes(args.fee);
    let mask = u64::from_le_bytes(args.mask);
    let strategy = AutomationStrategy::from_u64(args.strategy as u64);
    let reload = u64::from_le_bytes(args.reload) > 0;
    let referrer = Pubkey::new_from_array(args.referrer);
    let pooled = args.pooled != 0;

    // Load accounts.
    if accounts.len() < 13 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    // Detect Fogo session early: if authority != signer, it's a Fogo session
    let is_fogo_session_early = accounts.len() >= 2 && accounts[0].key != accounts[1].key;
    
    let has_referral = referrer != Pubkey::default();
    
    // Log minimal info for debugging
    sol_log(&format!("Automate: {} accounts, Fogo: {}, referral: {}", accounts.len(), is_fogo_session_early, has_referral));
    
    // Parse accounts
    let (signer_info, authority_info, program_signer_info_opt, payer_info_opt, automation_info, executor_info, miner_info, system_program, oil_program,
         user_wrapped_sol_info, automation_wrapped_sol_info, token_program_info, mint_info, ata_program_info,
         referral_info_opt) =
        if is_fogo_session_early {
            let expected_len = 14 + if has_referral { 1 } else { 0 };
            if accounts.len() < expected_len {
                return Err(ProgramError::NotEnoughAccountKeys);
            }
            let mut accounts_iter = accounts.iter();
            oil_api::extract_accounts!(accounts_iter, [s, a, ps, pay, aut, e, m, sp, op, uws, aws, tp, mi, atap]);
            let ref_info = if has_referral { accounts_iter.next() } else { None };
            (s, a, Some(ps), Some(pay), aut, e, m, sp, op, uws, aws, tp, mi, atap, ref_info)
        } else {
            // Regular wallet (no program_signer, no separate payer)
            let expected_len = 12 + if has_referral { 1 } else { 0 };
            if accounts.len() < expected_len {
                return Err(ProgramError::NotEnoughAccountKeys);
            }
            let mut accounts_iter = accounts.iter();
            oil_api::extract_accounts!(accounts_iter, [s, a, aut, e, m, sp, op, uws, aws, tp, mi, atap]);
            let ref_info = if has_referral { accounts_iter.next() } else { None };
            (s, a, None, None, aut, e, m, sp, op, uws, aws, tp, mi, atap, ref_info)
        };
    
    signer_info.is_signer()?;
    automation_info.is_writable()?;
    system_program.is_program(&system_program::ID)?;
    oil_program.is_program(&oil_api::ID)?;
    
    // Use the authority account (user's wallet public key) for PDA derivation
    // This allows Fogo sessions to work - the payer signs, but the authority is the user's wallet
    let authority = *authority_info.key;
    
    // is_fogo_session already determined during account parsing (is_fogo_session_early)
    let is_fogo_session = is_fogo_session_early;
    
    // Derive expected PDAs for validation
    let (expected_automation_pda, _) = Pubkey::find_program_address(&[AUTOMATION, &authority.to_bytes()], &oil_api::ID);
    
    // Log key values for debugging
    sol_log(&format!("automation_info.key: {}", automation_info.key));
    sol_log(&format!("expected_automation_pda: {}", expected_automation_pda));
    sol_log(&format!("mint_info.key: {}", mint_info.key));
    
    // Validate token accounts
    token_program_info.is_program(&spl_token::ID)?;
    mint_info.has_address(&SOL_MINT)?;
    ata_program_info.is_program(&spl_associated_token_account::ID)?;

    // Open miner account.
    let is_new_miner = miner_info.data_is_empty();
    let miner = if is_new_miner {
        // Determine payer for account creation
        let account_creation_payer = payer_info_opt.unwrap_or(signer_info);
        
        create_program_account::<Miner>(
            miner_info,
            system_program,
            account_creation_payer,
            &oil_api::ID,
            &[MINER, &authority.to_bytes()],
        )?;
        let miner = miner_info.as_account_mut::<Miner>(&oil_api::ID)?;
        miner.initialize(authority);

        // Set referrer if provided and valid (not default pubkey and not self-referral).
        if referrer != Pubkey::default() && referrer != authority {
            miner.referrer = referrer;
            Referral::process_new_miner_referral(
                referral_info_opt,
                referrer,
                authority,
            )?;
        } else {
            miner.referrer = Pubkey::default();
        }
        miner
    } else {
        miner_info
            .as_account_mut::<Miner>(&oil_api::ID)?
            .assert_mut_err(
                |m| m.authority == authority,
                OilError::NotAuthorized.into(),
            )?
    };

    // Close account if executor is Pubkey::default().
    if *executor_info.key == Pubkey::default() {
        // Only close if automation account exists
        if !automation_info.data_is_empty() {
            automation_info
                .as_account_mut::<Automation>(&oil_api::ID)?
                .assert_mut_err(
                    |a| a.authority == authority,
                    OilError::NotAuthorized.into(),
                )?;

            automation_info.close(authority_info)?;
        }
        return Ok(());
    }

    // Create automation account FIRST (before any ATA operations to avoid privilege escalation)
    let automation = if automation_info.data_is_empty() {
        // Determine payer for account creation
        let account_creation_payer = payer_info_opt.unwrap_or(signer_info);
        
        create_program_account::<Automation>(
            automation_info,
            system_program,
            account_creation_payer,
            &oil_api::ID,
            &[AUTOMATION, &authority.to_bytes()],
        )?;
        let automation = automation_info.as_account_mut::<Automation>(&oil_api::ID)?;
        automation.balance = 0;
        automation.authority = authority;
        automation
    } else {
        automation_info
            .as_account_mut::<Automation>(&oil_api::ID)?
            .assert_mut_err(
                |a| a.authority == authority,
                OilError::NotAuthorized.into(),
            )?
    };

    // These are temporary accounts needed for the transfer, but they need to exist
    let payer = payer_info_opt.unwrap_or(signer_info);
    if is_fogo_session {
        // Initialize user's wFOGO ATA if it doesn't exist (needed for transfers)
        create_or_validate_wrapped_sol_ata(
            user_wrapped_sol_info,
            authority_info,
            mint_info,
            payer,
            system_program,
            token_program_info,
            ata_program_info,
            None,
        )?;
        
        // Initialize automation's wFOGO ATA if needed (temporary, just for Fogo session transfers)
        create_or_validate_wrapped_sol_ata(
            automation_wrapped_sol_info,
            automation_info,
            mint_info,
            payer,
            system_program,
            token_program_info,
            ata_program_info,
            Some("automation_wrapped_sol"),
        )?;
    } else {
        // For regular wallets, validate the ATA exists
        automation_wrapped_sol_info.as_associated_token_account(automation_info.key, mint_info.key)?;
    }

    // Set strategy and mask.
    automation.amount = amount;
    automation.balance += deposit;
    automation.executor = *executor_info.key;
    automation.fee = fee;
    automation.mask = mask;
    automation.strategy = strategy as u64;
    automation.reload = reload as u64;
    automation.pooled = pooled as u64;

    // Top up checkpoint fee.
    if miner.checkpoint_fee == 0 {
        miner.checkpoint_fee = CHECKPOINT_FEE;
        
        // For Fogo sessions, transfer wrapped SOL from user's ATA to miner account
        // For regular wallets, transfer native SOL directly
        if is_fogo_session {
            // Transfer wFOGO from user's ATA to automation's ATA
            let program_signer_info = program_signer_info_opt.ok_or(ProgramError::InvalidAccountData)?;
            let (program_signer_pda, bump) = Pubkey::find_program_address(&[PROGRAM_SIGNER_SEED], &oil_api::ID);
            if program_signer_info.key != &program_signer_pda {
                return Err(ProgramError::InvalidArgument);
            }
            // Authority is the session public key (signer_info), program_signer is in signers (matching place_bid.rs pattern)
            let transfer_authority = signer_info.key;
            
            let transfer_ix = transfer_checked(
                token_program_info.key,
                user_wrapped_sol_info.key,
                mint_info.key,
                automation_wrapped_sol_info.key,
                transfer_authority,
                Some(program_signer_info.key),
                CHECKPOINT_FEE,
                9, // SOL_MINT has 9 decimals
            )?;
            
            // Execute the transfer using invoke_signed with program_signer PDA (matching place_bid.rs pattern)
            solana_invoke_signed(
                &transfer_ix,
                &[
                    user_wrapped_sol_info.clone(),
                    mint_info.clone(),
                    automation_wrapped_sol_info.clone(),
                    signer_info.clone(), // authority (session public key, is a signer in original transaction)
                    token_program_info.clone(),
                    program_signer_info.clone(), // program_signer PDA (signs on behalf of session)
                ],
                &[&[PROGRAM_SIGNER_SEED, &[bump]]],
            )?;
        } else {
            // Regular wallet: transfer native SOL directly
            miner_info.collect(CHECKPOINT_FEE, signer_info)?;
        }
    }

    // Transfer deposit to automation account.
    if is_fogo_session {
        // Transfer wFOGO from user's ATA to automation's ATA
        let program_signer_info = program_signer_info_opt.ok_or(ProgramError::InvalidAccountData)?;
        let (program_signer_pda, bump) = Pubkey::find_program_address(&[PROGRAM_SIGNER_SEED], &oil_api::ID);
        if program_signer_info.key != &program_signer_pda {
            return Err(ProgramError::InvalidArgument);
        }
        // Authority is the session public key (signer_info), program_signer is in signers (matching place_bid.rs pattern)
        let transfer_authority = signer_info.key;
        
        let transfer_ix = transfer_checked(
            token_program_info.key,
            user_wrapped_sol_info.key,
            mint_info.key,
            automation_wrapped_sol_info.key,
            transfer_authority,
            Some(program_signer_info.key),
            deposit,
            9, // SOL_MINT has 9 decimals
        )?;
        
        // Execute the transfer using invoke_signed with program_signer PDA (matching place_bid.rs pattern)
        solana_invoke_signed(
            &transfer_ix,
            &[
                user_wrapped_sol_info.clone(),
                mint_info.clone(),
                automation_wrapped_sol_info.clone(),
                signer_info.clone(), // authority (session public key, is a signer in original transaction)
                token_program_info.clone(),
                program_signer_info.clone(), // program_signer PDA (signs on behalf of session)
            ],
            &[&[PROGRAM_SIGNER_SEED, &[bump]]],
        )?;
        
        // Close the automation's wrapped SOL ATA to unwrap the SOL into the automation account
        sol_log("Closing automation wrapped SOL ATA to unwrap to native SOL");
        let close_ix = close_account(
            token_program_info.key,
            automation_wrapped_sol_info.key,
            automation_info.key,      // Destination for native SOL from closing ATA
            automation_info.key,       // Owner (automation PDA)
            &[],
        )?;
        let automation_seeds: &[&[u8]] = &[AUTOMATION, &authority.to_bytes()];
        invoke_signed(
            &close_ix,
            &[
                automation_wrapped_sol_info.clone(),
                automation_info.clone(), // Destination for native SOL
                automation_info.clone(), // Owner
                token_program_info.clone(),
            ],
            &oil_api::ID,
            automation_seeds,
        )?;
        sol_log("Closed automation wrapped SOL ATA - native SOL sent to automation account");
        
        // Transfer checkpoint fee from automation account to miner account (if checkpoint fee was added in FOGO session)
        if is_fogo_session && miner.checkpoint_fee == CHECKPOINT_FEE {
            automation_info.send(CHECKPOINT_FEE, miner_info);
            sol_log("Transferred checkpoint fee from automation to miner account");
        }
    } else {
        // Regular wallet: transfer native SOL directly
        automation_info.collect(deposit, signer_info)?;
    }

    Ok(())
}
