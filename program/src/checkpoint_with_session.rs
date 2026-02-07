use oil_api::prelude::*;
use oil_api::fogo;
use solana_program::{log::sol_log, native_token::lamports_to_sol, rent::Rent};
use spl_token::amount_to_ui_amount;
use steel::*;

/// Checkpoints a miner's rewards (FOGO session).
pub fn process_checkpoint_with_session<'a>(accounts: &'a [AccountInfo<'a>], _data: &[u8]) -> ProgramResult {
    let clock = Clock::get()?;
    let [signer_info, _authority_info, program_signer_info, board_info, config_info, miner_info, round_info, treasury_info, system_program] = accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    
    fogo::validate_session(signer_info)?;
    fogo::validate_program_signer(program_signer_info)?;
    
    // Allow anyone to checkpoint (like ORE) - signer can collect bot fee, rewards go to miner
    let board = board_info.as_account::<Board>(&oil_api::ID)?;
    let config = config_info.as_account::<Config>(&oil_api::ID)?;
    let miner = miner_info.as_account_mut::<Miner>(&oil_api::ID)?;
    
    let is_premine = config.tge_timestamp > 0 && clock.unix_timestamp < config.tge_timestamp;
    let treasury = treasury_info.as_account_mut::<Treasury>(&oil_api::ID)?;
    system_program.is_program(&system_program::ID)?;

    if miner.checkpoint_id == miner.round_id {
        return Ok(());
    }

    if round_info.data_is_empty() {
        sol_log(&format!("Round account is empty").as_str());
        round_info.has_seeds(&[ROUND, &miner.round_id.to_le_bytes()], &oil_api::ID)?;
        miner.checkpoint_id = miner.round_id;
        return Ok(());
    }

    let round = round_info.as_account_mut::<Round>(&oil_api::ID)?;
    sol_log(&format!("Round ID: {}", round.id).as_str());
    if round.id == board.round_id || round.id != miner.round_id || round.slot_hash == [0; 32] {
        sol_log(&format!("Round not valid").as_str());
        return Ok(());
    }

    if clock.slot >= round.expires_at {
        sol_log(&format!("Round expired").as_str());
        miner.checkpoint_id = miner.round_id;
        return Ok(());
    }

    let mut bot_fee = 0;
    if clock.slot >= round.expires_at - TWELVE_HOURS_SLOTS {
        bot_fee = miner.checkpoint_fee;
        miner.checkpoint_fee = 0;
    }

    let mut rewards_sol = 0;
    let mut rewards_oil = 0;
    let mut gusher_sol_portion = 0;

    if let Some(r) = round.rng() {
        let winning_square = round.winning_square(r) as usize;

        let is_pool_member = miner.pooled_deployed > 0;

        if miner.deployed[winning_square] > 0 && !is_pool_member {
            assert!(
                round.deployed[winning_square] >= miner.deployed[winning_square],
                "Invalid round deployed amount"
            );

            let original_deployment = miner.deployed[winning_square];
            let admin_fee_rate = if is_premine { 2 } else { 1 };
            let admin_fee = if original_deployment > 0 {
                (original_deployment * admin_fee_rate / 100).max(1)
            } else {
                0
            };
            rewards_sol = original_deployment.saturating_sub(admin_fee);
            rewards_sol += ((round.total_winnings as u128 * miner.deployed[winning_square] as u128)
                / round.deployed[winning_square] as u128) as u64;
            sol_log(&format!("Base rewards: {} SOL", lamports_to_sol(rewards_sol)).as_str());

            if round.top_miner == SPLIT_ADDRESS {
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

            if round.gusher_sol > 0 {
                let solo_gusher_sol =
                    ((round.gusher_sol as u128 * miner.deployed[winning_square] as u128)
                        / round.deployed[winning_square] as u128) as u64;
                sol_log(
                    &format!("Gusher SOL rewards: {} SOL", lamports_to_sol(solo_gusher_sol))
                        .as_str(),
                );
                rewards_sol += solo_gusher_sol;
                gusher_sol_portion = solo_gusher_sol;
            }
        }
        
        if is_pool_member && round.total_pooled > 0 {
            let pool_winning_stake = round.deployed_pooled[winning_square];
            
            if pool_winning_stake > 0 {
                let miner_pool_share = miner.pooled_deployed as u128;
                let total_pool = round.total_pooled as u128;
                
                let pool_refund = pool_winning_stake as u128;
                let miner_refund = (pool_refund * miner_pool_share / total_pool) as u64;
                let admin_fee_rate = if is_premine { 2 } else { 1 };
                let admin_fee = if miner_refund > 0 {
                    (miner_refund * admin_fee_rate / 100).max(1)
                } else {
                    0
                };
                let deployment_refund = miner_refund.saturating_sub(admin_fee);
                
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
                    rewards_sol += miner_gusher_sol;
                    gusher_sol_portion = miner_gusher_sol;
                }
            } else {
                sol_log("Pool did not deploy to winning square - no refund");
            }
        }
    } else {
        assert!(
            round.total_deployed == 0,
            "Round total deployed should be zero."
        );

        let refund_amount = miner.deployed.iter().sum::<u64>();
        sol_log(&format!("Refunding {} SOL", lamports_to_sol(refund_amount)).as_str());
        rewards_sol = refund_amount;
    }

    miner.update_rewards(treasury);

    miner.checkpoint_id = round.id;
    miner.block_rewards_oil += rewards_oil;
    miner.lifetime_rewards_oil += rewards_oil;
    miner.block_rewards_sol += rewards_sol;
    miner.lifetime_rewards_sol += rewards_sol;

    treasury.block_total_unclaimed += rewards_oil;

    let regular_rewards_sol = rewards_sol.saturating_sub(gusher_sol_portion);
    if regular_rewards_sol > 0 {
        round_info.send(regular_rewards_sol, &miner_info);
    }
    
    if gusher_sol_portion > 0 {
        treasury_info.send(gusher_sol_portion, &miner_info);
    }
    
    if bot_fee > 0 {
        miner_info.send(bot_fee, &signer_info);
    }

    let account_size = 8 + std::mem::size_of::<Miner>();
    let required_rent = Rent::get()?.minimum_balance(account_size);
    assert!(
        miner_info.lamports() >= required_rent + miner.checkpoint_fee + miner.block_rewards_sol,
        "Miner does not have sufficient funds for rent and rewards"
    );

    Ok(())
}
