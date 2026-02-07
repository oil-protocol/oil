use oil_api::prelude::*;
use solana_program::pubkey::Pubkey;
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
    
    let has_referral = referrer != Pubkey::default();
    let expected_len = 7 + if has_referral { 1 } else { 0 };
    
            if accounts.len() < expected_len {
                return Err(ProgramError::NotEnoughAccountKeys);
            }
    
            let mut accounts_iter = accounts.iter();
    oil_api::extract_accounts!(accounts_iter, [s, a, aut, e, m, sp, op]);
            let ref_info = if has_referral { accounts_iter.next() } else { None };
    let (signer_info, authority_info, automation_info, executor_info, miner_info, system_program, oil_program,
         referral_info_opt) = (s, a, aut, e, m, sp, op, ref_info);
    
    signer_info.is_signer()?;
    automation_info.is_writable()?;
    system_program.is_program(&system_program::ID)?;
    oil_program.is_program(&oil_api::ID)?;
    
    let authority = *authority_info.key;
    
    let is_new_miner = miner_info.data_is_empty();
    let miner = if is_new_miner {
        create_program_account::<Miner>(
            miner_info,
            system_program,
            signer_info,
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

    let automation = if automation_info.data_is_empty() {
        create_program_account::<Automation>(
            automation_info,
            system_program,
            signer_info,
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

    // Set strategy and mask.
    automation.amount = amount;
    automation.balance += deposit;
    automation.executor = *executor_info.key;
    automation.fee = fee;
    automation.mask = mask;
    automation.strategy = strategy as u64;
    automation.reload = reload as u64;
    automation.pooled = pooled as u64;

    if miner.checkpoint_fee == 0 {
        miner.checkpoint_fee = CHECKPOINT_FEE;
            miner_info.collect(CHECKPOINT_FEE, signer_info)?;
        }

        automation_info.collect(deposit, signer_info)?;

    Ok(())
}
