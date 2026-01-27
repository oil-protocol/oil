mod initialize;
mod automate;
mod buyback;
mod wrap;
mod checkpoint;
mod claim_oil;
mod claim_referral;
mod claim_seeker;   
mod claim_sol;
mod claim_yield;
mod close;
mod create_referral;
mod create_whitelist;
mod deploy;
mod deposit;
mod log;
mod migrate;
mod new_var;
mod reload_sol;
mod reset;
mod set_admin;
mod set_admin_fee;
mod set_fee_collector;
mod set_swap_program;
mod set_var_address;
mod withdraw;
mod place_bid;
mod claim_auction_oil;
mod claim_auction_sol;
mod set_auction;
mod set_tge_timestamp;
mod liq;
mod barrel;

use initialize::*;
use automate::*;
use buyback::*;
use wrap::*;
use checkpoint::*;
use claim_oil::*;
use claim_referral::*;
use claim_sol::*;
use claim_seeker::*;
use claim_yield::*;
use close::*;
use create_referral::*;
use create_whitelist::*;
use deploy::*;
use deposit::*;
use log::*;
use migrate::*;
use new_var::*;
use reload_sol::*;
use reset::*;
use set_admin::*;
use set_admin_fee::*;
use set_fee_collector::*;
use set_swap_program::*;
use set_var_address::*;
use withdraw::*;
use place_bid::*;
use claim_auction_oil::*;
use claim_auction_sol::*;
use set_auction::*;
use set_tge_timestamp::*;
use liq::*;
use barrel::*;
use oil_api::instruction::*;
use steel::*;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    let (ix, data) = parse_instruction(&oil_api::ID, program_id, data)?;

    match ix {
        
        // Miner
        OilInstruction::Automate => process_automate(accounts, data)?,
        OilInstruction::Checkpoint => process_checkpoint(accounts, data)?,
        OilInstruction::ClaimSOL => process_claim_sol(accounts, data)?,
        OilInstruction::ClaimOIL => process_claim_oil(accounts, data)?,
        OilInstruction::ClaimSeeker => process_claim_seeker(accounts, data)?,
        OilInstruction::Deploy => process_deploy(accounts, data)?,
        OilInstruction::Log => process_log(accounts, data)?,
        OilInstruction::Close => process_close(accounts, data)?,
        OilInstruction::Reset => process_reset(accounts, data)?,
        OilInstruction::ReloadSOL => process_reload_sol(accounts, data)?,

        // Staker
        OilInstruction::Deposit => process_deposit(accounts, data)?,
        OilInstruction::Withdraw => process_withdraw(accounts, data)?,
        OilInstruction::ClaimYield => process_claim_yield(accounts, data)?,

        // Admin
        OilInstruction::Initialize => process_initialize(accounts, data)?,
        OilInstruction::Buyback => process_buyback(accounts, data)?,
        OilInstruction::Wrap => process_wrap(accounts, data)?,
        OilInstruction::Liq => process_liq(accounts, data)?,
        OilInstruction::SetAdmin => process_set_admin(accounts, data)?,
        OilInstruction::SetFeeCollector => process_set_fee_collector(accounts, data)?,
        OilInstruction::SetSwapProgram => process_set_swap_program(accounts, data)?,
        OilInstruction::SetVarAddress => process_set_var_address(accounts, data)?,
        OilInstruction::NewVar => process_new_var(accounts, data)?,
        OilInstruction::SetAdminFee => process_set_admin_fee(accounts, data)?,
        OilInstruction::Migrate => process_migrate(accounts, data)?,
        
        // Referral
        OilInstruction::CreateReferral => process_create_referral(accounts, data)?,
        OilInstruction::ClaimReferral => process_claim_referral(accounts, data)?,

        // Pre-mine
        OilInstruction::CreateWhitelist => process_create_whitelist(accounts, data)?,
        OilInstruction::SetTgeTimestamp => process_set_tge_timestamp(accounts, data)?,

        // Auction-based mining
        OilInstruction::PlaceBid => process_place_bid(accounts, data)?,
        OilInstruction::ClaimAuctionOIL => process_claim_auction_oil(accounts, data)?,
        OilInstruction::ClaimAuctionSOL => process_claim_auction_sol(accounts, data)?,
        OilInstruction::SetAuction => process_set_auction(accounts, data)?,
        OilInstruction::Barrel => process_barrel(accounts, data)?,

    }

    Ok(())
}

entrypoint!(process_instruction);