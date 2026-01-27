use serde::{Deserialize, Serialize};
use steel::*;

pub enum OilEvent {
    Reset = 0,
    Barrel = 1,
    Deploy = 2,
    Liq = 3,
    Bid = 4,
    JoinPool = 5,
    ClaimAuctionOIL = 6,
    ClaimAuctionSOL = 7,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct ResetEvent {
    /// The event discriminator.
    pub disc: u64,

    /// The block that was opened for trading.
    pub round_id: u64,

    /// The start slot of the next block.
    pub start_slot: u64,

    /// The end slot of the next block.
    pub end_slot: u64,

    /// The winning square of the round.
    pub winning_square: u64,

    /// The top miner of the round.
    pub top_miner: Pubkey,

    /// The number of miners on the winning square.
    pub num_winners: u64,

    /// The total amount of SOL prospected in the round.
    pub total_deployed: u64,

    /// The total amount of SOL put in the OIL vault.
    pub total_vaulted: u64,

    /// The total amount of SOL won by miners for the round.
    pub total_winnings: u64,

    /// The total amount of OIL minted for the round.
    pub total_minted: u64,

    /// The timestamp of the event.
    pub ts: i64,

    /// The amount of SOL in the gusher.
    pub gusher_sol: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct BarrelEvent {
    /// The event discriminator.
    pub disc: u64,

    /// The amount of OIL barreled.
    pub oil_barreled: u64,

    /// The amount of OIL shared with stakers.
    pub oil_shared: u64,

    /// The amount of SOL swapped.
    pub sol_amount: u64,

    /// The new circulating supply of OIL.
    pub new_circulating_supply: u64,

    /// The timestamp of the event.
    pub ts: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct DeployEvent {
    /// The event discriminator.
    pub disc: u64,

    /// The authority of the deployer.
    pub authority: Pubkey,

    /// The amount of SOL deployed per square.
    pub amount: u64,

    /// The mask of the squares deployed to.
    pub mask: u64,

    /// The round id.
    pub round_id: u64,

    /// The signer of the deployer.
    pub signer: Pubkey,

    /// The strategy used by the autominer (u64::MAX if manual).
    pub strategy: u64,

    /// The total number of squares deployed to.
    pub total_squares: u64,

    /// The timestamp of the event.
    pub ts: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct LiqEvent {
    /// The event discriminator.
    pub disc: u64,

    /// The amount of SOL sent to the liq manager.
    pub sol_amount: u64,

    /// The recipient of the SOL.
    pub recipient: Pubkey,

    /// The timestamp of the event.
    pub ts: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct BidEvent {
    /// The event discriminator.
    pub disc: u64,
    
    /// The authority of the bidder.
    pub authority: Pubkey,
    
    /// The square ID (well) that was bid on (0-3).
    pub square_id: u64,
    
    /// The bid amount in lamports.
    pub bid_amount: u64,
    
    /// The current price at time of bid (in lamports).
    pub current_price: u64,
    
    /// The previous owner (Pubkey::default() if no previous owner).
    pub previous_owner: Pubkey,
    
    /// The accumulated OIL transferred to previous owner (0 if no previous owner).
    pub accumulated_oil_transferred: u64,
    
    /// The new starting price for next auction (in lamports).
    pub new_start_price: u64,
    
    /// The timestamp of the event.
    pub ts: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct JoinAuctionPoolEvent {
    /// The event discriminator.
    pub disc: u64,
    
    /// The authority of the contributor.
    pub authority: Pubkey,
    
    /// The square ID (well) for this pool contribution (0-3).
    pub square_id: u64,
    
    /// The contribution amount in lamports.
    pub contribution: u64,
    
    /// The total pool amount after this contribution (in lamports).
    pub pool_total: u64,
    
    /// The current price at time of contribution (in lamports).
    pub current_price: u64,
    
    /// The timestamp of the event.
    pub ts: u64,
    
    /// Whether the pool won the well (auto-bid triggered).
    /// 0 = false, 1 = true (using u64 for Pod compatibility and alignment)
    pub pool_won: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct ClaimAuctionOILEvent {
    /// The event discriminator.
    pub disc: u64,
    
    /// The authority of the claimant.
    pub authority: Pubkey,
    
    /// The total OIL claimed (after refining fees).
    pub oil_claimed: u64,
    
    /// The refining fee charged (10%).
    pub refining_fee: u64,
    
    /// The timestamp of the event.
    pub ts: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable, Serialize, Deserialize)]
pub struct ClaimAuctionSOLEvent {
    /// The event discriminator.
    pub disc: u64,
    
    /// The authority of the claimant.
    pub authority: Pubkey,
    
    /// The total SOL claimed (rewards + refunds).
    pub sol_claimed: u64,
    
    /// The SOL from auction rewards (being outbid).
    pub rewards_sol: u64,
    
    /// The SOL from pool refunds.
    pub refunds_sol: u64,
    
    /// The timestamp of the event.
    pub ts: u64,
}

event!(ResetEvent);
event!(BarrelEvent);
event!(DeployEvent);
event!(LiqEvent);
event!(BidEvent);
event!(JoinAuctionPoolEvent);
event!(ClaimAuctionOILEvent);
event!(ClaimAuctionSOLEvent);