use serde::{Deserialize, Serialize};
use steel::*;

use crate::state::{stake_pda, Pool};

use super::OilAccount;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Stake {
    /// The authority of this miner account.
    pub authority: Pubkey,

    /// The balance of this stake account.
    pub balance: u64,

    /// Lock duration in days (0 = no lock, 1-730 days)
    pub lock_duration_days: u64,

    /// Unix timestamp when lock expires (0 = no lock)
    pub lock_ends_at: u64,

    /// Buffer c (placeholder)
    pub buffer_c: u64,

    /// Buffer d (placeholder)
    pub buffer_d: u64,

    /// Buffer e (placeholder)
    pub buffer_e: u64,

    /// The timestamp of last claim.
    pub last_claim_at: i64,

    /// The timestamp the last time this staker deposited.
    pub last_deposit_at: i64,

    /// The timestamp the last time this staker withdrew.
    pub last_withdraw_at: i64,

    /// The rewards factor last time rewards were updated on this stake account.
    pub rewards_factor: Numeric,

    /// The amount of SOL this staker can claim.
    pub rewards: u64,

    /// The total amount of SOL this staker has earned over its lifetime.
    pub lifetime_rewards: u64,

    /// Buffer f (placeholder)
    pub buffer_f: u64,
}

impl Stake {
    pub fn pda(&self) -> (Pubkey, u8) {
        stake_pda(self.authority)
    }

    pub fn calculate_multiplier(lock_duration_days: u64) -> f64 {
        if lock_duration_days == 0 {
            return 1.0;
        }

        let lookup: [(u64, f64); 6] = [
            (7, 1.18),
            (30, 1.78),
            (90, 3.35),
            (180, 5.69),
            (365, 10.5),
            (730, 20.0),
        ];

        // Cap at 730 days
        let days = lock_duration_days.min(730);

        // Exact match
        for &(d, m) in &lookup {
            if days == d {
                return m;
            }
        }

        // Linear interpolation between lookup points
        for i in 0..lookup.len() - 1 {
            let (d1, m1) = lookup[i];
            let (d2, m2) = lookup[i + 1];

            if days >= d1 && days <= d2 {
                let ratio = (days - d1) as f64 / (d2 - d1) as f64;
                return m1 + (m2 - m1) * ratio;
            }
        }

        // Extrapolate beyond 730 days (shouldn't happen, but cap at 20.0)
        if days > 730 {
            return 20.0;
        }

        // Below 7 days: linear from 1.0 to 1.18
        if days < 7 {
            return 1.0 + (days as f64 / 7.0) * 0.18;
        }

        1.0 // Fallback
    }

    pub fn score(&self) -> u64 {
        let multiplier = Self::calculate_multiplier(self.lock_duration_days);
        // Use fixed-point arithmetic: multiply by 1_000_000 for precision, then divide
        ((self.balance as u128 * (multiplier * 1_000_000.0) as u128) / 1_000_000) as u64
    }

    pub fn is_locked(&self, clock: &Clock) -> bool {
        self.lock_ends_at > 0 && (clock.unix_timestamp as u64) < self.lock_ends_at
    }

    pub fn remaining_lock_seconds(&self, clock: &Clock) -> u64 {
        if self.lock_ends_at == 0 || (clock.unix_timestamp as u64) >= self.lock_ends_at {
            return 0;
        }
        self.lock_ends_at - (clock.unix_timestamp as u64)
    }

    pub fn calculate_penalty_percent(lock_duration_days: u64) -> u64 {
        match lock_duration_days {
            0 => 0,                    // No lock = no penalty
            1..=7 => 5,                // 1-7 days = 5%
            8..=30 => 10,              // 8-30 days = 10%
            31..=180 => 25,            // 31-180 days = 25%
            181..=365 => 40,           // 181-365 days = 40%
            366..=730 => 60,           // 366-730 days = 60% (capped)
            _ => 60,                   // 730+ days = 60% (capped)
        }
    }

    pub fn claim(&mut self, amount: u64, clock: &Clock, pool: &Pool) -> u64 {
        self.update_rewards(pool);
        let amount = self.rewards.min(amount);
        self.rewards -= amount;
        self.last_claim_at = clock.unix_timestamp;
        amount
    }

    pub fn deposit(
        &mut self,
        amount: u64,
        clock: &Clock,
        pool: &mut Pool,
        sender: &TokenAccount,
    ) -> u64 {
        self.update_rewards(pool);
        
        // Calculate old score before deposit
        let old_score = self.score();
        
        let amount = sender.amount().min(amount);
        self.balance += amount;
        self.last_deposit_at = clock.unix_timestamp;
        
        // Calculate new score after deposit
        let new_score = self.score();
        
        // Update pool totals
        pool.total_staked += amount;
        pool.total_staked_score = pool.total_staked_score.saturating_add(new_score).saturating_sub(old_score);
        
        amount
    }

    pub fn withdraw(&mut self, amount: u64, clock: &Clock, pool: &mut Pool) -> u64 {
        self.update_rewards(pool);
        
        // Calculate old score before withdraw
        let old_score = self.score();
        
        let amount = self.balance.min(amount);
        self.balance -= amount;
        self.last_withdraw_at = clock.unix_timestamp;
        
        // If balance reaches 0, reset the lock so user can set a new lock when depositing again
        // This allows users to start fresh after withdrawing all funds (with or without penalty)
        if self.balance == 0 {
            self.lock_duration_days = 0;
            self.lock_ends_at = 0;
        }
        
        // Calculate new score after withdraw
        let new_score = self.score();
        
        // Update pool totals
        pool.total_staked -= amount;
        pool.total_staked_score = pool.total_staked_score.saturating_add(new_score).saturating_sub(old_score);
        
        amount
    }

    pub fn update_rewards(&mut self, pool: &Pool) {
        // Accumulate SOL rewards, weighted by stake score (balance * multiplier).
        if pool.stake_rewards_factor > self.rewards_factor {
            let accumulated_rewards = pool.stake_rewards_factor - self.rewards_factor;
            if accumulated_rewards < Numeric::ZERO {
                panic!("Accumulated rewards is negative");
            }
            // Use score instead of balance for lock-based weighted staking
            let score = self.score();
            let personal_rewards = accumulated_rewards * Numeric::from_u64(score);
            self.rewards += personal_rewards.to_u64();
            self.lifetime_rewards += personal_rewards.to_u64();
        }

        // Update this stake account's last seen rewards factor.
        self.rewards_factor = pool.stake_rewards_factor;
    }
}

account!(OilAccount, Stake);