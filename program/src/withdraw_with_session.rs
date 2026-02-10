use oil_api::prelude::*;
use oil_api::fogo;
use solana_program::log::sol_log;
use spl_token::amount_to_ui_amount;
use steel::*;

/// Withdraws OIL from the staking contract (FOGO session)
pub fn process_withdraw_with_session<'a>(accounts: &'a [AccountInfo<'a>], data: &[u8]) -> ProgramResult {
    let args = Withdraw::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);
    let stake_id = u64::from_le_bytes(args.stake_id);
    
    if stake_id != 0 {
        return Err(ProgramError::InvalidArgument);
    }

    let clock = Clock::get()?;
    let [signer_info, authority_info, program_signer_info, payer_info, mint_info, recipient_info, stake_info, stake_tokens_info, pool_info, pool_tokens_info, miner_info, treasury_info, treasury_oil_info, system_program, token_program, associated_token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    
    fogo::validate_session(signer_info)?;
    fogo::validate_program_signer(program_signer_info)?;
    
    let authority = *authority_info.key;
    
    mint_info.has_address(&MINT_ADDRESS)?.as_mint()?;
    recipient_info
        .is_writable()?
        .as_associated_token_account(authority_info.key, mint_info.key)?;
    stake_info.has_seeds(&[STAKE, &authority.to_bytes(), &stake_id.to_le_bytes()], &oil_api::ID)?;
    let stake = stake_info
        .as_account_mut::<Stake>(&oil_api::ID)?
        .assert_mut(|s| s.authority == authority)?;
    stake_tokens_info.as_associated_token_account(stake_info.key, mint_info.key)?;
    miner_info.is_writable()?;
    let pool = pool_info.as_account_mut::<Pool>(&oil_api::ID)?;
    pool_tokens_info.as_associated_token_account(pool_info.key, mint_info.key)?;
    treasury_info.is_writable()?;
    treasury_info.as_account_mut::<Treasury>(&oil_api::ID)?;
    treasury_oil_info.is_writable()?;
    treasury_oil_info.as_associated_token_account(treasury_info.key, mint_info.key)?;
    system_program.is_program(&system_program::ID)?;
    token_program.is_program(&spl_token::ID)?;
    associated_token_program.is_program(&spl_associated_token_account::ID)?;

    let is_locked = stake.is_locked(&clock);
    let penalty_percent = if is_locked {
        Stake::calculate_penalty_percent(stake.lock_duration_days)
    } else {
        0
    };

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
    }

    let old_stake_score = stake.score();
    
    let withdrawn_amount = stake.withdraw(amount, &clock, pool);
    
    let new_stake_score = stake.score();
    let stake_score_delta = old_stake_score.saturating_sub(new_stake_score);
    
    if !miner_info.data_is_empty() {
        if let Ok(miner) = miner_info.as_account_mut::<Miner>(&oil_api::ID) {
            if miner.authority == authority {
                miner.total_stake_score = miner.total_stake_score.saturating_sub(stake_score_delta);
            }
        }
    }

    let penalty_amount = if penalty_percent > 0 {
        (withdrawn_amount as u128 * penalty_percent as u128 / 100) as u64
    } else {
        0
    };
    
    let user_amount = withdrawn_amount.saturating_sub(penalty_amount);

    if penalty_amount > 0 {
        transfer_signed(
            pool_info,
            pool_tokens_info,
            treasury_oil_info,
            token_program,
            penalty_amount,
            &[POOL],
        )?;
        
        burn_signed(
            treasury_oil_info,
            mint_info,
            treasury_info,
            token_program,
            penalty_amount,
            &[TREASURY],
        )?;
        
        pool.total_burned_penalties = pool.total_burned_penalties.saturating_add(penalty_amount);
        
        sol_log(
            &format!(
                "Early withdrawal penalty: {}% = {} OIL (burned, total: {} OIL)",
                penalty_percent,
                amount_to_ui_amount(penalty_amount, TOKEN_DECIMALS),
                amount_to_ui_amount(pool.total_burned_penalties, TOKEN_DECIMALS)
            )
            .as_str(),
        );
    }

    if user_amount > 0 {
    transfer_signed(
        pool_info,
        pool_tokens_info,
        recipient_info,
        token_program,
            user_amount,
        &[POOL],
    )?;
    }

    let pool_tokens = pool_tokens_info.as_associated_token_account(pool_info.key, mint_info.key)?;
    assert!(
        pool_tokens.amount() >= pool.total_staked,
        "Pool tokens insufficient to cover total staked"
    );

    sol_log(
        &format!(
            "Withdrawing {} OIL ({} to user, {} penalty)",
            amount_to_ui_amount(withdrawn_amount, TOKEN_DECIMALS),
            amount_to_ui_amount(user_amount, TOKEN_DECIMALS),
            amount_to_ui_amount(penalty_amount, TOKEN_DECIMALS)
        )
        .as_str(),
    );

    Ok(())
}
