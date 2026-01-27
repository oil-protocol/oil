use oil_api::prelude::*;
use solana_program::program::invoke;
use solana_program::program::invoke_signed;
use solana_program::log::sol_log;
use fogo_sessions_sdk::token::instruction::transfer_checked;
use fogo_sessions_sdk::token::PROGRAM_SIGNER_SEED;
use spl_token::amount_to_ui_amount;
use steel::*;

/// Deposits OIL into the staking contract. Stakers earn SOL rewards from protocol revenue (2% of round winnings).
pub fn process_deposit(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse data.
    let args = Deposit::try_from_bytes(data)?;
    let amount = u64::from_le_bytes(args.amount);
    let lock_duration_days = u64::from_le_bytes(args.lock_duration_days);
    let stake_id = u64::from_le_bytes(args.stake_id);
    
    // Only allow a single stake account per user (stake_id must be 0)
    if stake_id != 0 {
        return Err(ProgramError::InvalidArgument);
    }

    // Load accounts.
    let clock = Clock::get()?;
    
    // Check account count to determine structure
    let (signer_info, authority_info, program_signer_info, payer_info, mint_info, sender_info, stake_info, stake_tokens_info, pool_info, pool_tokens_info, miner_info, system_program, token_program, associated_token_program) = 
        if accounts.len() >= 14 {
            // Fogo session with program_signer and payer
            let [s, a, ps, pay, m, se, st, stt, p, pt, mi, sp, tp, atp] = &accounts[0..14] else {
                return Err(ProgramError::NotEnoughAccountKeys);
            };
            (s, a, Some(ps), Some(pay), m, se, st, stt, p, pt, mi, sp, tp, atp)
        } else if accounts.len() >= 13 {
            // Fogo session with program_signer but no separate payer (use signer as payer)
            let [s, a, ps, m, se, st, stt, p, pt, mi, sp, tp, atp] = &accounts[0..13] else {
                return Err(ProgramError::NotEnoughAccountKeys);
            };
            (s, a, Some(ps), None, m, se, st, stt, p, pt, mi, sp, tp, atp)
        } else {
            // Regular wallet (no program_signer, no separate payer)
            let [s, a, m, se, st, stt, p, pt, mi, sp, tp, atp] = &accounts[0..12] else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
            (s, a, None, None, m, se, st, stt, p, pt, mi, sp, tp, atp)
        };
    
    signer_info.is_signer()?;
    
    let authority = *authority_info.key;
    let signer_key = *signer_info.key;
    
    let is_fogo_session = authority != signer_key;
    
    if !is_fogo_session {
        // Regular wallet case: authority must match signer
        if authority != signer_key {
            return Err(ProgramError::InvalidAccountData);
        }
    }
    
    // Use authority as user for the rest of the function
    let user = authority;
    
    // Validate mint
    mint_info.has_address(&MINT_ADDRESS)?.as_mint()?;
    
    // Validate sender ATA is owned by the user (extracted from session or signer)
    let sender = sender_info
        .is_writable()?
        .as_associated_token_account(&user, &MINT_ADDRESS)?;
    
    // Validate writable accounts
    stake_info.is_writable()?;
    miner_info.is_writable()?;
    pool_info.is_writable()?;
    
    // Pool account must exist (created during initialize)
    let pool = pool_info.as_account_mut::<Pool>(&oil_api::ID)
        .map_err(|_| ProgramError::InvalidAccountData)?;
    pool_tokens_info.as_associated_token_account(pool_info.key, mint_info.key)?;
    system_program.is_program(&system_program::ID)?;
    token_program.is_program(&spl_token::ID)?;
    associated_token_program.is_program(&spl_associated_token_account::ID)?;

    // Determine payer for account creation
    let account_creation_payer = payer_info.unwrap_or(signer_info);

    // Open stake account.
    let stake = if stake_info.data_is_empty() {
        // New stake account
        create_program_account::<Stake>(
            stake_info,
            system_program,
            account_creation_payer, // Use paymaster sponsor for Fogo sessions, signer for regular
            &oil_api::ID,
            &[STAKE, &authority.to_bytes(), &stake_id.to_le_bytes()],
        )?;
        let stake = stake_info.as_account_mut::<Stake>(&oil_api::ID)?;
        stake.authority = authority;
        stake.balance = 0;
        stake.lock_duration_days = lock_duration_days;
        // Calculate lock_ends_at: current timestamp + (lock_duration_days * 86400 seconds)
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
        // Existing stake account
        let stake = stake_info
            .as_account_mut::<Stake>(&oil_api::ID)?
            .assert_mut(|s| s.authority == authority)?;
        
        // Check if lock_duration_days matches existing stake
        if stake.lock_duration_days > 0 && lock_duration_days != stake.lock_duration_days {
            return Err(ProgramError::InvalidArgument);
        }
        // If stake has no lock (lock_duration_days == 0) and user wants to set a lock, allow it
        if stake.lock_duration_days == 0 && lock_duration_days > 0 {
            // Setting lock for the first time on existing stake
            stake.lock_duration_days = lock_duration_days;
            stake.lock_ends_at = (clock.unix_timestamp as u64) + (lock_duration_days * 86400);
        }
        // If stake has a lock and user deposits more, reset the lock timer (like macaron.bid)
        if stake.lock_duration_days > 0 && lock_duration_days == stake.lock_duration_days {
            // Reset lock timer when depositing more into existing locked stake
            stake.lock_ends_at = (clock.unix_timestamp as u64) + (lock_duration_days * 86400);
        }
        
        stake
    };

    // Create stake tokens account.
    if stake_tokens_info.data_is_empty() {
        create_associated_token_account(
            account_creation_payer, // Use paymaster sponsor for Fogo sessions, signer for regular
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

    // Only allow deposits from seekers.
    // assert!(stake.is_seeker == 1, "Only seekers can deposit stake");

    // Calculate old score before deposit (for miner account update)
    let old_stake_score = stake.score();
    
    // Deposit into stake account (updates balance and pool.total_staked_score)
    let amount = stake.deposit(amount, &clock, pool, &sender);
    
    // Log deposit.
    sol_log(
        &format!(
            "Depositing {} OIL",
            amount_to_ui_amount(amount, TOKEN_DECIMALS)
        )
        .as_str(),
    );
    
    // Calculate new score after deposit
    let new_stake_score = stake.score();
    let stake_score_delta = new_stake_score.saturating_sub(old_stake_score);
    
    // Update miner account's total_stake_score
    if !miner_info.data_is_empty() {
        if let Ok(miner) = miner_info.as_account_mut::<Miner>(&oil_api::ID) {
            if miner.authority == authority {
                miner.total_stake_score = miner.total_stake_score.saturating_add(stake_score_delta);
            }
        }
    }

    let transfer_authority = if is_fogo_session {
        signer_info.key // Session public key for Fogo sessions
    } else {
        authority_info.key // User wallet for regular transactions
    };
    
    let transfer_ix = transfer_checked(
        token_program.key,
        sender_info.key,
        mint_info.key,
        pool_tokens_info.key,
        transfer_authority, // Session public key for Fogo sessions, user wallet for regular
        program_signer_info.map(|info| info.key), // program_signer (PDA that will sign on behalf of session)
        amount,
        11, // TOKEN_DECIMALS
    )?;
    
    // Execute the transfer based on whether it's a session or not
    match (is_fogo_session, program_signer_info) {
        (_, Some(program_signer)) => {
            // Fogo session case: use program_signer PDA to sign the token transfer
            let (program_signer_pda, bump) = Pubkey::find_program_address(
                &[PROGRAM_SIGNER_SEED],
                &oil_api::ID,
            );
            
            // Validate program_signer matches expected PDA
            if program_signer.key != &program_signer_pda {
                return Err(ProgramError::InvalidArgument);
            }
            
            // Invoke with program_signer signature
            invoke_signed(
                &transfer_ix,
                &[
                    sender_info.clone(),      // source account
                    mint_info.clone(),         // mint
                    pool_tokens_info.clone(),  // destination
                    signer_info.clone(),       // authority (session public key, is a signer in original transaction)
                    token_program.clone(),     // token program
                    program_signer.clone(),    // program_signer PDA (signs on behalf of session)
                ],
                &[&[PROGRAM_SIGNER_SEED, &[bump]]],
            )?;
        }
        (false, _) => {
            // Regular wallet case: direct invoke (signer is the user's wallet)
            invoke(
                &transfer_ix,
                &[
                    signer_info.clone(),
                    sender_info.clone(),
                    mint_info.clone(),
                    pool_tokens_info.clone(),
                    token_program.clone(),
                ],
            )?;
        }
        (true, _) => {
            // Fogo session but no program_signer - this is an error
            return Err(ProgramError::NotEnoughAccountKeys);
        }
    }

    // Safety check: Verify pool has enough tokens to cover all stakes.
    let pool_tokens = pool_tokens_info.as_associated_token_account(pool_info.key, mint_info.key)?;
    assert!(
        pool_tokens.amount() >= pool.total_staked,
        "Pool tokens insufficient to cover total staked"
    );

    Ok(())
}