pub mod consts;
pub mod error;
pub mod event;
pub mod fogo;
pub mod instruction;
pub mod sdk;
pub mod state;
pub mod utils;

pub mod prelude {
    pub use crate::consts::*;
    pub use crate::error::*;
    pub use crate::event::*;
    pub use crate::instruction::*;
    pub use crate::sdk::*;
    // Export state types explicitly to avoid ambiguous re-export warning
    pub use crate::state::{
        Auction, Automation, AutomationStrategy, Board, Config, Micro, Miner, OilAccount, Pool, Referral, Rig, Round, Share, Stake, Treasury, Well,
        Whitelist,
    };
    // Re-export state module functions (PDAs, etc.)
    pub use crate::state::{
        auction_pda, automation_pda, board_pda, config_pda, micro_pda, miner_pda, pool_pda, pool_tokens_address,
        referral_pda, rig_pda, round_pda, share_pda, stake_pda, stake_pda_with_id, treasury_pda, treasury_tokens_address, well_pda,
        whitelist_pda,
    };
    // Re-export utils
    pub use crate::utils::*;
}

use steel::*;

declare_id!("rigwXYKkE8rXiiyu6eFs3ZuDNH2eYHb1y87tYqwDJhk");
