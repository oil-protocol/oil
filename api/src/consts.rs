use const_crypto::ed25519;
use solana_program::{pubkey, pubkey::Pubkey};

/// The authority allowed to initialize the program.
pub const ADMIN_ADDRESS: Pubkey = pubkey!("DEvGq2WVuA3qkSCtwwuMYThY4onkJunEHSAxU5cieph8");

/// The decimal precision of the OIL token.
/// There are 100 billion indivisible units per OIL (called "grams").
pub const TOKEN_DECIMALS: u8 = 11;

/// One OIL token, denominated in indivisible units.
pub const ONE_OIL: u64 = 10u64.pow(TOKEN_DECIMALS as u32);

/// The duration of one minute, in seconds.
pub const ONE_MINUTE: i64 = 60;

/// The duration of one hour, in seconds.
pub const ONE_HOUR: i64 = 60 * ONE_MINUTE;

/// The duration of one day, in seconds.
pub const ONE_DAY: i64 = 24 * ONE_HOUR;

/// The number of seconds for when the winning square expires.
pub const ONE_WEEK: i64 = 7 * ONE_DAY;

/// The number of slots in one minute.
/// Fogo has ~40ms slots, so 60 seconds = 1500 slots (60 / 0.04 = 1500)
pub const ONE_MINUTE_SLOTS: u64 = 1500;

/// The number of slots in one hour.
pub const ONE_HOUR_SLOTS: u64 = 60 * ONE_MINUTE_SLOTS;

/// The number of slots in 12 hours.
pub const TWELVE_HOURS_SLOTS: u64 = 12 * ONE_HOUR_SLOTS;

/// The number of slots in one day.
pub const ONE_DAY_SLOTS: u64 = 24 * ONE_HOUR_SLOTS;

/// The number of slots in one week.
pub const ONE_WEEK_SLOTS: u64 = 7 * ONE_DAY_SLOTS;

/// The number of slots for breather between rounds.
pub const INTERMISSION_SLOTS: u64 = 300;

/// The maximum token supply (21 million).
/// Mirrors Bitcoin's 21M supply, representing a Solana-native store of value.
pub const MAX_SUPPLY: u64 = ONE_OIL * 21_000_000;

/// The seed of the automation account PDA.
pub const AUTOMATION: &[u8] = b"automation";

/// The seed of the board account PDA.
pub const BOARD: &[u8] = b"board";

/// The seed of the config account PDA.
pub const CONFIG: &[u8] = b"config";

/// The seed of the miner account PDA.
pub const MINER: &[u8] = b"miner";

/// The seed of the rig account PDA (auction-based mining).
pub const RIG: &[u8] = b"rig";

/// The seed of the referral account PDA.
pub const REFERRAL: &[u8] = b"referral";

/// The seed of the seeker account PDA.
pub const SEEKER: &[u8] = b"seeker";

/// The seed of the square account PDA.
pub const SQUARE: &[u8] = b"square";

/// The seed of the stake account PDA.
pub const STAKE: &[u8] = b"stake";

/// The seed of the round account PDA.
pub const ROUND: &[u8] = b"round";

/// The seed of the treasury account PDA.
pub const TREASURY: &[u8] = b"treasury";

/// The seed of the pool account PDA.
pub const POOL: &[u8] = b"pool";

/// The seed of the well account PDA.
pub const WELL: &[u8] = b"well";

/// The seed of the bid account PDA.
pub const BID: &[u8] = b"bid";

/// The seed of the auction account PDA.
pub const AUCTION: &[u8] = b"auction";

/// The seed of the square account PDA (auction state per well).
pub const EPOCH: &[u8] = b"epoch";

/// Program id for const pda derivations
const PROGRAM_ID: [u8; 32] = unsafe { *(&crate::id() as *const Pubkey as *const [u8; 32]) };

/// The address of the config account.
pub const CONFIG_ADDRESS: Pubkey =
    Pubkey::new_from_array(ed25519::derive_program_address(&[CONFIG], &PROGRAM_ID).0);

/// The address of the mint account.
pub const MINT_ADDRESS: Pubkey = pubkey!("oiLTuhTJc9qRDr2FcMiCUBJ3BCunNXP1LGJCG7svBSy");

/// The address of the sol mint account.
pub const SOL_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");

/// The address to indicate OIL rewards are split between all miners.
pub const SPLIT_ADDRESS: Pubkey = pubkey!("SpLiT11111111111111111111111111111111111112");

/// The address to indicate the mining pool won the lottery.
pub const POOL_ADDRESS: Pubkey = pubkey!("PooL111111111111111111111111111111111111112");

/// The address of the treasury account.
pub const TREASURY_ADDRESS: Pubkey =
    Pubkey::new_from_array(ed25519::derive_program_address(&[TREASURY], &PROGRAM_ID).0);

/// The address of the treasury account.
pub const TREASURY_BUMP: u8 = ed25519::derive_program_address(&[TREASURY], &PROGRAM_ID).1;

/// Denominator for fee calculations.
pub const DENOMINATOR_BPS: u64 = 10_000;

/// The fee paid to bots if they checkpoint a user.
pub const CHECKPOINT_FEE: u64 = 10_000; // 0.00001 SOL

/// The fixed emission per round for block-based mining.
pub const EMISSION_PER_ROUND: u64 = 200;

/// The minimum cooldown period (in seconds) between auction OIL claims to prevent spam.
pub const CLAIM_AUCTION_OIL_COOLDOWN_SECONDS: i64 = 10;

/// The floor price for auction wells (in lamports).
/// Price decays linearly from init_price down to this floor over auction_duration_seconds.
/// Once price reaches floor, it stays at floor until someone bids.
pub const AUCTION_FLOOR_PRICE: u64 = 10_000_000; // 0.01 SOL (testnet)

/// The fee paid to the admin for each transaction.
pub const ADMIN_FEE: u64 = 100; // 1%

/// The address to receive the admin fee.
pub const ADMIN_FEE_COLLECTOR: Pubkey = pubkey!("FEEFFuugN2rj9fcgrdoSfMdPFkRCfkpKi4vCGGYPyEVV");

/// The swap program used for buybacks.
pub const SWAP_PROGRAM: Pubkey = pubkey!("vnt1u7PzorND5JjweFWmDawKe2hLWoTwHU6QKz6XX98");

/// The address of the var account.
pub const VAR_ADDRESS: Pubkey = pubkey!("DQGNTK6bcSMgDQ73b5Fdg6xUwVmvzxP18v4sVdFpEHXb");

/// The address which can call the bury and wrap instructions.
pub const BURY_AUTHORITY: Pubkey = pubkey!("BoT3qYmE6xePWPU96Kf2QeuJr1pDgQ3gLWbA6kSyjzV");
