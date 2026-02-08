mod automation;
mod auction;
mod board;
mod config;
mod micro;
mod miner;
mod pool;
mod referral;
mod rig;
mod round;
mod share;
mod well;
mod stake;
mod treasury;
mod whitelist;

pub use automation::*;
pub use auction::*;
pub use board::*;
pub use config::*;
pub use micro::*;
pub use miner::*;
pub use pool::*;
pub use referral::*;
pub use rig::*;
pub use round::*;
pub use share::*;
pub use well::*;
pub use stake::*;
pub use treasury::*;
#[allow(unused_imports)] // Exported for use in other crates (e.g., program crate)
pub use whitelist::*;
use crate::consts::*;

use steel::*;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
pub enum OilAccount {
    Automation = 100,
    Config = 101,
    Miner = 103,
    Treasury = 104,
    Board = 105,
    Stake = 108,
    Round = 109,
    Referral = 110,
    Pool = 111,
    Auction = 114,
    Well = 115,
    Rig = 116,
    Whitelist = 117,
    Micro = 118,
    Share = 119,
}

pub fn automation_pda(authority: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[AUTOMATION, &authority.to_bytes()], &crate::ID)
}

pub fn board_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[BOARD], &crate::ID)
}

pub fn config_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[CONFIG], &crate::ID)
}

pub fn miner_pda(authority: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MINER, &authority.to_bytes()], &crate::ID)
}

pub fn round_pda(id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[ROUND, &id.to_le_bytes()], &crate::ID)
}

pub fn stake_pda(authority: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[STAKE, &authority.to_bytes()], &crate::ID)
}

pub fn stake_pda_with_id(authority: Pubkey, stake_id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[STAKE, &authority.to_bytes(), &stake_id.to_le_bytes()], &crate::ID)
}

pub fn referral_pda(authority: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[REFERRAL, &authority.to_bytes()], &crate::ID)
}

pub fn treasury_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[TREASURY], &crate::ID)
}

pub fn treasury_tokens_address() -> Pubkey {
    spl_associated_token_account::get_associated_token_address(&TREASURY_ADDRESS, &MINT_ADDRESS)
}

pub fn pool_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[POOL], &crate::ID)
}

pub fn pool_tokens_address() -> Pubkey {
    let pool_address = pool_pda().0;
    spl_associated_token_account::get_associated_token_address(&pool_address, &MINT_ADDRESS)
}

pub fn auction_pda() -> (Pubkey, u8) {
    Pubkey::find_program_address(&[AUCTION], &crate::ID)
}

pub fn well_pda(well_id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[WELL, &well_id.to_le_bytes()], &crate::ID)
}

pub fn whitelist_pda(code_hash: [u8; 32]) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[WHITELIST, &code_hash], &crate::ID)
}

pub fn rig_pda(authority: Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[RIG, &authority.to_bytes()], &crate::ID)
}

pub fn micro_pda(well_id: u64, epoch_id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[MICRO, &well_id.to_le_bytes(), &epoch_id.to_le_bytes()], &crate::ID)
}

pub fn share_pda(authority: Pubkey, well_id: u64, epoch_id: u64) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[SHARE, &authority.to_bytes(), &well_id.to_le_bytes(), &epoch_id.to_le_bytes()], &crate::ID)
}