use serde::{Deserialize, Serialize};
use steel::*;

use crate::state::round_pda;

use super::OilAccount;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize, Default)]
pub struct Round {
    /// The round number.
    pub id: u64,

    /// The amount of SOL deployed in each square.
    pub deployed: [u64; 25],

    /// The hash of the end slot, provided by solana, used for random number generation.
    pub slot_hash: [u8; 32],

    /// The count of miners on each square.
    pub count: [u64; 25],

    /// The slot at which claims for this round account end.
    pub expires_at: u64,

    /// The account to which rent should be returned when this account is closed.
    pub rent_payer: Pubkey,

    /// The top miner of the round.
    pub top_miner: Pubkey,

    /// The amount of OIL to distribute to the top miner.
    pub top_miner_reward: u64,

    /// The total amount of SOL deployed in the round.
    pub total_deployed: u64,

    /// The total number of unique miners that played in the round.
    pub total_miners: u64,

    /// The total amount of SOL put in the OIL vault.
    pub total_vaulted: u64,

    /// The total amount of SOL won by miners for the round.
    pub total_winnings: u64,

    /// The amount of SOL in the gusher.
    pub gusher_sol: u64,

    /// The amount of pooled SOL deployed in each square.
    pub deployed_pooled: [u64; 25],

    /// The total amount of pooled SOL deployed in the round.
    pub total_pooled: u64,

    /// The SOL rewards allocated to the pool for this round.
    pub pool_rewards_sol: u64,

    /// The OIL rewards allocated to the pool for this round.
    pub pool_rewards_oil: u64,

    /// The number of unique miners who deployed as pooled this round.
    pub pool_members: u64,

    /// The cumulative SOL deployed by solo miners before pool deployed to each square.
    pub pool_cumulative: [u64; 25],
}

impl Round {
    pub fn pda(&self) -> (Pubkey, u8) {
        round_pda(self.id)
    }

    pub fn rng(&self) -> Option<u64> {
        if self.slot_hash == [0; 32] || self.slot_hash == [u8::MAX; 32] {
            return None;
        }
        let r1 = u64::from_le_bytes(self.slot_hash[0..8].try_into().unwrap());
        let r2 = u64::from_le_bytes(self.slot_hash[8..16].try_into().unwrap());
        let r3 = u64::from_le_bytes(self.slot_hash[16..24].try_into().unwrap());
        let r4 = u64::from_le_bytes(self.slot_hash[24..32].try_into().unwrap());
        let r = r1 ^ r2 ^ r3 ^ r4;
        Some(r)
    }

    pub fn winning_square(&self, rng: u64) -> usize {
        (rng % 25) as usize
    }

    pub fn top_miner_sample(&self, rng: u64, winning_square: usize) -> u64 {
        if self.deployed[winning_square] == 0 {
            return 0;
        }
        rng.reverse_bits() % self.deployed[winning_square]
    }

    pub fn calculate_total_winnings(&self, winning_square: usize) -> u64 {
        let mut total_winnings = 0;
        for (i, &deployed) in self.deployed.iter().enumerate() {
            if i != winning_square {
                total_winnings += deployed;
            }
        }
        total_winnings
    }

    pub fn is_split_reward(&self, rng: u64) -> bool {
        // One out of four rounds get split rewards.
        let rng = rng.reverse_bits().to_le_bytes();
        let r1 = u16::from_le_bytes(rng[0..2].try_into().unwrap());
        let r2 = u16::from_le_bytes(rng[2..4].try_into().unwrap());
        let r3 = u16::from_le_bytes(rng[4..6].try_into().unwrap());
        let r4 = u16::from_le_bytes(rng[6..8].try_into().unwrap());
        let r = r1 ^ r2 ^ r3 ^ r4;
        r % 2 == 0
    }

    pub fn did_hit_gusher_sol_only(&self, rng: u64) -> bool {
        rng.reverse_bits() % 625 == 0
    }
}

account!(OilAccount, Round);

#[cfg(test)]
mod tests {
    use solana_program::rent::Rent;

    use super::*;

    #[test]
    fn test_rent() {
        let size_of_round = 8 + std::mem::size_of::<Round>();
        let required_rent = Rent::default().minimum_balance(size_of_round);
        println!("Round account size: {} bytes", size_of_round);
        println!("Required rent: {} lamports ({} SOL)", required_rent, required_rent as f64 / 1_000_000_000.0);
        
        // Verify the account size matches expected size (1008 bytes for v4 with pool_cumulative field)
        // 8 bytes discriminator + 1000 bytes struct = 1008 bytes total
        assert_eq!(size_of_round, 1008, "Round account size should be 1008 bytes (v4 with pool_cumulative, no padding)");
        
        // Verify rent is reasonable (should be > 0 and < 10 SOL for a 816 byte account)
        assert!(required_rent > 0, "Required rent should be greater than 0");
        assert!(required_rent < 10_000_000_000, "Required rent should be less than 10 SOL");
    }
}