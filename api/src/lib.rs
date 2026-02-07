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
    // Note: state::Bid and instruction::Bid both exist - use explicit imports where needed
    // Export state types explicitly to avoid ambiguous re-export warning
    pub use crate::state::{
        Auction, Automation, AutomationStrategy, Bid as BidAccount, Board, Config, Miner, OilAccount, Pool, Referral, Round, Stake, Treasury, Well,
    };
    // Re-export state module functions (PDAs, etc.)
    pub use crate::state::{
        auction_pda, automation_pda, bid_pda, board_pda, config_pda, miner_pda, pool_pda, pool_tokens_address,
        referral_pda, round_pda, stake_pda, stake_pda_with_id, treasury_pda, treasury_tokens_address, well_pda,
    };
    // Re-export utils
    pub use crate::utils::*;
}

use steel::*;

declare_id!("rigwXYKkE8rXiiyu6eFs3ZuDNH2eYHb1y87tYqwDJhk");
