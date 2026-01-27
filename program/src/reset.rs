use entropy_rng_api::state::Var;
use oil_api::prelude::*;
use oil_api::consts::{POOL_ADDRESS, SOL_MINT};
use solana_program::{keccak, log::sol_log, native_token::lamports_to_sol};
use steel::*;

/// Pays out the winners and block reward.
pub fn process_reset(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let clock = Clock::get()?;
    let (oil_accounts, other_accounts) = accounts.split_at(16);
    sol_log(&format!("Oil accounts: {:?}", oil_accounts.len()).to_string());
    sol_log(&format!("Other accounts: {:?}", other_accounts.len()).to_string());
    let [signer_info, board_info, config_info, fee_collector_info, mint_info, round_info, round_next_info, top_miner_info, treasury_info, pool_info, treasury_tokens_info, system_program, token_program, oil_program, slot_hashes_sysvar, sol_mint_info] =
        oil_accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    let board = board_info
        .as_account_mut::<Board>(&oil_api::ID)?
        .assert_mut(|b| {
            // Check if round has ended (end_slot != u64::MAX) and intermission has passed
            if b.end_slot == u64::MAX {
                return false; // Round hasn't ended yet
            }
            // Use saturating_add to prevent overflow
            let reset_slot = b.end_slot.saturating_add(INTERMISSION_SLOTS);
            clock.slot >= reset_slot
        })?;
    let config = config_info.as_account_mut::<Config>(&oil_api::ID)?;
    fee_collector_info
        .is_writable()?
        .has_address(&config.fee_collector)?;
    let round = round_info
        .as_account_mut::<Round>(&oil_api::ID)?
        .assert_mut(|r| r.id == board.round_id)?;
    round_next_info
        .is_empty()?
        .is_writable()?
        .has_seeds(&[ROUND, &(board.round_id + 1).to_le_bytes()], &oil_api::ID)?;
    let mint = mint_info.has_address(&MINT_ADDRESS)?.as_mint()?;
    let treasury = treasury_info.as_account_mut::<Treasury>(&oil_api::ID)?;
    let pool = pool_info.as_account_mut::<Pool>(&oil_api::ID)?;
    treasury_tokens_info.as_associated_token_account(&treasury_info.key, &mint_info.key)?;
    system_program.is_program(&system_program::ID)?;
    token_program.is_program(&spl_token::ID)?;
    oil_program.is_program(&oil_api::ID)?;
    slot_hashes_sysvar.is_sysvar(&sysvar::slot_hashes::ID)?;
    sol_mint_info.has_address(&SOL_MINT)?;

    // Open next round account.
    create_program_account::<Round>(
        round_next_info,
        oil_program,
        signer_info,
        &oil_api::ID,
        &[ROUND, &(board.round_id + 1).to_le_bytes()],
    )?;
    let round_next = round_next_info.as_account_mut::<Round>(&oil_api::ID)?;
    round_next.id = board.round_id + 1;
    round_next.deployed = [0; 25];
    round_next.slot_hash = [0; 32];
    round_next.count = [0; 25];
    round_next.expires_at = u64::MAX; // Set to max, to indicate round is waiting for first deploy to begin.
    round_next.rent_payer = *signer_info.key;
    round_next.gusher_sol = 0;
    round_next.top_miner = Pubkey::default();
    round_next.top_miner_reward = 0;
    round_next.total_deployed = 0;
    round_next.total_vaulted = 0;
    round_next.total_winnings = 0;
    // Pool fields
    round_next.deployed_pooled = [0; 25];
    round_next.total_pooled = 0;
    round_next.pool_rewards_sol = 0;
    round_next.pool_rewards_oil = 0;
    round_next.pool_members = 0;
    round_next.pool_cumulative = [0; 25];

    // Sample random variable
    let (entropy_accounts, mint_accounts) = other_accounts.split_at(2);
    sol_log(&format!("Entropy accounts: {:?}", entropy_accounts.len()).to_string());
    sol_log(&format!("Mint accounts: {:?}", mint_accounts.len()).to_string());
    let [var_info, entropy_program] = entropy_accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    let var = var_info
        .has_address(&VAR_ADDRESS)?
        .as_account::<Var>(&entropy_rng_api::ID)?
        .assert(|v| v.authority == *board_info.key)?
        .assert(|v| v.slot_hash != [0; 32])?
        .assert(|v| v.seed != [0; 32])?
        .assert(|v| v.value != [0; 32])?;
    entropy_program.is_program(&entropy_rng_api::ID)?;

    // Print the seed and slot hash.
    let seed = keccak::Hash::new_from_array(var.seed);
    let slot_hash = keccak::Hash::new_from_array(var.slot_hash);
    sol_log(&format!("var slothash: {:?}", slot_hash).to_string());
    sol_log(&format!("var seed: {:?}", seed).to_string());

    // Read the finalized value from the var.
    let value = keccak::Hash::new_from_array(var.value);
    sol_log(&format!("var value: {:?}", value).to_string());
    round.slot_hash = var.value;

    // Exit early if no slot hash was found.
    let Some(r) = round.rng() else {
        // Slot hash could not be found, refund all SOL.
        round.total_vaulted = 0;
        round.total_winnings = 0;
        round.total_deployed = 0;

        // Emit event.
        program_log(
            &[board_info.clone(), oil_program.clone()],
            ResetEvent {
                disc: 0,
                round_id: round.id,
                start_slot: board.start_slot,
                end_slot: board.end_slot,
                winning_square: u64::MAX,
                top_miner: Pubkey::default(),
                num_winners: 0,
                total_deployed: round.total_deployed,
                total_vaulted: round.total_vaulted,
                total_winnings: round.total_winnings,
                total_minted: 0,
                ts: clock.unix_timestamp,
                gusher_sol: round.gusher_sol,
            }
            .to_bytes(),
        )?;

        // Update board for next round.
        board.round_id += 1;
        board.start_slot = clock.slot + 1;
        board.end_slot = u64::MAX;
        return Ok(());
    };

    // Check if we're in pre-mine phase
    let is_premine = config.tge_timestamp > 0 && clock.unix_timestamp < config.tge_timestamp;
    
    // Calculate admin fees
    // During premine: 2% admin fee, otherwise 1%
    let deployment_admin_fee_rate = if is_premine { 2 } else { 1 };
    let total_admin_fee = (round.total_deployed * deployment_admin_fee_rate) / 100;

    // Get the winning square.
    let winning_square = round.winning_square(r);
    sol_log(&format!("round.id: {:?}", round.id).to_string());
    sol_log(&format!("winning_square: {:?}", winning_square).to_string());
    // If no one deployed on the winning square, vault all deployed.
    if round.deployed[winning_square] == 0 {
        // Vault all deployed.
        round.total_vaulted = round.total_deployed - total_admin_fee;
        treasury.balance += round.total_deployed - total_admin_fee;

        // Emit event.
        program_log(
            &[board_info.clone(), oil_program.clone()],
            ResetEvent {
                disc: 0,
                round_id: round.id,
                start_slot: board.start_slot,
                end_slot: board.end_slot,
                winning_square: winning_square as u64,
                top_miner: Pubkey::default(),
                num_winners: 0,
                total_deployed: round.total_deployed,
                total_vaulted: round.total_vaulted,
                total_winnings: round.total_winnings,
                total_minted: 0,
                ts: clock.unix_timestamp,
                gusher_sol: 0, // No gusher if no winners
            }
            .to_bytes(),
        )?;

        // Update board for next round.
        board.round_id += 1;
        board.start_slot = clock.slot + 1;
        board.end_slot = u64::MAX;

        // Do SOL transfers.
        round_info.send(total_admin_fee, &fee_collector_info);
        round_info.send(round.total_deployed - total_admin_fee, &treasury_info);
        return Ok(());
    }

    // Get winnings amount (total deployed on all non-winning squares).
    let original_winnings = round.calculate_total_winnings(winning_square);
    
    // Calculate all percentages from original winnings:
    // - 1% admin fee (2% during premine)
    // - 8% buybacks
    // - 2% staking SOL rewards
    // - 1% gusher SOL
    // - 88% winners (87% during premine, since extra 1% admin fee comes from miner rewards)
    let admin_fee_rate = if is_premine { 2 } else { 1 };
    let winnings_admin_fee = (original_winnings * admin_fee_rate) / 100;
    let buyback_amount = (original_winnings * 8) / 100; // 8% of original winnings for buybacks.
    let staking_sol_amount = (original_winnings * 2) / 100; // 2% of original winnings for staking SOL rewards.
    let gusher_sol_amount = original_winnings / 100; // 1% of original winnings for gusher_sol.
    let total_protocol_revenue = buyback_amount + staking_sol_amount + gusher_sol_amount; // 11% total
    
    // Winners get 88% normally, or 87% during premine (extra 1% admin fee comes from miner rewards)
    // Formula: original_winnings - admin_fee - protocol_revenue
    let winnings = original_winnings - winnings_admin_fee - total_protocol_revenue;
    round.total_winnings = winnings;
    round.total_vaulted = total_protocol_revenue;
    
    // Calculate pool rewards based on pool's share of the winning square.
    let winning_pool_amount = round.deployed_pooled[winning_square];
    let winning_total_amount = round.deployed[winning_square];
    if winning_pool_amount > 0 && winning_total_amount > 0 {
        // Pool's share of SOL winnings: (pool's winning stake / total winning stake) * total winnings
        round.pool_rewards_sol = (winnings as u128 * winning_pool_amount as u128 / winning_total_amount as u128) as u64;
    }
    
    // Add buyback amount to treasury.balance (for buybacks)
    treasury.balance += buyback_amount;
    
    // Distribute 2% of winnings to stakers as SOL rewards.
    if pool.total_staked_score > 0 && staking_sol_amount > 0 {
        pool.stake_rewards_factor +=
            Numeric::from_fraction(staking_sol_amount, pool.total_staked_score);
    }
    pool.balance += staking_sol_amount;
    // Sanity check.
    assert!(
        round.total_deployed
            >= round.total_vaulted
                + round.total_winnings
                + round.deployed[winning_square]
                + winnings_admin_fee
    );

    let mint_supply = mint.supply();
    let remaining_supply = MAX_SUPPLY.saturating_sub(mint_supply);
    
    // Emission per round (capped by remaining supply)
    let mint_amount = remaining_supply.min(EMISSION_PER_ROUND * ONE_OIL);
    
    sol_log(&format!(
        "Emission: per_round={} OIL, total_mint={} OIL",
        EMISSION_PER_ROUND,
        mint_amount / ONE_OIL
    ));

    // Reward OIL for the winning miner(s).
    round.top_miner_reward = mint_amount;
    
    let pool_won_lottery = if winning_pool_amount > 0 && winning_total_amount > 0 && !round.is_split_reward(r) {
        let top_miner_sample = round.top_miner_sample(r, winning_square);
        let pool_cumulative = round.pool_cumulative[winning_square];
        let pool_range_end = pool_cumulative + winning_pool_amount;
        // Pool wins if sample falls in pool's range
        top_miner_sample >= pool_cumulative && top_miner_sample < pool_range_end
    } else {
        false
    };
    
    // Set pool OIL rewards based on lottery result (GODL-style)
    if pool_won_lottery {
        // Pool won the lottery: pool gets 100% OIL
        round.pool_rewards_oil = mint_amount;
        round.top_miner = POOL_ADDRESS; // Set top_miner to POOL_ADDRESS so indexer knows mining pool won
        sol_log(&format!("Pool won lottery: pool_rewards_oil={} OIL, pool_winning_stake={} SOL, total_deployed_to_winning={} SOL", 
            mint_amount / ONE_OIL, 
            lamports_to_sol(winning_pool_amount),
            lamports_to_sol(winning_total_amount)));
    } else if winning_pool_amount > 0 && winning_total_amount > 0 {
        // Pool didn't win but covered winning square: pool gets 0% OIL (will get SOL refund only in checkpoint)
        round.pool_rewards_oil = 0;
        sol_log(&format!("Pool did not win lottery: pool_winning_stake={} SOL, total_deployed_to_winning={} SOL", 
            lamports_to_sol(winning_pool_amount),
            lamports_to_sol(winning_total_amount)));
    } else {
        // Pool didn't cover winning square: no rewards
        round.pool_rewards_oil = 0;
    }

    // With 1 in 2 odds, split the +1 OIL reward.
    if round.is_split_reward(r) {
        round.top_miner = SPLIT_ADDRESS;
        // In split rounds, pool gets proportional share (same as before)
        if winning_pool_amount > 0 && winning_total_amount > 0 {
            round.pool_rewards_oil = (mint_amount as u128 * winning_pool_amount as u128 / winning_total_amount as u128) as u64;
        }
    }

    // Payout the gusher if it was activated.
    let hit_sol_only_gusher = round.did_hit_gusher_sol_only(r);
    
    if hit_sol_only_gusher {
        // Rolling gusher: distribute 90%, keep 10% rolling over
        let payout = treasury.gusher_sol * 90 / 100; // 90% payout
        round.gusher_sol = payout;
        treasury.gusher_sol = treasury.gusher_sol - payout; // Keep 10% rolling
    }

    // Add 1% to gusher_sol pool (after gusher check, so current round's contribution goes to next round if gusher was hit).
    treasury.gusher_sol += gusher_sol_amount;

    // Mint OIL to the treasury.
    let [mint_authority_info, mint_program] = mint_accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    mint_authority_info.as_account::<oil_mint_api::state::Authority>(&oil_mint_api::ID)?;
    mint_program.is_program(&oil_mint_api::ID)?;
    
    invoke_signed(
        &oil_mint_api::sdk::mint_oil(mint_amount),
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

    // Validate top miner (dry-run - no errors on failure).
    if round.top_miner == SPLIT_ADDRESS {
        sol_log("Split round");
    } else if pool_won_lottery {
        sol_log(&format!("Pool won lottery: pool_rewards_oil={} OIL (100% of rewards)", mint_amount / ONE_OIL));
    } else {
        // Try to parse and validate the top miner account (following ORE's pattern)
        if let Ok(miner) = top_miner_info.as_account::<Miner>(&oil_api::ID) {
            if miner.round_id == round.id {
                let top_miner_sample = round.top_miner_sample(r, winning_square);
                if top_miner_sample >= miner.cumulative[winning_square]
                    && top_miner_sample
                        < miner.cumulative[winning_square] + miner.deployed[winning_square]
                {
                    sol_log("Top miner verified");
                } else {
                    sol_log("Top miner verification failed");
                }
            } else {
                sol_log("Top miner round id mismatch");
            }
        } else {
            sol_log("Top miner account cannot be parsed");
        }
    }

    // Emit event.
    program_log(
        &[board_info.clone(), oil_program.clone()],
        ResetEvent {
            disc: 0,
            round_id: round.id,
            start_slot: board.start_slot,
            end_slot: board.end_slot,
            winning_square: winning_square as u64,
            top_miner: round.top_miner,
            num_winners: round.count[winning_square],
            total_deployed: round.total_deployed,
            total_vaulted: round.total_vaulted,
            total_winnings: round.total_winnings,
            total_minted: mint_amount,
            ts: clock.unix_timestamp,
            gusher_sol: round.gusher_sol,
        }
        .to_bytes(),
    )?;

    // Reset board.
    board.round_id += 1;
    board.start_slot = clock.slot + 1;
    board.end_slot = u64::MAX; // board.start_slot + 150;

    // Do SOL transfers.
    round_info.send(total_admin_fee, &fee_collector_info);
    round_info.send(buyback_amount + gusher_sol_amount, &treasury_info); // 8% + 1% to treasury
    round_info.send(staking_sol_amount, &pool_info); // 2% to pool

    Ok(())
}