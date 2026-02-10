use oil_api::prelude::*;
use oil_api::consts::SOL_MINT;
use oil_api::fogo;
use oil_api::utils::create_or_validate_wrapped_sol_ata;
use solana_program::pubkey::Pubkey;
use steel::*;

pub fn process_automate_with_session<'a>(accounts: &'a [AccountInfo<'a>], data: &[u8]) -> ProgramResult {
    let args = Automate::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);
    let deposit = u64::from_le_bytes(args.deposit);
    let fee = u64::from_le_bytes(args.fee);
    let mask = u64::from_le_bytes(args.mask);
    let strategy = AutomationStrategy::from_u64(args.strategy as u64);
    let reload = u64::from_le_bytes(args.reload) > 0;
    let referrer = Pubkey::new_from_array(args.referrer);
    let pooled = args.pooled != 0;

    let has_referral = referrer != Pubkey::default();
    let expected_len = 14 + if has_referral { 1 } else { 0 };
    
    if accounts.len() < expected_len {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    let mut accounts_iter = accounts.iter();
    oil_api::extract_accounts!(accounts_iter, [s, a, ps, pay, aut, e, m, sp, op, uws, aws, tp, mi, atap]);
    let ref_info = if has_referral { accounts_iter.next() } else { None };
    let (signer_info, authority_info, program_signer_info, payer_info, automation_info, executor_info, miner_info, system_program, oil_program,
         user_wrapped_sol_info, automation_wrapped_sol_info, token_program_info, mint_info, ata_program_info,
         referral_info_opt) = (s, a, ps, pay, aut, e, m, sp, op, uws, aws, tp, mi, atap, ref_info);
    
    signer_info.is_signer()?;
    
    fogo::validate_session(signer_info)?;
    fogo::validate_program_signer(program_signer_info)?;
    
    automation_info.is_writable()?;
    system_program.is_program(&system_program::ID)?;
    oil_program.is_program(&oil_api::ID)?;
    
    let authority = *authority_info.key;
    
    token_program_info.is_program(&spl_token::ID)?;
    mint_info.has_address(&SOL_MINT)?;
    ata_program_info.is_program(&spl_associated_token_account::ID)?;

    let is_new_miner = miner_info.data_is_empty();
    let miner = if is_new_miner {
        create_program_account::<Miner>(
            miner_info,
            system_program,
            payer_info,
            &oil_api::ID,
            &[MINER, &authority.to_bytes()],
        )?;
        let miner = miner_info.as_account_mut::<Miner>(&oil_api::ID)?;
        miner.initialize(authority);

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

    if *executor_info.key == Pubkey::default() {
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

    let automation = if automation_info.data_is_empty() {
        create_program_account::<Automation>(
            automation_info,
            system_program,
            payer_info,
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

    // Create user's wrapped SOL ATA if needed (needed for transfers)
    create_or_validate_wrapped_sol_ata(
        user_wrapped_sol_info,
        authority_info,
        mint_info,
        payer_info,
        system_program,
        token_program_info,
        ata_program_info,
        None,
    )?;
    
    // Create automation's wrapped SOL ATA if needed (temporary, just for Fogo session transfers)
    create_or_validate_wrapped_sol_ata(
        automation_wrapped_sol_info,
        automation_info,
        mint_info,
        payer_info,
        system_program,
        token_program_info,
        ata_program_info,
        Some("automation_wrapped_sol"),
    )?;

    automation.amount = amount;
    automation.balance += deposit;
    automation.executor = *executor_info.key;
    automation.fee = fee;
    automation.mask = mask;
    automation.strategy = strategy as u64;
    automation.reload = reload as u64;
    automation.pooled = pooled as u64;

    let automation_seeds: &[&[u8]] = &[AUTOMATION, &authority.to_bytes()];

    if miner.checkpoint_fee == 0 {
        miner.checkpoint_fee = CHECKPOINT_FEE;
        
        fogo::transfer_wrapped_sol(
            signer_info,
            program_signer_info,
            CHECKPOINT_FEE,
            user_wrapped_sol_info,
            automation_wrapped_sol_info,
            mint_info,
            token_program_info,
        )?;
        
        let close_ix = spl_token::instruction::close_account(
            token_program_info.key,
            automation_wrapped_sol_info.key,
            automation_info.key,
            automation_info.key,
            &[],
        )?;
        invoke_signed(
            &close_ix,
            &[
                automation_wrapped_sol_info.clone(),
                automation_info.clone(),
                automation_info.clone(),
                token_program_info.clone(),
            ],
            &oil_api::ID,
            automation_seeds,
        )?;
        
        automation_info.send(CHECKPOINT_FEE, miner_info);
    }
    
    fogo::transfer_wrapped_sol(
        signer_info,
        program_signer_info,
        deposit,
        user_wrapped_sol_info,
        automation_wrapped_sol_info,
        mint_info,
        token_program_info,
    )?;
    
    let close_ix = spl_token::instruction::close_account(
        token_program_info.key,
        automation_wrapped_sol_info.key,
        automation_info.key,
        automation_info.key,
        &[],
    )?;
    invoke_signed(
        &close_ix,
        &[
            automation_wrapped_sol_info.clone(),
            automation_info.clone(),
            automation_info.clone(),
            token_program_info.clone(),
        ],
        &oil_api::ID,
        automation_seeds,
    )?;

    if miner.checkpoint_fee == CHECKPOINT_FEE {
        automation_info.send(CHECKPOINT_FEE, miner_info);
    }

    Ok(())
}
