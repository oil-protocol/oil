use serde::{Deserialize, Serialize};
use steel::*;

use crate::state::{miner_pda, Treasury};

use super::OilAccount;


#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Miner {
    /// The authority of this miner account.
    pub authority: Pubkey,

    /// The miner's prospects in the current round.
    pub deployed: [u64; 25],

    /// The cumulative amount of SOL deployed on each square prior to this miner's move.
    pub cumulative: [u64; 25],

    /// SOL witheld in reserve to pay for checkpointing.
    pub checkpoint_fee: u64,

    /// The last round that this miner checkpointed.
    pub checkpoint_id: u64,

    /// The last time this miner claimed OIL rewards.
    pub last_claim_block_oil_at: i64,

    /// The last time this miner claimed SOL rewards.
    pub last_claim_block_sol_at: i64,

    /// The rewards factor last time rewards were updated on this miner account.
    pub block_rewards_factor: Numeric,

    /// The amount of SOL this miner can claim.
    pub block_rewards_sol: u64,

    /// The amount of OIL this miner can claim.
    pub block_rewards_oil: u64,

    /// The amount of OIL this miner has earned from claim fees.
    pub block_refined_oil: u64,

    /// The ID of the round this miner last played in.
    pub round_id: u64,

    /// The pooled deployed amount of this miner.
    pub pooled_deployed: u64,

    /// OIL rewards from auction wells (not yet claimed)
    pub auction_rewards_oil: u64,

    /// SOL rewards from auction wells (not yet claimed)
    pub auction_rewards_sol: u64,

    /// The rewards factor last time auction rewards were updated on this rig account.
    pub auction_rewards_factor: Numeric,

    /// The amount of OIL this rig has earned from auction claim fees (refined OIL).
    pub auction_refined_oil: u64,

    /// The last time this rig claimed OIL rewards.
    pub last_claim_auction_oil_at: i64,

    /// The last time this rig claimed SOL rewards.
    pub last_claim_auction_sol_at: i64,

    /// The total amount of SOL this miner has mined across all blocks.
    pub lifetime_rewards_sol: u64,

    /// The total amount of OIL this miner has mined across all blocks.
    pub lifetime_rewards_oil: u64,

    /// The total amount of OIL this miner has deployed across all rounds.
    pub lifetime_deployed: u64,

    pub lifetime_bid: u64,

    /// The pubkey of the referrer who referred this miner.
    pub referrer: Pubkey,

    /// Total stake score across all stake accounts for this miner.
    pub total_stake_score: u64,

    pub is_seeker: u64,

    pub buffer_a: u64,
}

impl Miner {
    pub fn pda(&self) -> (Pubkey, u8) {
        miner_pda(self.authority)
    }

    pub fn initialize(&mut self, authority: Pubkey) {
        self.authority = authority;
        self.deployed = [0; 25];
        self.cumulative = [0; 25];
        self.checkpoint_fee = 0;
        self.checkpoint_id = 0;
        self.last_claim_block_oil_at = 0;
        self.last_claim_block_sol_at = 0;
        self.block_rewards_factor = Numeric::ZERO;
        self.block_rewards_sol = 0;
        self.block_rewards_oil = 0;
        self.block_refined_oil = 0;
        self.round_id = 0;
        self.pooled_deployed = 0;
        self.is_seeker = 0;
        self.buffer_a = 0;
        self.auction_rewards_oil = 0;
        self.auction_rewards_sol = 0;
        self.auction_rewards_factor = Numeric::ZERO;
        self.auction_refined_oil = 0;
        self.last_claim_auction_oil_at = 0;
        self.last_claim_auction_sol_at = 0;
        self.lifetime_rewards_sol = 0;
        self.lifetime_rewards_oil = 0;
        self.lifetime_deployed = 0;
        self.lifetime_bid = 0;
        self.referrer = Pubkey::default();
        self.total_stake_score = 0;
    }

    pub fn claim_oil(&mut self, clock: &Clock, treasury: &mut Treasury) -> u64 {
        self.update_rewards(treasury);
        let refined_oil = self.block_refined_oil;
        let rewards_oil = self.block_rewards_oil;
        let mut amount = refined_oil + rewards_oil;
        self.block_refined_oil = 0;
        self.block_rewards_oil = 0;

        // Charge a 10% fee and share with miners who haven't claimed yet.
        // Check block_total_unclaimed BEFORE subtracting this miner's rewards_oil
        // to ensure fee is charged even if this is the only miner with unclaimed oil.
        if treasury.block_total_unclaimed > 0 {
            let fee = rewards_oil / 10;
            amount -= fee;
            treasury.block_rewards_factor += Numeric::from_fraction(fee, treasury.block_total_unclaimed);
            treasury.block_total_refined += fee;
            self.lifetime_rewards_oil -= fee;
        }
        
        treasury.block_total_unclaimed -= rewards_oil;
        treasury.block_total_refined -= refined_oil;
        self.last_claim_block_oil_at = clock.unix_timestamp;

        amount
    }

    pub fn claim_sol(&mut self, clock: &Clock) -> u64 {
        let amount = self.block_rewards_sol;
        self.block_rewards_sol = 0;
        self.last_claim_block_sol_at = clock.unix_timestamp;
        amount
    }

    pub fn update_rewards(&mut self, treasury: &Treasury) {
        // Accumulate rewards, weighted by stake balance.
        if treasury.block_rewards_factor > self.block_rewards_factor {
            let accumulated_rewards = treasury.block_rewards_factor - self.block_rewards_factor;
            if accumulated_rewards < Numeric::ZERO {
                panic!("Accumulated rewards is negative");
            }
            let personal_rewards = accumulated_rewards * Numeric::from_u64(self.block_rewards_oil);
            self.block_refined_oil += personal_rewards.to_u64();
            self.lifetime_rewards_oil += personal_rewards.to_u64();
        }

        // Update this miner account's last seen rewards factor.
        self.block_rewards_factor = treasury.block_rewards_factor;
    }

    pub fn update_auction_rewards(&mut self, treasury: &Treasury) {
        // Accumulate auction rewards, weighted by unclaimed auction OIL.
        if treasury.auction_rewards_factor > self.auction_rewards_factor {
            let accumulated_rewards = treasury.auction_rewards_factor - self.auction_rewards_factor;
            if accumulated_rewards < Numeric::ZERO {
                panic!("Accumulated auction rewards is negative");
            }
            let personal_rewards = accumulated_rewards * Numeric::from_u64(self.auction_rewards_oil);
            self.auction_refined_oil += personal_rewards.to_u64();
        }

        // Update this miner account's last seen auction rewards factor.
        self.auction_rewards_factor = treasury.auction_rewards_factor;
    }
}

account!(OilAccount, Miner);