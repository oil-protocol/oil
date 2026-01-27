use serde::{Deserialize, Serialize};
use steel::*;

use crate::state::config_pda;

use super::OilAccount;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct Config {
    /// The address that can update the config.
    pub admin: Pubkey,

    /// The adress with authority to call wrap and barrel.
    pub barrel_authority: Pubkey,

    /// The address that receives admin fees.
    pub fee_collector: Pubkey,

    /// The program to be used for protocol swaps.
    pub swap_program: Pubkey,

    /// The address of the entropy var account.
    pub var_address: Pubkey,

    /// Amount to pay to fee collector (bps)
    pub admin_fee: u64,

    /// Current emission week (used for automatic weekly halving)
    pub emission_week: u64,

    /// Timestamp when the last emission week was updated (for automatic weekly progression)
    pub last_emission_week_update: u64,

    /// Timestamp for Token Generation Event (TGE). If current time < tge_timestamp, pre-mine is active.
    /// Set to 0 to disable pre-mine.
    pub tge_timestamp: i64,
}

impl Config {
    pub fn pda() -> (Pubkey, u8) {
        config_pda()
    }
}

account!(OilAccount, Config);
