use oil_api::prelude::*;
use oil_api::fogo;
use solana_program::log::sol_log;
use spl_token::amount_to_ui_amount;
use steel::*;

/// Deposits OIL into the staking contract. Stakers earn SOL rewards from protocol revenue (2% of round winnings).
pub fn process_deposit_with_session<'a>(accounts: &'a [AccountInfo<'a>], data: &[u8]) -> ProgramResult {
    let args = Deposit::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);
    let lock_duration_days = u64::from_le_bytes(args.lock_duration_days);
    let stake_id = u64::from_le_bytes(args.stake_id);
    
    if stake_id != 0 {
        return Err(ProgramError::InvalidArgument);
    }

    let clock = Clock::get()?;
    
    let [signer_info, authority_info, program_signer_info, payer_info, mint_info, sender_info, stake_info, stake_tokens_info, pool_info, pool_tokens_info, miner_info, system_program, token_program, associated_token_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    
    signer_info.is_signer()?;
    
    fogo::validate_session(signer_info)?;
    fogo::validate_program_signer(program_signer_info)?;
    
    let authority = *authority_info.key;
    
    let user = authority;
    
    mint_info.has_address(&MINT_ADDRESS)?.as_mint()?;
    
    let sender = sender_info
        .is_writable()?
        .as_associated_token_account(&user, &MINT_ADDRESS)?;
    
    stake_info.is_writable()?;
    miner_info.is_writable()?;
    pool_info.is_writable()?;
    
    let pool = pool_info.as_account_mut::<Pool>(&oil_api::ID)
        .map_err(|_| ProgramError::InvalidAccountData)?;
    pool_tokens_info.as_associated_token_account(pool_info.key, mint_info.key)?;
    system_program.is_program(&system_program::ID)?;
    token_program.is_program(&spl_token::ID)?;
    associated_token_program.is_program(&spl_associated_token_account::ID)?;

    let stake = if stake_info.data_is_empty() {
        create_program_account::<Stake>(
            stake_info,
            system_program,
            payer_info,
            &oil_api::ID,
            &[STAKE, &authority.to_bytes(), &stake_id.to_le_bytes()],
        )?;
        let stake = stake_info.as_account_mut::<Stake>(&oil_api::ID)?;
        stake.authority = authority;
        stake.balance = 0;
        stake.lock_duration_days = lock_duration_days;
        stake.lock_ends_at = if lock_duration_days > 0 {
            (clock.unix_timestamp as u64) + (lock_duration_days * 86400)
        } else {
            0
        };
        stake.buffer_c = 0;
        stake.buffer_d = 0;
        stake.buffer_e = 0;
        stake.last_claim_at = 0;
        stake.last_deposit_at = 0;
        stake.last_withdraw_at = 0;
        stake.rewards_factor = pool.stake_rewards_factor;
        stake.rewards = 0;
        stake.lifetime_rewards = 0;
        stake.buffer_f = 0;
        stake
    } else {
        let stake = stake_info
            .as_account_mut::<Stake>(&oil_api::ID)?
            .assert_mut(|s| s.authority == authority)?;
        
        if stake.lock_duration_days > 0 && lock_duration_days != stake.lock_duration_days {
            return Err(ProgramError::InvalidArgument);
        }
        if stake.lock_duration_days == 0 && lock_duration_days > 0 {
            stake.lock_duration_days = lock_duration_days;
            stake.lock_ends_at = (clock.unix_timestamp as u64) + (lock_duration_days * 86400);
        }
        if stake.lock_duration_days > 0 && lock_duration_days == stake.lock_duration_days {
            stake.lock_ends_at = (clock.unix_timestamp as u64) + (lock_duration_days * 86400);
        }
        
        stake
    };

    if stake_tokens_info.data_is_empty() {
        create_associated_token_account(
            payer_info,
            stake_info,
            stake_tokens_info,
            mint_info,
            system_program,
            token_program,
            associated_token_program,
        )?;
    } else {
        stake_tokens_info.as_associated_token_account(stake_info.key, mint_info.key)?;
    }

    let old_stake_score = stake.score();
    
    let amount = stake.deposit(amount, &clock, pool, &sender);
    
    sol_log(
        &format!(
            "Depositing {} OIL",
            amount_to_ui_amount(amount, TOKEN_DECIMALS)
        )
        .as_str(),
    );
    
    let new_stake_score = stake.score();
    let stake_score_delta = new_stake_score.saturating_sub(old_stake_score);
    
    if !miner_info.data_is_empty() {
        if let Ok(miner) = miner_info.as_account_mut::<Miner>(&oil_api::ID) {
            if miner.authority == authority {
                miner.total_stake_score = miner.total_stake_score.saturating_add(stake_score_delta);
            }
        }
    }

    fogo::transfer_token_with_program_signer(
        token_program,
        sender_info,
        mint_info,
        pool_tokens_info,
        signer_info,
        program_signer_info,
        amount,
    )?;

    let pool_tokens = pool_tokens_info.as_associated_token_account(pool_info.key, mint_info.key)?;
    assert!(
        pool_tokens.amount() >= pool.total_staked,
        "Pool tokens insufficient to cover total staked"
    );

    Ok(())
}
