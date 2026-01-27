use entropy_rng_api::state::Var;
use oil_api::prelude::*;
use oil_api::consts::SOL_MINT;
use solana_program::{keccak::hashv, log::sol_log, native_token::lamports_to_sol};
use solana_program::program::invoke_signed as solana_invoke_signed;
use fogo_sessions_sdk::token::instruction::transfer_checked;
use fogo_sessions_sdk::token::PROGRAM_SIGNER_SEED;
use spl_token::instruction::close_account;
use steel::*;

/// Deploys capital to prospect on a square.
pub fn process_deploy(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    let args = Deploy::try_from_bytes(data)?;
    let mut amount = u64::from_le_bytes(args.amount);
    let mask = u32::from_le_bytes(args.squares);
    let referrer = Pubkey::new_from_array(args.referrer);
    let pooled = args.pooled != 0;

    let clock = Clock::get()?;
    let has_referrer = referrer != Pubkey::default() && referrer != *accounts[1].key;
    
    // Detect FOGO session: signer != authority and enough accounts for FOGO structure
    let is_fogo_session = accounts[0].key != accounts[1].key && accounts.len() >= 16;
    let base_accounts = if is_fogo_session { 16 } else { 12 };
    
    let oil_accounts_count = base_accounts + if has_referrer { 1 } else { 0 };
    
    if accounts.len() != oil_accounts_count + 2 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    let (oil_accounts, entropy_accounts) = accounts.split_at(oil_accounts_count);
    sol_log(&format!("Oil accounts: {}, Entropy accounts: {}, FOGO: {}", 
        oil_accounts.len(), entropy_accounts.len(), is_fogo_session));
    
    // Parse accounts based on FOGO session or regular wallet
    let (signer_info, authority_info, program_signer_info_opt, payer_info_opt, automation_info, 
         board_info, config_info, miner_info, round_info, system_program, oil_program,
         user_wrapped_sol_info_opt, round_wrapped_sol_info_opt, token_program_info, 
         mint_info, ata_program_info, referral_info_opt) = 
        if is_fogo_session {
            let expected_len = 16 + if has_referrer { 1 } else { 0 };
            if oil_accounts.len() != expected_len {
                return Err(ProgramError::NotEnoughAccountKeys);
            }
            let mut accounts_iter = oil_accounts.iter();
            oil_api::extract_accounts!(accounts_iter, [s, a, ps, pay, aut, b, c, m, r, sp, op, uws, rws, tp, mi, atap]);
            let ref_info = if has_referrer { accounts_iter.next() } else { None };
            (s, a, Some(ps), Some(pay), aut, b, c, m, r, sp, op,
             Some(uws), Some(rws), tp, mi, atap,
             ref_info)
        } else {
            let expected_len = 12 + if has_referrer { 1 } else { 0 };
            if oil_accounts.len() != expected_len {
                return Err(ProgramError::NotEnoughAccountKeys);
            }
            let mut accounts_iter = oil_accounts.iter();
            oil_api::extract_accounts!(accounts_iter, [s, a, aut, b, c, m, r, sp, op, tp, mi, atap]);
            let ref_info = if has_referrer { accounts_iter.next() } else { None };
            (s, a, None, None, aut, b, c, m, r, sp, op,
             None, None, tp, mi, atap,
             ref_info)
        };
        
    signer_info.is_signer()?;
    authority_info.is_writable()?;
    automation_info.is_writable()?.has_seeds(&[AUTOMATION, &authority_info.key.to_bytes()], &oil_api::ID)?;
    let _config = config_info.as_account::<Config>(&oil_api::ID)?;
    let board = board_info.as_account_mut::<Board>(&oil_api::ID)?;
    token_program_info.is_program(&spl_token::ID)?;
    mint_info.has_address(&SOL_MINT)?;
    ata_program_info.is_program(&spl_associated_token_account::ID)?;
    
    if let Some(user_wrapped_sol_info) = user_wrapped_sol_info_opt {
        let payer = payer_info_opt.unwrap_or(signer_info);
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
    }
    
    if board.end_slot != u64::MAX {
        board.assert_mut(|b| clock.slot >= b.start_slot && clock.slot < b.end_slot)?;
    }
    
    let mut strategy = u64::MAX;
    let automation = if !automation_info.data_is_empty() {
        let automation = automation_info
            .as_account_mut::<Automation>(&oil_api::ID)?
            .assert_mut(|a| a.executor == *signer_info.key)?
            .assert_mut(|a| a.authority == *authority_info.key)?;
        strategy = automation.strategy as u64;
        Some(automation)
    } else {
        None
    };
    
    let round = if round_info.data_is_empty() {
        round_info.is_writable()?.has_seeds(&[ROUND, &board.round_id.to_le_bytes()], &oil_api::ID)?;
        let account_creation_payer = payer_info_opt.unwrap_or(signer_info);
        create_program_account::<Round>(
            round_info,
            system_program,
            account_creation_payer,
            &oil_api::ID,
            &[ROUND, &board.round_id.to_le_bytes()],
        )?;
        let round = round_info.as_account_mut::<Round>(&oil_api::ID)?;
        round.id = board.round_id;
        round.deployed = [0; 25];
        round.slot_hash = [0; 32];
        round.count = [0; 25];
        round.expires_at = u64::MAX;
        round.rent_payer = *signer_info.key;
        round.gusher_sol = 0;
        round.top_miner = Pubkey::default();
        round.top_miner_reward = 0;
        round.total_deployed = 0;
        round.total_vaulted = 0;
        round.total_winnings = 0;
        round.deployed_pooled = [0; 25];
        round.total_pooled = 0;
        round.pool_rewards_sol = 0;
        round.pool_rewards_oil = 0;
        round.pool_members = 0;
        round.pool_cumulative = [0; 25];
        
        if let Some(round_wrapped_sol_info) = round_wrapped_sol_info_opt {
            let payer = payer_info_opt.unwrap_or(signer_info);
            create_or_validate_wrapped_sol_ata(
                round_wrapped_sol_info,
                round_info,
                mint_info,
                payer,
                system_program,
                token_program_info,
                ata_program_info,
                Some("Created round wrapped SOL ATA"),
            )?;
        }
        
        round
    } else {
        let round = round_info.as_account_mut::<Round>(&oil_api::ID)?.assert_mut(|r| r.id == board.round_id)?;
        
        if let Some(round_wrapped_sol_info) = round_wrapped_sol_info_opt {
            let payer = payer_info_opt.unwrap_or(signer_info);
            create_or_validate_wrapped_sol_ata(
                round_wrapped_sol_info,
                round_info,
                mint_info,
                payer,
                system_program,
                token_program_info,
                ata_program_info,
                Some("Created round wrapped SOL ATA (round already existed)"),
            )?;
        }
        
        round
    };
    
    miner_info.is_writable()?.has_seeds(&[MINER, &authority_info.key.to_bytes()], &oil_api::ID)?;
    system_program.is_program(&system_program::ID)?;

    if board.end_slot == u64::MAX {
        board.start_slot = clock.slot;
        board.end_slot = board.start_slot + ONE_MINUTE_SLOTS;
        round.expires_at = board.end_slot + ONE_DAY_SLOTS;

        let [var_info, entropy_program] = entropy_accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };
        var_info.has_address(&VAR_ADDRESS)?
            .as_account::<Var>(&entropy_rng_api::ID)?
            .assert(|v| v.authority == *board_info.key)?;
        entropy_program.is_program(&entropy_rng_api::ID)?;
        
        let entropy_ix = if board.round_id == 0 {
            entropy_rng_api::sdk::update(*board_info.key, *var_info.key, board.end_slot)
        } else {
            entropy_rng_api::sdk::next(*board_info.key, *var_info.key, board.end_slot)
        };
        invoke_signed(&entropy_ix, &[board_info.clone(), var_info.clone()], &oil_api::ID, &[BOARD])?;
    }

    let mut squares = [false; 25];
    if let Some(automation) = &automation {
        amount = automation.amount;
        match AutomationStrategy::from_u64(automation.strategy as u64) {
            AutomationStrategy::Preferred => {
                for i in 0..25 {
                    squares[i] = (automation.mask & (1 << i)) != 0;
                }
            }
            AutomationStrategy::Repeat => {
                // Repeat works like Preferred - uses mask to determine squares
                // Mask will be updated after deployment with squares that were actually deployed
                for i in 0..25 {
                    squares[i] = (automation.mask & (1 << i)) != 0;
                }
            }
            AutomationStrategy::Random => {
                let num_squares = ((automation.mask & 0xFF) as u64).min(25);
                if num_squares == 25 {
                    squares.fill(true);
                } else {
                    let r = hashv(&[&automation.authority.to_bytes(), &round.id.to_le_bytes()]).0;
                    squares = generate_random_mask(num_squares, &r);
                }
            }
        }
    } else {
        for i in 0..25 {
            squares[i] = (mask & (1 << i)) != 0;
        }
    }

    let is_new_miner = miner_info.data_is_empty();
    let miner = if is_new_miner {
        let account_creation_payer = payer_info_opt.unwrap_or(signer_info);
        create_program_account::<Miner>(
            miner_info,
            system_program,
            account_creation_payer,
            &oil_api::ID,
            &[MINER, &authority_info.key.to_bytes()],
        )?;
        let miner = miner_info.as_account_mut::<Miner>(&oil_api::ID)?;
        miner.initialize(*authority_info.key);
        
        if referrer != Pubkey::default() && referrer != *authority_info.key {
            miner.referrer = referrer;
            Referral::process_new_miner_referral(
                referral_info_opt,
                referrer,
                *authority_info.key,
            )?;
        }
        miner
    } else {
        miner_info.as_account_mut::<Miner>(&oil_api::ID)?.assert_mut(|m| {
            if let Some(automation) = &automation {
                m.authority == automation.authority
            } else {
                m.authority == *authority_info.key
            }
        })?
    };

    if miner.round_id != round.id {
        assert!(miner.checkpoint_id == miner.round_id, "Miner has not checkpointed");
        miner.deployed = [0; 25];
        miner.cumulative = round.deployed;
        miner.round_id = round.id;
        miner.pooled_deployed = 0;
    }

    let is_first_deploy = miner.deployed.iter().sum::<u64>() == 0;
    let mut total_amount = 0;
    let mut total_squares = 0;
    let mut deployed_squares_this_tx = [false; 25]; // Track squares deployed in this transaction
    for (square_id, &should_deploy) in squares.iter().enumerate() {
        if square_id > 24 {
            break;
        }
        if !should_deploy {
            continue;
        }
        if miner.deployed[square_id] > 0 {
            continue;
        }

        miner.cumulative[square_id] = round.deployed[square_id];
        miner.deployed[square_id] = amount;
        round.deployed[square_id] += amount;
        round.total_deployed += amount;
        round.count[square_id] += 1;
        total_amount += amount;
        total_squares += 1;
        deployed_squares_this_tx[square_id] = true; // Mark as deployed in this transaction

        if let Some(automation) = &automation {
            if total_amount + automation.fee + amount > automation.balance {
                break;
            }
        }
    }
    
    if pooled && total_amount > 0 {
        if miner.pooled_deployed == 0 {
            round.pool_members += 1;
        }
        miner.pooled_deployed += total_amount;
        for (square_id, &should_deploy) in squares.iter().enumerate() {
            if square_id > 24 {
                break;
            }
            if should_deploy && miner.deployed[square_id] > 0 {
                if round.deployed_pooled[square_id] == 0 {
                    round.pool_cumulative[square_id] = round.deployed[square_id].saturating_sub(miner.deployed[square_id]);
                }
                round.deployed_pooled[square_id] += miner.deployed[square_id];
            }
        }
        round.total_pooled += total_amount;
    }

    if is_first_deploy && total_amount > 0 {
        round.total_miners += 1;
    }

    miner.lifetime_deployed += total_amount;

    if miner.checkpoint_fee == 0 {
        miner.checkpoint_fee = CHECKPOINT_FEE;
        miner_info.collect(CHECKPOINT_FEE, payer_info_opt.unwrap_or(signer_info))?;
    }

    if let Some(automation) = automation {
        automation.balance -= total_amount + automation.fee;
        automation_info.send(total_amount, &round_info);
        automation_info.send(automation.fee, &signer_info);
        
        // For Repeat strategy, update mask with squares that were actually deployed in this transaction
        if AutomationStrategy::from_u64(automation.strategy as u64) == AutomationStrategy::Repeat {
            let mut new_mask = 0u64;
            // Build mask from squares that were deployed in THIS transaction only
            for i in 0..25 {
                if deployed_squares_this_tx[i] {
                    new_mask |= 1 << i;
                }
            }
            automation.mask = new_mask;
        }
        
        if automation.balance < automation.amount + automation.fee {
            automation_info.close(authority_info)?;
        }
    } else {
        if let (Some(user_wrapped_sol_info), Some(round_wrapped_sol_info)) = (user_wrapped_sol_info_opt, round_wrapped_sol_info_opt) {
            if user_wrapped_sol_info.data_is_empty() {
                return Err(ProgramError::InvalidAccountData);
            }
            user_wrapped_sol_info.as_associated_token_account(authority_info.key, mint_info.key)?;
            if round_wrapped_sol_info.data_is_empty() {
                return Err(ProgramError::InvalidAccountData);
            }
            round_wrapped_sol_info.as_associated_token_account(round_info.key, mint_info.key)?;

            let transfer_ix = transfer_checked(
                token_program_info.key,
                user_wrapped_sol_info.key,
                mint_info.key,
                round_wrapped_sol_info.key,
                signer_info.key,
                program_signer_info_opt.map(|info| info.key),
                total_amount,
                9,
            )?;
            
            let program_signer = program_signer_info_opt.ok_or(ProgramError::InvalidAccountData)?;
            let (program_signer_pda, bump) = Pubkey::find_program_address(&[PROGRAM_SIGNER_SEED], &oil_api::ID);
            if program_signer.key != &program_signer_pda {
                return Err(ProgramError::InvalidArgument);
            }
            solana_invoke_signed(
                &transfer_ix,
                &[
                    user_wrapped_sol_info.clone(),
                    mint_info.clone(),
                    round_wrapped_sol_info.clone(),
                    signer_info.clone(),
                    token_program_info.clone(),
                    program_signer.clone(),
                ],
                &[&[PROGRAM_SIGNER_SEED, &[bump]]],
            )?;
            
            // Close the wrapped SOL ATA to unwrap the SOL into the round account
            sol_log("Closing round wrapped SOL ATA to unwrap to native SOL");
            let close_ix = close_account(
                token_program_info.key,
                round_wrapped_sol_info.key,
                round_info.key,      // Destination for native SOL from closing ATA
                round_info.key,       // Owner (round PDA)
                &[],
            )?;
            let round_id_bytes = round.id.to_le_bytes();
            let round_seeds: &[&[u8]] = &[ROUND, &round_id_bytes];
            invoke_signed(
                &close_ix,
                &[
                    round_wrapped_sol_info.clone(),
                    round_info.clone(), // Destination for native SOL
                    round_info.clone(), // Owner
                    token_program_info.clone(),
                ],
                &oil_api::ID,
                round_seeds,
            )?;
            sol_log("Closed round wrapped SOL ATA - native SOL sent to round account");
        } else {
            round_info.collect(total_amount, signer_info)?;
        }
    }

    program_log(
        &[board_info.clone(), oil_program.clone()],
        DeployEvent {
            disc: 2,
            authority: miner.authority,
            amount,
            mask: mask as u64,
            round_id: round.id,
            signer: *signer_info.key,
            strategy,
            total_squares,
            ts: clock.unix_timestamp,
        }
        .to_bytes(),
    )?;

    sol_log(&format!(
        "Round #{}: deploying {} SOL to {} squares{}",
        round.id,
        lamports_to_sol(amount),
        total_squares,
        if pooled { " (pooled)" } else { "" },
    ));

    Ok(())
}

fn generate_random_mask(num_squares: u64, r: &[u8]) -> [bool; 25] {
    let mut new_mask = [false; 25];
    let mut selected = 0;
    for i in 0..25 {
        let rand_byte = r[i];
        let remaining_needed = num_squares - selected;
        let remaining_positions = 25 - i;
        if remaining_needed > 0 && (rand_byte as u64) * (remaining_positions as u64) < (remaining_needed * 256) {
            new_mask[i] = true;
            selected += 1;
        }
    }
    new_mask
}
