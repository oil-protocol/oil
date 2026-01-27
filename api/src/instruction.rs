use steel::*;
use bytemuck;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, TryFromPrimitive)]
pub enum OilInstruction {
    // Miner
    Automate = 0,
    Initialize = 1,
    Checkpoint = 2,
    ClaimSOL = 3,
    ClaimOIL = 4,
    Close = 5,
    Deploy = 6,
    ClaimSeeker = 7,
    Log = 8,
    Reset = 9,
    ReloadSOL = 22,
    CreateReferral = 27,
    ClaimReferral = 28,

    // Auction-based mining
    PlaceBid = 29,
    ClaimAuctionOIL = 31,
    ClaimAuctionSOL = 32,

    // Staker
    Deposit = 10,
    Withdraw = 11,
    ClaimYield = 12,

    // Admin
    Buyback = 13,
    Wrap = 14,
    SetAdmin = 16,
    SetFeeCollector = 17,
    SetSwapProgram = 18,
    SetVarAddress = 19,
    NewVar = 20,
    SetAdminFee = 21,
    Migrate = 26,
    SetAuction = 33,
    CreateWhitelist = 34,
    SetTgeTimestamp = 35,
    Liq = 37,
    Barrel = 38,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Automate {
    pub amount: [u8; 8],
    pub deposit: [u8; 8],
    pub fee: [u8; 8],
    pub mask: [u8; 8],
    pub strategy: u8,
    pub reload: [u8; 8],
    /// Optional referrer pubkey for new miners. Set to Pubkey::default() for no referrer.
    pub referrer: [u8; 32],
    /// Whether automated deployments should be pooled (1 = pooled, 0 = not pooled).
    pub pooled: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct InitRound {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ClaimSOL {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ClaimOIL {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ClaimSeeker {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Deploy {
    pub amount: [u8; 8],
    pub squares: [u8; 4],
    /// Optional referrer pubkey. Set to Pubkey::default() for no referrer.
    pub referrer: [u8; 32],
    /// Whether this deploy is pooled. 0 = solo, 1 = pooled.
    pub pooled: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Log {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Reset {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Close {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Mine {
    pub nonce: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Swap {
    pub amount: [u8; 8],
    pub direction: u8,
    pub precision: u8,
    pub seed: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Uncommit {
    pub amount: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SetAdmin {
    pub admin: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SetFeeCollector {
    pub fee_collector: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SetFeeRate {
    pub fee_rate: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Buyback {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Liq {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Barrel {
    pub amount: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Wrap {
    /// 0 = use balance, 1 = use liquidity
    pub use_liquidity: u8,
    pub amount: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ReloadSOL {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Deposit {
    pub amount: [u8; 8],
    pub lock_duration_days: [u8; 8],  // 0 = no lock, 1-730 days
    pub stake_id: [u8; 8],  // Unique ID for this stake account (allows multiple stakes per user)
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Withdraw {
    pub amount: [u8; 8],
    pub stake_id: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ClaimYield {
    pub amount: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Checkpoint {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct NewVar {
    pub id: [u8; 8],
    pub commit: [u8; 32],
    pub samples: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SetAdminFee {
    pub admin_fee: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SetSwapProgram {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SetVarAddress {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Migrate {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CreateReferral {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ClaimReferral {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CreateWhitelist {
    /// The code hash (first 32 bytes of keccak256 hash of the code string)
    pub code_hash: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SetTgeTimestamp {
    /// Unix timestamp for Token Generation Event (TGE).
    /// If current time < tge_timestamp, pre-mine is active.
    /// Set to 0 to disable pre-mine.
    pub tge_timestamp: [u8; 8],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Initialize {
    pub barrel_authority: [u8; 32],
    pub fee_collector: [u8; 32],
    pub swap_program: [u8; 32],
    pub var_address: [u8; 32],
    pub admin_fee: [u8; 8],
    // Auction configuration (optional - only used if auction accounts need initialization)
    pub halving_period_seconds: [u8; 8],
    pub base_mining_rates: [[u8; 8]; 4],  // 4 wells
    pub auction_duration_seconds: [u8; 8],
    pub starting_prices: [[u8; 8]; 4],  // 4 wells
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct PlaceBid {
    pub square_id: [u8; 8],
    /// Optional referrer pubkey for new miners. Set to Pubkey::default() for no referrer.
    pub referrer: [u8; 32],
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ClaimAuctionOIL {
    /// Well IDs to claim OIL from (0-3), can claim multiple at once
    /// Bitmask: bit 0 = well 0, bit 1 = well 1, etc.
    pub well_mask: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ClaimAuctionSOL {
    /// Reserved for future use (currently unused, but kept for consistency)
    pub _reserved: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct SetAuction {
    pub halving_period_seconds: [u8; 8],
    pub last_halving_time: [u8; 8],
    pub base_mining_rates: [[u8; 8]; 4],  // 4 wells
    pub auction_duration_seconds: [u8; 8],
    pub starting_prices: [[u8; 8]; 4],  // 4 wells
    pub well_id: [u8; 8],  // Well ID to update (0-3). If >= 4, only updates auction account.
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct JoinAuctionPool {
    /// Square ID (well ID) for this pool contribution (0-3)
    pub square_id: [u8; 8],
    /// SOL amount to contribute (in lamports)
    pub amount: [u8; 8],
}

impl JoinAuctionPool {
    pub fn to_bytes(&self) -> Vec<u8> {
        bytemuck::bytes_of(self).to_vec()
    }
}

instruction!(OilInstruction, Automate);
instruction!(OilInstruction, Initialize);
instruction!(OilInstruction, Checkpoint);
instruction!(OilInstruction, ClaimSOL);
instruction!(OilInstruction, ClaimOIL);
instruction!(OilInstruction, ClaimSeeker);
instruction!(OilInstruction, ReloadSOL);
instruction!(OilInstruction, Deploy);
instruction!(OilInstruction, Log);
instruction!(OilInstruction, Buyback);
instruction!(OilInstruction, Wrap);
instruction!(OilInstruction, Reset);
instruction!(OilInstruction, Close);
instruction!(OilInstruction, SetAdmin);
instruction!(OilInstruction, SetFeeCollector);
instruction!(OilInstruction, Deposit);
instruction!(OilInstruction, Withdraw);
instruction!(OilInstruction, ClaimYield);
instruction!(OilInstruction, NewVar);
instruction!(OilInstruction, SetAdminFee);
instruction!(OilInstruction, SetSwapProgram);
instruction!(OilInstruction, SetVarAddress);
instruction!(OilInstruction, Migrate);
instruction!(OilInstruction, CreateReferral);
instruction!(OilInstruction, ClaimReferral);
instruction!(OilInstruction, PlaceBid);
instruction!(OilInstruction, ClaimAuctionOIL);
instruction!(OilInstruction, ClaimAuctionSOL);
instruction!(OilInstruction, SetAuction);
instruction!(OilInstruction, CreateWhitelist);
instruction!(OilInstruction, SetTgeTimestamp);
instruction!(OilInstruction, Liq);
instruction!(OilInstruction, Barrel);