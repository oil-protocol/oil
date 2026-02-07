mod initialize;
mod automate;
mod buyback;
mod wrap;
mod checkpoint;
mod checkpoint_with_session;
mod claim_oil;
mod claim_referral;
mod claim_referral_with_session;
mod claim_sol;
mod claim_yield;
mod claim_yield_with_session;
mod close;
mod create_referral;
mod create_referral_with_session;
mod create_whitelist;
mod deploy;
mod deploy_with_session;
mod automate_with_session;
mod place_bid_with_session;
mod claim_auction_oil_with_session;
mod claim_auction_sol_with_session;
mod claim_sol_with_session;
mod claim_oil_with_session;
mod withdraw_with_session;
mod deposit;
mod deposit_with_session;
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
use checkpoint_with_session::*;
use claim_oil::*;
use claim_referral::*;
use claim_referral_with_session::*;
use claim_sol::*;
use claim_yield::*;
use claim_yield_with_session::*;
use close::*;
use create_referral::*;
use create_referral_with_session::*;
use create_whitelist::*;
use deploy::*;
use deploy_with_session::*;
use automate_with_session::*;
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
use place_bid_with_session::*;
use claim_auction_oil_with_session::*;
use claim_auction_sol_with_session::*;
use claim_sol_with_session::*;
use claim_oil_with_session::*;
use withdraw_with_session::*;
use deposit_with_session::*;
use claim_auction_oil::*;
use claim_auction_sol::*;
use set_auction::*;
use set_tge_timestamp::*;
use liq::*;
use barrel::*;
use oil_api::instruction::*;
use steel::*;

pub fn process_instruction<'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    data: &[u8],
) -> ProgramResult {
    let (ix, data) = parse_instruction(&oil_api::ID, program_id, data)?;

    match ix {
        
        // Miner
        OilInstruction::Automate => process_automate(accounts, data)?,
        OilInstruction::AutomateWithSession => process_automate_with_session(accounts, data)?,
        OilInstruction::Checkpoint => process_checkpoint(accounts, data)?,
        OilInstruction::CheckpointWithSession => process_checkpoint_with_session(accounts, data)?,
        OilInstruction::ClaimSOL => process_claim_sol(accounts, data)?,
        OilInstruction::ClaimSOLWithSession => process_claim_sol_with_session(accounts, data)?,
        OilInstruction::ClaimOIL => process_claim_oil(accounts, data)?,
        OilInstruction::ClaimOILWithSession => process_claim_oil_with_session(accounts, data)?,
        OilInstruction::Deploy => process_deploy(accounts, data)?,
        OilInstruction::DeployWithSession => process_deploy_with_session(accounts, data)?,
        OilInstruction::Log => process_log(accounts, data)?,
        OilInstruction::Close => process_close(accounts, data)?,
        OilInstruction::Reset => process_reset(accounts, data)?,
        OilInstruction::ReloadSOL => process_reload_sol(accounts, data)?,

        // Staker
        OilInstruction::Deposit => process_deposit(accounts, data)?,
        OilInstruction::DepositWithSession => process_deposit_with_session(accounts, data)?,
        OilInstruction::Withdraw => process_withdraw(accounts, data)?,
        OilInstruction::WithdrawWithSession => process_withdraw_with_session(accounts, data)?,
        OilInstruction::ClaimYield => process_claim_yield(accounts, data)?,
        OilInstruction::ClaimYieldWithSession => process_claim_yield_with_session(accounts, data)?,

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
        OilInstruction::CreateReferralWithSession => process_create_referral_with_session(accounts, data)?,
        OilInstruction::ClaimReferral => process_claim_referral(accounts, data)?,
        OilInstruction::ClaimReferralWithSession => process_claim_referral_with_session(accounts, data)?,

        // Pre-mine
        OilInstruction::CreateWhitelist => process_create_whitelist(accounts, data)?,
        OilInstruction::SetTgeTimestamp => process_set_tge_timestamp(accounts, data)?,

        // Auction-based mining
        OilInstruction::PlaceBid => process_place_bid(accounts, data)?,
        OilInstruction::PlaceBidWithSession => process_place_bid_with_session(accounts, data)?,
        OilInstruction::ClaimAuctionOIL => process_claim_auction_oil(accounts, data)?,
        OilInstruction::ClaimAuctionOILWithSession => process_claim_auction_oil_with_session(accounts, data)?,
        OilInstruction::ClaimAuctionSOL => process_claim_auction_sol(accounts, data)?,
        OilInstruction::ClaimAuctionSOLWithSession => process_claim_auction_sol_with_session(accounts, data)?,
        OilInstruction::SetAuction => process_set_auction(accounts, data)?,
        OilInstruction::Barrel => process_barrel(accounts, data)?,

    }

    Ok(())
}

entrypoint!(process_instruction);