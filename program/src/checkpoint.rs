use oil_api::prelude::*;
use solana_program::{log::sol_log, native_token::lamports_to_sol, rent::Rent};
use spl_token::amount_to_ui_amount;
use steel::*;

/// Checkpoints a miner's rewards.
pub fn process_checkpoint(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let clock = Clock::get()?;
    let [signer_info, authority_info, board_info, config_info, miner_info, round_info, treasury_info, system_program] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    
    // Use the authority account (user's wallet public key) for PDA derivation
    let authority = *authority_info.key;
    
    let board = board_info.as_account::<Board>(&oil_api::ID)?;
    let config = config_info.as_account::<Config>(&oil_api::ID)?;
    let miner = miner_info.as_account_mut::<Miner>(&oil_api::ID)?;
    
    // Check if we're in pre-mine phase
    let is_premine = config.tge_timestamp > 0 && clock.unix_timestamp < config.tge_timestamp;
    
    // Validate that miner.authority matches the provided authority
    if miner.authority != authority {
        return Err(ProgramError::InvalidAccountData);
    }
    let treasury = treasury_info.as_account_mut::<Treasury>(&oil_api::ID)?;
    system_program.is_program(&system_program::ID)?;

    // If miner has already checkpointed this round, return.
    if miner.checkpoint_id == miner.round_id {
        return Ok(());
    }

    // If round account is empty, verify the correct account was provided.
    if round_info.data_is_empty() {
        sol_log(&format!("Round account is empty").as_str());
        round_info.has_seeds(&[ROUND, &miner.round_id.to_le_bytes()], &oil_api::ID)?;
        miner.checkpoint_id = miner.round_id;
        return Ok(());
    }

    // If round is current round, or the miner round ID does not match the provided round, return.
    let round = round_info.as_account_mut::<Round>(&oil_api::ID)?; // Round has been closed.
    sol_log(&format!("Round ID: {}", round.id).as_str());
    if round.id == board.round_id || round.id != miner.round_id || round.slot_hash == [0; 32] {
        sol_log(&format!("Round not valid").as_str());
        return Ok(());
    }

    // Ensure round is not expired.
    if clock.slot >= round.expires_at {
        sol_log(&format!("Round expired").as_str());
        miner.checkpoint_id = miner.round_id;
        return Ok(());
    }

    // Calculate bot fee.
    let mut bot_fee = 0;
    if clock.slot >= round.expires_at - TWELVE_HOURS_SLOTS {
        bot_fee = miner.checkpoint_fee;
        miner.checkpoint_fee = 0;
    }

    // Calculate miner rewards.
    let mut rewards_sol = 0;
    let mut rewards_oil = 0;
    let mut gusher_sol_portion = 0; // Track gusher SOL portion separately for treasury transfer

    // Get the RNG.
    if let Some(r) = round.rng() {
        // Get the winning square.
        let winning_square = round.winning_square(r) as usize;

        // Check if miner is a pool member (pooled deployments go through pool reward distribution)
        let is_pool_member = miner.pooled_deployed > 0;

        // If the miner deployed to the winning square AND is not a pool member, calculate solo rewards.
        if miner.deployed[winning_square] > 0 && !is_pool_member {
            // Sanity check.
            assert!(
                round.deployed[winning_square] >= miner.deployed[winning_square],
                "Invalid round deployed amount"
            );

            // Calculate SOL rewards (solo miners only).
            let original_deployment = miner.deployed[winning_square];
            // During premine: 2% admin fee, otherwise 1%
            let admin_fee_rate = if is_premine { 2 } else { 1 };
            let admin_fee = if original_deployment > 0 {
                (original_deployment * admin_fee_rate / 100).max(1)
            } else {
                0 // No admin fee if no deployment (shouldn't happen due to condition above)
            };
            rewards_sol = original_deployment.saturating_sub(admin_fee);
            rewards_sol += ((round.total_winnings as u128 * miner.deployed[winning_square] as u128)
                / round.deployed[winning_square] as u128) as u64;
            sol_log(&format!("Base rewards: {} SOL", lamports_to_sol(rewards_sol)).as_str());

            // Calculate OIL rewards.
            if round.top_miner == SPLIT_ADDRESS {
                // If round is split, split the reward evenly among all miners.
                rewards_oil = ((round.top_miner_reward as u128
                    * miner.deployed[winning_square] as u128)
                    / round.deployed[winning_square] as u128) as u64;
                sol_log(
                    &format!(
                        "Split rewards: {} OIL",
                        amount_to_ui_amount(rewards_oil, TOKEN_DECIMALS)
                    )
                    .as_str(),
                );
            } else {
                // If round is not split, payout to the top miner.
                let top_miner_sample = round.top_miner_sample(r, winning_square);
                if top_miner_sample >= miner.cumulative[winning_square]
                    && top_miner_sample
                        < miner.cumulative[winning_square] + miner.deployed[winning_square]
                {
                    rewards_oil = round.top_miner_reward;
                    round.top_miner = miner.authority;
                    sol_log(
                        &format!(
                            "Top miner rewards: {} OIL",
                            amount_to_ui_amount(rewards_oil, TOKEN_DECIMALS)
                        )
                        .as_str(),
                    );
                }
            }

            // Calculate gusher SOL rewards.
            if round.gusher_sol > 0 {
                let solo_gusher_sol =
                    ((round.gusher_sol as u128 * miner.deployed[winning_square] as u128)
                        / round.deployed[winning_square] as u128) as u64;
                sol_log(
                    &format!("Gusher SOL rewards: {} SOL", lamports_to_sol(solo_gusher_sol))
                        .as_str(),
                );
                // Add to rewards_sol for accounting (miner.rewards_sol will include it)
                rewards_sol += solo_gusher_sol;
                // Track separately for treasury transfer
                gusher_sol_portion = solo_gusher_sol;
            }
        }
        
        // Calculate pool share rewards if miner participated in the pool.
        if is_pool_member && round.total_pooled > 0 {
            let pool_winning_stake = round.deployed_pooled[winning_square];
            
            // Pool only gets rewards if it had stake on the winning square
            if pool_winning_stake > 0 {
                // Calculate this miner's share of pool (their contribution / total pool)
                let miner_pool_share = miner.pooled_deployed as u128;
                let total_pool = round.total_pooled as u128;
                
                // Deployment refund: miner's share of pool's winning stake (minus admin fee)
                let pool_refund = pool_winning_stake as u128;
                let miner_refund = (pool_refund * miner_pool_share / total_pool) as u64;
                // During premine: 2% admin fee, otherwise 1%
                let admin_fee_rate = if is_premine { 2 } else { 1 };
                let admin_fee = if miner_refund > 0 {
                    (miner_refund * admin_fee_rate / 100).max(1)
                } else {
                    0 // No admin fee if no refund
                };
                let deployment_refund = miner_refund.saturating_sub(admin_fee);
                
                // Winnings share: miner's share of pool_rewards_sol
                let pool_sol_share = if round.pool_rewards_sol > 0 {
                    ((round.pool_rewards_sol as u128 * miner_pool_share) / total_pool) as u64
                } else {
                    0
                };
                
                rewards_sol = deployment_refund + pool_sol_share;
                sol_log(
                    &format!(
                        "Base rewards: {} SOL",
                        lamports_to_sol(rewards_sol)
                    )
                    .as_str(),
                );
                // Pool OIL share (from pool_rewards_oil calculated in reset)
                if round.pool_rewards_oil > 0 {
                    let pool_oil_share = ((round.pool_rewards_oil as u128 * miner_pool_share) / total_pool) as u64;
                    sol_log(
                        &format!(
                            "Pool share: {} OIL",
                            amount_to_ui_amount(pool_oil_share, TOKEN_DECIMALS)
                        )
                        .as_str(),
                    );
                    rewards_oil += pool_oil_share;
                }
                
                // Pool gusher rewards (if gusher hit)
                if round.gusher_sol > 0 {
                    let pool_gusher_sol = ((round.gusher_sol as u128 * pool_winning_stake as u128)
                        / round.deployed[winning_square] as u128) as u64;
                    let miner_gusher_sol = ((pool_gusher_sol as u128 * miner_pool_share) / total_pool) as u64;
                    sol_log(
                        &format!(
                            "Pool gusher SOL: {} SOL",
                            lamports_to_sol(miner_gusher_sol)
                        )
                        .as_str(),
                    );
                    // Add to rewards_sol for accounting (miner.rewards_sol will include it)
                    rewards_sol += miner_gusher_sol;
                    // Track separately for treasury transfer
                    gusher_sol_portion = miner_gusher_sol;
                }
            } else {
                // Pool had no stake on winning square - pool members lose their deployment
                sol_log("Pool did not deploy to winning square - no refund");
            }
        }
    } else {
        // Sanity check.
        // If there is no rng, total deployed should have been reset to zero.
        assert!(
            round.total_deployed == 0,
            "Round total deployed should be zero."
        );

        // Round has no slot hash, refund all SOL.
        let refund_amount = miner.deployed.iter().sum::<u64>();
        sol_log(&format!("Refunding {} SOL", lamports_to_sol(refund_amount)).as_str());
        rewards_sol = refund_amount;
    }

    // Checkpoint rewards.
    miner.update_rewards(treasury);

    // Checkpoint miner.
    miner.checkpoint_id = round.id;
    miner.block_rewards_oil += rewards_oil;
    miner.lifetime_rewards_oil += rewards_oil;
    miner.block_rewards_sol += rewards_sol;
    miner.lifetime_rewards_sol += rewards_sol;

    // Update treasury.
    treasury.block_total_unclaimed += rewards_oil;

    // Do SOL transfers.
    // Regular rewards come from round account (like ORE).
    let regular_rewards_sol = rewards_sol.saturating_sub(gusher_sol_portion);
    if regular_rewards_sol > 0 {
        round_info.send(regular_rewards_sol, &miner_info);
    }
    
    // Gusher SOL comes from treasury (not round account, since it's stored in treasury.gusher_sol).
    if gusher_sol_portion > 0 {
        treasury_info.send(gusher_sol_portion, &miner_info);
    }
    
    if bot_fee > 0 {
        miner_info.send(bot_fee, &signer_info);
    }

    // Assert miner account has sufficient funds for rent and rewards.
    let account_size = 8 + std::mem::size_of::<Miner>();
    let required_rent = Rent::get()?.minimum_balance(account_size);
    assert!(
        miner_info.lamports() >= required_rent + miner.checkpoint_fee + miner.block_rewards_sol,
        "Miner does not have sufficient funds for rent and rewards"
    );

    Ok(())
}