use std::{collections::HashMap, str::FromStr};

use entropy_rng_api::prelude::*;
// Jupiter swap imports removed - unused
use oil_api::prelude::*;
use oil_api::state::Bid;
use solana_account_decoder::UiAccountEncoding;
use solana_client::{
    client_error::{reqwest::StatusCode, ClientErrorKind},
    nonblocking::rpc_client::RpcClient,
    rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig},
    rpc_filter::{Memcmp, RpcFilterType},
};
use solana_sdk::{
    address_lookup_table::{state::AddressLookupTable, AddressLookupTableAccount},
    compute_budget::ComputeBudgetInstruction,
    message::{v0::Message, VersionedMessage},
    native_token::lamports_to_sol,
    pubkey::Pubkey,
    rent::Rent,
    signature::{read_keypair_file, Signature, Signer},
    transaction::{Transaction, VersionedTransaction},
};
use solana_sdk::{keccak, pubkey};
use spl_associated_token_account::get_associated_token_address;
use spl_token::amount_to_ui_amount;
use steel::{AccountDeserialize, AccountMeta, Clock, Discriminator, Instruction};

#[tokio::main]
async fn main() {
    // Read keypair from file
    let payer =
        read_keypair_file(&std::env::var("KEYPAIR").expect("Missing KEYPAIR env var")).unwrap();

    // Build transaction
    let rpc = RpcClient::new(std::env::var("RPC").expect("Missing RPC env var"));
    match std::env::var("COMMAND")
        .expect("Missing COMMAND env var")
        .as_str()
    {
        "automations" => {
            log_automations(&rpc).await.unwrap();
        }
        "clock" => {
            log_clock(&rpc).await.unwrap();
        }
        "claim" => {
            claim(&rpc, &payer).await.unwrap();
        }
        "board" => {
            log_board(&rpc).await.unwrap();
        }
        "config" => {
            log_config(&rpc).await.unwrap();
        }
        "var" => {
            log_var(&rpc).await.unwrap();
        }
        "reset" => {
            reset(&rpc, &payer).await.unwrap();
        }
        "treasury" => {
            log_treasury(&rpc).await.unwrap();
        }
        "pool" => {
            log_pool(&rpc).await.unwrap();
        }
        "miner" => {
            log_miner(&rpc, &payer).await.unwrap();
        }
        "deploy" => {
            deploy(&rpc, &payer).await.unwrap();
        }
        "stake" => {
            log_stake(&rpc, &payer).await.unwrap();
        }
        "deploy_all" => {
            deploy_all(&rpc, &payer).await.unwrap();
        }
        "round" => {
            log_round(&rpc).await.unwrap();
        }
        "inspect_round" => {
            inspect_round(&rpc).await.unwrap();
        }
        "set_admin" => {
            set_admin(&rpc, &payer).await.unwrap();
        }
        "set_fee_collector" => {
            set_fee_collector(&rpc, &payer).await.unwrap();
        }
        "ata" => {
            ata(&rpc, &payer).await.unwrap();
        }
        "checkpoint" => {
            checkpoint(&rpc, &payer).await.unwrap();
        }
        "checkpoint_all" => {
            checkpoint_all(&rpc, &payer).await.unwrap();
        }
        "close_all" => {
            close_all(&rpc, &payer).await.unwrap();
        }
        "participating_miners" => {
            participating_miners(&rpc).await.unwrap();
        }
        "new_var" => {
            new_var(&rpc, &payer).await.unwrap();
        }
        "set_admin_fee" => {
            set_admin_fee(&rpc, &payer).await.unwrap();
        }
        "set_swap_program" => {
            set_swap_program(&rpc, &payer).await.unwrap();
        }
        "set_var_address" => {
            set_var_address(&rpc, &payer).await.unwrap();
        }
        "keys" => {
            keys(&rpc, &payer).await.unwrap();
        }
        "lut" => {
            lut(&rpc, &payer).await.unwrap();
        }
        "automation" => {
            log_automation(&rpc).await.unwrap();
        }
        "verify_migration" => {
            verify_migration(&rpc).await.unwrap();
        }
        "migrate" => {
            migrate(&rpc, &payer).await.unwrap();
        }
        "set_tge_timestamp" => {
            set_tge_timestamp(&rpc, &payer).await.unwrap();
        }
        "create_referral" => {
            create_referral(&rpc, &payer).await.unwrap();
        }
        "claim_referral" => {
            claim_referral_cmd(&rpc, &payer).await.unwrap();
        }
        "referral" => {
            log_referral(&rpc, &payer).await.unwrap();
        }
        "well" => {
            log_well(&rpc).await.unwrap();
        }
        "bid" => {
            log_bid(&rpc, &payer).await.unwrap();
        }
        "auction" => {
            log_auction(&rpc).await.unwrap();
        }
        "place_bid" => {
            place_bid(&rpc, &payer).await.unwrap();
        }
        // "initialize" => {
        //     initialize(&rpc, &payer).await.unwrap();
        // }
        _ => panic!("Invalid command"),
    };
}

async fn lut(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let recent_slot = rpc.get_slot().await? - 4;
    let (ix, lut_address) = solana_address_lookup_table_interface::instruction::create_lookup_table(
        payer.pubkey(),
        payer.pubkey(),
        recent_slot,
    );
    let board_address = oil_api::state::board_pda().0;
    let config_address = oil_api::state::config_pda().0;
    let treasury_address = oil_api::state::treasury_pda().0;
    let treasury_tokens_address = oil_api::state::treasury_tokens_address();
    let treasury_sol_address = get_associated_token_address(&treasury_address, &SOL_MINT);
    let mint_address = MINT_ADDRESS;
    let oil_program_address = oil_api::ID;
    let ex_ix = solana_address_lookup_table_interface::instruction::extend_lookup_table(
        lut_address,
        payer.pubkey(),
        Some(payer.pubkey()),
        vec![
            board_address,
            config_address,
            treasury_address,
            treasury_tokens_address,
            treasury_sol_address,
            mint_address,
            oil_program_address,
        ],
    );
    let ix_1 = Instruction {
        program_id: ix.program_id,
        accounts: ix
            .accounts
            .iter()
            .map(|a| AccountMeta::new(a.pubkey, a.is_signer))
            .collect(),
        data: ix.data,
    };
    let ix_2 = Instruction {
        program_id: ex_ix.program_id,
        accounts: ex_ix
            .accounts
            .iter()
            .map(|a| AccountMeta::new(a.pubkey, a.is_signer))
            .collect(),
        data: ex_ix.data,
    };
    submit_transaction(rpc, payer, &[ix_1, ix_2]).await?;
    println!("LUT address: {}", lut_address);
    Ok(())
}

async fn set_admin_fee(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let admin_fee = std::env::var("ADMIN_FEE").expect("Missing ADMIN_FEE env var");
    let admin_fee = u64::from_str(&admin_fee).expect("Invalid ADMIN_FEE");
    let ix = oil_api::sdk::set_admin_fee(payer.pubkey(), admin_fee);
    submit_transaction(rpc, payer, &[ix]).await?;
    Ok(())
}

async fn set_var_address(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let new_var_address = std::env::var("VAR").expect("Missing VAR env var");
    let new_var_address = Pubkey::from_str(&new_var_address).expect("Invalid VAR");
    let ix = oil_api::sdk::set_var_address(payer.pubkey(), new_var_address);
    submit_transaction(rpc, payer, &[ix]).await?;
    Ok(())
}

async fn new_var(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let provider = std::env::var("PROVIDER").expect("Missing PROVIDER env var");
    let provider = Pubkey::from_str(&provider).expect("Invalid PROVIDER");
    let commit = std::env::var("COMMIT").expect("Missing COMMIT env var");
    let commit = keccak::Hash::from_str(&commit).expect("Invalid COMMIT");
    let samples = std::env::var("SAMPLES").expect("Missing SAMPLES env var");
    let samples = u64::from_str(&samples).expect("Invalid SAMPLES");
    let board_address = board_pda().0;
    let var_address = entropy_rng_api::state::var_pda(board_address, 0).0;
    println!("Var address: {}", var_address);
    let ix = oil_api::sdk::new_var(payer.pubkey(), provider, 0, commit.to_bytes(), samples);
    submit_transaction(rpc, payer, &[ix]).await?;
    Ok(())
}

async fn participating_miners(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let round_id = std::env::var("ID").expect("Missing ID env var");
    let round_id = u64::from_str(&round_id).expect("Invalid ID");
    let miners = get_miners_participating(rpc, round_id).await?;
    for (i, (_address, miner)) in miners.iter().enumerate() {
        println!("{}: {}", i, miner.authority);
    }
    Ok(())
}

async fn log_stake(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let authority = std::env::var("AUTHORITY").unwrap_or(payer.pubkey().to_string());
    let authority = Pubkey::from_str(&authority).expect("Invalid AUTHORITY");
    let staker_address = oil_api::state::stake_pda(authority).0;
    let stake = get_stake(rpc, authority).await?;
    println!("Stake");
    println!("  address: {}", staker_address);
    println!("  authority: {}", authority);
    println!(
        "  balance: {} OIL",
        amount_to_ui_amount(stake.balance, TOKEN_DECIMALS)
    );
    println!("  last_claim_at: {}", stake.last_claim_at);
    println!("  last_deposit_at: {}", stake.last_deposit_at);
    println!("  last_withdraw_at: {}", stake.last_withdraw_at);
    println!(
        "  rewards_factor: {}",
        stake.rewards_factor.to_i80f48().to_string()
    );
    println!(
        "  rewards: {} SOL",
        lamports_to_sol(stake.rewards)
    );
    println!(
        "  lifetime_rewards: {} SOL",
        lamports_to_sol(stake.lifetime_rewards)
    );
    println!(
        "  lock_duration_days: {}",
        stake.lock_duration_days
    );
    println!(
        "  lock_ends_at: {}",
        stake.lock_ends_at
    );
    
    Ok(())
}

async fn ata(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let user = pubkey!("FgZFnb3bi7QexKCdXWPwWy91eocUD7JCFySHb83vLoPD");
    let token = pubkey!("8H8rPiWW4iTFCfEkSnf7jpqeNpFfvdH9gLouAL3Fe2Zx");
    let ata = get_associated_token_address(&user, &token);
    let ix = spl_associated_token_account::instruction::create_associated_token_account(
        &payer.pubkey(),
        &user,
        &token,
        &spl_token::ID,
    );
    submit_transaction(rpc, payer, &[ix]).await?;
    let account = rpc.get_account(&ata).await?;
    println!("ATA: {}", ata);
    println!("Account: {:?}", account);
    Ok(())
}

async fn keys(rpc: &RpcClient, payer: &solana_sdk::signer::keypair::Keypair) -> Result<(), anyhow::Error> {
    let treasury_address = oil_api::state::treasury_pda().0;
    let config_address = oil_api::state::config_pda().0;
    let board_address = oil_api::state::board_pda().0;
    
    // Use the actual signer's address for miner PDA
    let signer_address = payer.pubkey();
    let miner_address = oil_api::state::miner_pda(signer_address).0;
    
    // Get current round ID from board
    let board = get_board(rpc).await?;
    let round = oil_api::state::round_pda(board.round_id).0;
    
    println!("Round: {}", round);
    println!("Treasury: {}", treasury_address);
    println!("Config: {}", config_address);
    println!("Board: {}", board_address);
    println!("Miner: {} (for signer: {})", miner_address, signer_address);
    Ok(())
}

async fn claim(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    // Get miner to check for referrer
    let miner = get_miner(rpc, payer.pubkey()).await?;
    
    // Get referrer accounts if miner has a referrer (single-tier system)
    let (referrer_miner, referrer_referral, referrer_referral_oil_ata) = if miner.referrer != Pubkey::default() {
        let referrer_miner_pda = oil_api::state::miner_pda(miner.referrer).0;
        let referrer_referral_pda = oil_api::state::referral_pda(miner.referrer).0;
        let referrer_referral_oil_ata = spl_associated_token_account::get_associated_token_address(&referrer_referral_pda, &oil_api::consts::MINT_ADDRESS);
        (Some(referrer_miner_pda), Some(referrer_referral_pda), Some(referrer_referral_oil_ata))
                } else {
        (None, None, None)
    };
    
    let ix_sol = oil_api::sdk::claim_sol(payer.pubkey(), referrer_miner, referrer_referral);
    let ix_oil = oil_api::sdk::claim_oil(payer.pubkey(), referrer_miner, referrer_referral, referrer_referral_oil_ata);
    submit_transaction(rpc, payer, &[ix_sol, ix_oil]).await?;
    Ok(())
}

#[allow(dead_code)]
pub async fn get_address_lookup_table_accounts(
    rpc_client: &RpcClient,
    addresses: Vec<Pubkey>,
) -> Result<Vec<AddressLookupTableAccount>, anyhow::Error> {
    let mut accounts = Vec::new();
    for key in addresses {
        if let Ok(account) = rpc_client.get_account(&key).await {
            if let Ok(address_lookup_table_account) = AddressLookupTable::deserialize(&account.data)
            {
                accounts.push(AddressLookupTableAccount {
                    key,
                    addresses: address_lookup_table_account.addresses.to_vec(),
                });
            }
        }
    }
    Ok(accounts)
}

async fn reset(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let board = get_board(rpc).await?;
    let config = get_config(rpc).await?;
    // Use var address from config (set via set_var_address)
    let var_address = config.var_address;
    let var = get_var(rpc, var_address).await?;

    println!("Var: {:?}", var);

    let client = reqwest::Client::new();
    let url = format!("https://entropy-rng-api.up.railway.app/var/{var_address}/seed");
    let response = client
        .get(url)
        .send()
        .await?
        .json::<entropy_types::response::GetSeedResponse>()
        .await?;
    println!("Entropy seed: {:?}", response);

    let sample_ix = entropy_rng_api::sdk::sample(payer.pubkey(), var_address);
    let reveal_ix = entropy_rng_api::sdk::reveal(payer.pubkey(), var_address, response.seed);
    let reset_ix = oil_api::sdk::reset(
        payer.pubkey(),
        config.fee_collector,
        board.round_id,
        Pubkey::default(),
        var_address, // Var address from Config account
    );
    let sig = submit_transaction(rpc, payer, &[sample_ix, reveal_ix, reset_ix]).await?;
    println!("Reset: {}", sig);

    // let slot_hashes = get_slot_hashes(rpc).await?;
    // if let Some(slot_hash) = slot_hashes.get(&board.end_slot) {
    //     let id = get_winning_square(&slot_hash.to_bytes());
    //     // let square = get_square(rpc).await?;
    //     println!("Winning square: {}", id);
    //     // println!("Miners: {:?}", square.miners);
    //     // miners = square.miners[id as usize].to_vec();
    // };

    // let reset_ix = oil_api::sdk::reset(
    //     payer.pubkey(),
    //     config.fee_collector,
    //     board.round_id,
    //     Pubkey::default(),
    // );
    // // simulate_transaction(rpc, payer, &[reset_ix]).await;
    // submit_transaction(rpc, payer, &[reset_ix]).await?;
    Ok(())
}

async fn deploy(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let amount = std::env::var("AMOUNT").expect("Missing AMOUNT env var");
    let amount = u64::from_str(&amount).expect("Invalid AMOUNT");
    let square_id = std::env::var("SQUARE").expect("Missing SQUARE env var");
    let square_id = u64::from_str(&square_id).expect("Invalid SQUARE");
    let board = get_board(rpc).await?;
    let mut squares = [false; 25];
    squares[square_id as usize] = true;
    // Check for optional referrer env var.
    let referrer = std::env::var("REFERRER")
        .ok()
        .and_then(|s| Pubkey::from_str(&s).ok());
    // Check for optional pooled env var.
    let pooled = std::env::var("POOLED")
        .ok()
        .map(|s| s == "1" || s.to_lowercase() == "true")
        .unwrap_or(false);
    let ix = oil_api::sdk::deploy(
        payer.pubkey(),
        payer.pubkey(),
        amount,
        board.round_id,
        squares,
        referrer,
        pooled
    );
    submit_transaction(rpc, payer, &[ix]).await?;
    Ok(())
}

async fn deploy_all(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let amount = std::env::var("AMOUNT").expect("Missing AMOUNT env var");
    let amount = u64::from_str(&amount).expect("Invalid AMOUNT");
    let board = get_board(rpc).await?;
    let squares = [true; 25];
    // Check for optional referrer env var.
    let referrer = std::env::var("REFERRER")
        .ok()
        .and_then(|s| Pubkey::from_str(&s).ok());
    // Check for optional pooled env var.
    let pooled = std::env::var("POOLED")
        .ok()
        .map(|s| s == "1" || s.to_lowercase() == "true")
        .unwrap_or(false);
    let ix = oil_api::sdk::deploy(
        payer.pubkey(),
        payer.pubkey(),
        amount,
        board.round_id,
        squares,
        referrer,
        pooled
    );
    submit_transaction(rpc, payer, &[ix]).await?;
    Ok(())
}

// async fn initialize(
//     rpc: &RpcClient,
//     payer: &solana_sdk::signer::keypair::Keypair,
// ) -> Result<(), anyhow::Error> {
//     let barrel_authority = std::env::var("BURY_AUTHORITY")
//         .ok()
//         .and_then(|s| Pubkey::from_str(&s).ok())
//         .unwrap_or(payer.pubkey());
//     let fee_collector = std::env::var("FEE_COLLECTOR")
//         .ok()
//         .and_then(|s| Pubkey::from_str(&s).ok())
//         .unwrap_or(payer.pubkey());
//     let swap_program = std::env::var("SWAP_PROGRAM")
//         .ok()
//         .and_then(|s| Pubkey::from_str(&s).ok())
//         .unwrap_or(Pubkey::default());
//     let var_address = std::env::var("VAR_ADDRESS")
//         .ok()
//         .and_then(|s| Pubkey::from_str(&s).ok())
//         .unwrap_or(Pubkey::default());
//     let admin_fee = std::env::var("ADMIN_FEE")
//         .ok()
//         .and_then(|s| u64::from_str(&s).ok())
//         .unwrap_or(100); // Default 1% (100 basis points)
//     
//     let ix = oil_api::sdk::initialize(
//         payer.pubkey(),
//         barrel_authority,
//         fee_collector,
//         swap_program,
//         var_address,
//         admin_fee,
//     );
//     submit_transaction(rpc, payer, &[ix]).await?;
//     Ok(())
// }

async fn set_admin(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let ix = oil_api::sdk::set_admin(payer.pubkey(), payer.pubkey());
    submit_transaction(rpc, payer, &[ix]).await?;
    Ok(())
}

async fn set_swap_program(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let swap_program = std::env::var("SWAP_PROGRAM").expect("Missing SWAP_PROGRAM env var");
    let swap_program = Pubkey::from_str(&swap_program).expect("Invalid SWAP_PROGRAM");
    let ix = oil_api::sdk::set_swap_program(payer.pubkey(), swap_program);
    submit_transaction(rpc, payer, &[ix]).await?;
    Ok(())
}

async fn set_fee_collector(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let fee_collector = std::env::var("FEE_COLLECTOR").expect("Missing FEE_COLLECTOR env var");
    let fee_collector = Pubkey::from_str(&fee_collector).expect("Invalid FEE_COLLECTOR");
    let ix = oil_api::sdk::set_fee_collector(payer.pubkey(), fee_collector);
    submit_transaction(rpc, payer, &[ix]).await?;
    Ok(())
}

async fn set_tge_timestamp(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    // Check if TGE_TIMESTAMP env var is set, otherwise calculate 2 hours from now
    let tge_timestamp = if let Ok(ts_str) = std::env::var("TGE_TIMESTAMP") {
        // Use provided timestamp
        i64::from_str(&ts_str).expect("Invalid TGE_TIMESTAMP (must be Unix timestamp)")
    } else {
        // Default: 2 hours from now
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let two_hours = 2 * 60 * 60; // 2 hours in seconds
        let tge = now + two_hours;
        
        println!("No TGE_TIMESTAMP provided, setting to 2 hours from now");
        println!("  Current time: {} (Unix timestamp)", now);
        println!("  TGE timestamp: {} (Unix timestamp)", tge);
        println!("  Duration: 2 hours (7200 seconds)");
        
        tge
    };
    
    println!("\n🔧 Setting TGE Timestamp");
    println!("   TGE timestamp: {}", tge_timestamp);
    
    if tge_timestamp == 0 {
        println!("   Pre-mine: 🔴 Disabled");
    } else {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        if now < tge_timestamp {
            let remaining = tge_timestamp - now;
            let hours = remaining / 3600;
            let minutes = (remaining % 3600) / 60;
            println!("   Pre-mine: 🟢 Active for {}h {}m", hours, minutes);
        } else {
            println!("   Pre-mine: 🔴 Inactive (TGE has passed)");
        }
    }
    
    let ix = oil_api::sdk::set_tge_timestamp(payer.pubkey(), tge_timestamp);
    let sig = submit_transaction(rpc, payer, &[ix]).await?;
    println!("   ✅ Transaction: {}\n", sig);
    
    Ok(())
}

async fn checkpoint(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let authority = std::env::var("AUTHORITY").unwrap_or(payer.pubkey().to_string());
    let authority = Pubkey::from_str(&authority).expect("Invalid AUTHORITY");
    let miner = get_miner(rpc, authority).await?;
    let steel_ix = oil_api::sdk::checkpoint(payer.pubkey(), authority, miner.round_id);
    
    // Convert steel::Instruction to solana_sdk::instruction::Instruction
    let solana_ix = solana_sdk::instruction::Instruction {
        program_id: steel_ix.program_id,
        accounts: steel_ix.accounts.iter().map(|a| solana_sdk::instruction::AccountMeta {
            pubkey: a.pubkey,
            is_signer: a.is_signer,
            is_writable: a.is_writable,
        }).collect(),
        data: steel_ix.data,
    };
    
    submit_transaction(rpc, payer, &[solana_ix]).await?;
    Ok(())
}

async fn checkpoint_all(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let clock = get_clock(rpc).await?;
    let miners = get_miners(rpc).await?;
    let mut expiry_slots = HashMap::new();
    let mut ixs: Vec<solana_sdk::instruction::Instruction> = vec![];
    for (i, (_address, miner)) in miners.iter().enumerate() {
        if miner.checkpoint_id < miner.round_id {
            // Log the expiry slot for the round.
            if !expiry_slots.contains_key(&miner.round_id) {
                if let Ok(round) = get_round(rpc, miner.round_id).await {
                    expiry_slots.insert(miner.round_id, round.expires_at);
                }
            }

            // Get the expiry slot for the round.
            let Some(expires_at) = expiry_slots.get(&miner.round_id) else {
                continue;
            };

            // If we are in fee collection period, checkpoint the miner.
            if clock.slot >= expires_at.saturating_sub(TWELVE_HOURS_SLOTS) {
                let time_remaining = expires_at.saturating_sub(clock.slot);
                println!(
                    "[{}/{}] Checkpoint miner: {} ({} s)",
                    i + 1,
                    miners.len(),
                    miner.authority,
                    time_remaining as f64 * 0.4
                );
                let steel_ix = oil_api::sdk::checkpoint(
                    payer.pubkey(),
                    miner.authority,
                    miner.round_id,
                );
                // Convert steel::Instruction to solana_sdk::instruction::Instruction
                let solana_ix = solana_sdk::instruction::Instruction {
                    program_id: steel_ix.program_id,
                    accounts: steel_ix.accounts.iter().map(|a| solana_sdk::instruction::AccountMeta {
                        pubkey: a.pubkey,
                        is_signer: a.is_signer,
                        is_writable: a.is_writable,
                    }).collect(),
                    data: steel_ix.data,
                };
                ixs.push(solana_ix);
            }
        }
    }

    // Batch and submit the instructions.
    // Each checkpoint uses ~200k CU, so max 6 per batch to stay under 1.4M CU limit
    const MAX_BATCH_SIZE: usize = 6;
    let mut batch_num = 0;
    while !ixs.is_empty() {
        let batch = ixs
            .drain(..std::cmp::min(MAX_BATCH_SIZE, ixs.len()))
            .collect::<Vec<solana_sdk::instruction::Instruction>>();
        batch_num += 1;
        match submit_transaction(rpc, payer, &batch).await {
            Ok(sig) => {
                println!("Batch {} submitted successfully: {}", batch_num, sig);
            }
            Err(e) => {
                eprintln!("Error submitting batch {}: {:?}", batch_num, e);
                eprintln!("This batch will be skipped. Continuing with remaining batches...");
                // Continue processing other batches even if one fails
            }
        }
    }

    Ok(())
}

async fn close_all(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let rounds = get_rounds(rpc).await?;
    let mut ixs = vec![];
    let clock = get_clock(rpc).await?;
    for (_i, (_address, round)) in rounds.iter().enumerate() {
        if clock.slot >= round.expires_at {
            ixs.push(oil_api::sdk::close(
                payer.pubkey(),
                round.id,
                round.rent_payer,
            ));
        }
    }

    // Batch and submit the instructions.
    while !ixs.is_empty() {
        let batch = ixs
            .drain(..std::cmp::min(12, ixs.len()))
            .collect::<Vec<Instruction>>();
        // simulate_transaction(rpc, payer, &batch).await;
        submit_transaction(rpc, payer, &batch).await?;
    }

    Ok(())
}

// async fn log_meteora_pool(rpc: &RpcClient) -> Result<(), anyhow::Error> {
//     let address = pubkey!("GgaDTFbqdgjoZz3FP7zrtofGwnRS4E6MCzmmD5Ni1Mxj");
//     let pool = get_meteora_pool(rpc, address).await?;
//     let vault_a = get_meteora_vault(rpc, pool.a_vault).await?;
//     let vault_b = get_meteora_vault(rpc, pool.b_vault).await?;

//     println!("Pool");
//     println!("  address: {}", address);
//     println!("  lp_mint: {}", pool.lp_mint);
//     println!("  token_a_mint: {}", pool.token_a_mint);
//     println!("  token_b_mint: {}", pool.token_b_mint);
//     println!("  a_vault: {}", pool.a_vault);
//     println!("  b_vault: {}", pool.b_vault);
//     println!("  a_token_vault: {}", vault_a.token_vault);
//     println!("  b_token_vault: {}", vault_b.token_vault);
//     println!("  a_vault_lp_mint: {}", vault_a.lp_mint);
//     println!("  b_vault_lp_mint: {}", vault_b.lp_mint);
//     println!("  a_vault_lp: {}", pool.a_vault_lp);
//     println!("  b_vault_lp: {}", pool.b_vault_lp);
//     println!("  protocol_token_fee: {}", pool.protocol_token_b_fee);

//     // pool: *pool.key,
//     // user_source_token: *user_source_token.key,
//     // user_destination_token: *user_destination_token.key,
//     // a_vault: *a_vault.key,
//     // b_vault: *b_vault.key,
//     // a_token_vault: *a_token_vault.key,
//     // b_token_vault: *b_token_vault.key,
//     // a_vault_lp_mint: *a_vault_lp_mint.key,
//     // b_vault_lp_mint: *b_vault_lp_mint.key,
//     // a_vault_lp: *a_vault_lp.key,
//     // b_vault_lp: *b_vault_lp.key,
//     // protocol_token_fee: *protocol_token_fee.key,
//     // user: *user.key,
//     // vault_program: *vault_program.key,
//     // token_program: *token_program.key,

//     Ok(())
// }

async fn log_automation(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let authority = std::env::var("AUTHORITY").expect("Missing AUTHORITY env var");
    let authority = Pubkey::from_str(&authority).expect("Invalid AUTHORITY");
    let address = automation_pda(authority).0;
    let automation = get_automation(rpc, address).await?;
    let account_balance = rpc.get_balance(&address).await?;
    let size = 8 + std::mem::size_of::<Automation>();
    let required_rent = Rent::default().minimum_balance(size);
    println!("Automation");
    println!("  address: {}", address);
    println!("  amount: {} SOL", lamports_to_sol(automation.amount));
    println!("  required rent: {} SOL", lamports_to_sol(required_rent));
    println!("  authority: {}", automation.authority);
    println!("  balance: {} SOL", lamports_to_sol(automation.balance));
    println!("  lamports: {} SOL", lamports_to_sol(account_balance));
    println!("  executor: {}", automation.executor);
    println!("  fee: {} SOL", lamports_to_sol(automation.fee));
    println!("  mask: {}", automation.mask);
    println!("  strategy: {}", automation.strategy);
    println!("  reload: {}", automation.reload);
    Ok(())
}

async fn log_automations(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let automations = get_automations(rpc).await?;
    for (i, (address, automation)) in automations.iter().enumerate() {
        println!("[{}/{}] {}", i + 1, automations.len(), address);
        println!("  authority: {}", automation.authority);
        println!("  balance: {}", automation.balance);
        println!("  executor: {}", automation.executor);
        println!("  fee: {}", automation.fee);
        println!("  mask: {}", automation.mask);
        println!("  strategy: {}", automation.strategy);
        println!();
    }
    Ok(())
}

async fn log_treasury(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let treasury_address = oil_api::state::treasury_pda().0;
    let treasury_pda = oil_api::state::treasury_pda();
    let account = rpc.get_account(&treasury_pda.0).await?;
    let treasury = get_treasury(rpc).await?;
    
    // Check account size to determine if migrated
    let expected_size = 8 + std::mem::size_of::<Treasury>();
    let is_migrated = account.data.len() >= expected_size;
    
    println!("Treasury");
    println!("  address: {}", treasury_address);
    println!("  account size: {} bytes (expected: {} bytes) {}", 
        account.data.len(), 
        expected_size,
        if is_migrated { "✅ Migrated" } else { "⚠️  Needs Migration" }
    );
    println!("  balance: {} SOL", lamports_to_sol(treasury.balance));
    println!(
        "  gusher_sol: {} SOL",
        lamports_to_sol(treasury.gusher_sol)
    );
    println!(
        "  block_rewards_factor: {}",
        treasury.block_rewards_factor.to_i80f48().to_string()
    );
    println!(
        "  buffer_a: {}",
        treasury.buffer_a.to_i80f48().to_string()
    );
    println!(
        "  total_barrelled: {} OIL",
        amount_to_ui_amount(treasury.total_barrelled, TOKEN_DECIMALS)
    );
    println!(
        "  block_total_refined: {} OIL",
        amount_to_ui_amount(treasury.block_total_refined, TOKEN_DECIMALS)
    );
    println!(
        "  auction_rewards_sol: {} SOL",
        lamports_to_sol(treasury.auction_rewards_sol)
    );
    println!(
        "  block_total_unclaimed: {} OIL",
        amount_to_ui_amount(treasury.block_total_unclaimed, TOKEN_DECIMALS)
    );
    println!(
        "  auction_rewards_factor: {}",
        treasury.auction_rewards_factor.to_i80f48().to_string()
    );
    println!(
        "  auction_total_unclaimed: {} OIL",
        amount_to_ui_amount(treasury.auction_total_unclaimed, TOKEN_DECIMALS)
    );
    println!(
        "  auction_total_refined: {} OIL",
        amount_to_ui_amount(treasury.auction_total_refined, TOKEN_DECIMALS)
    );
    println!(
        "  liquidity: {} SOL",
        lamports_to_sol(treasury.liquidity)
    );
    
    Ok(())
}

async fn log_pool(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let pool_address = oil_api::state::pool_pda().0;
    let pool_pda = oil_api::state::pool_pda();
    let account = rpc.get_account(&pool_pda.0).await?;
    
    if account.data.is_empty() {
        println!("Pool account not found (not initialized yet)");
        return Ok(());
    }
    
    let pool = Pool::try_from_bytes(&account.data)?;
    let expected_size = 8 + std::mem::size_of::<Pool>();
    
    println!("Pool (Staking)");
    println!("  address: {}", pool_address);
    println!("  account size: {} bytes (expected: {} bytes)", 
        account.data.len(), 
        expected_size
    );
    println!(
        "  balance: {} SOL (available for stakers)",
        lamports_to_sol(pool.balance)
    );
    println!(
        "  stake_rewards_factor: {}",
        pool.stake_rewards_factor.to_i80f48().to_string()
    );
    println!(
        "  total_staked: {} OIL (stakers earn SOL rewards)",
        amount_to_ui_amount(pool.total_staked, TOKEN_DECIMALS)
    );
    println!(
        "  total_staked_score: {} OIL",
        amount_to_ui_amount(pool.total_staked_score, TOKEN_DECIMALS)
    );
    
    Ok(())
}

async fn log_round(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let id = std::env::var("ID").expect("Missing ID env var");
    let id = u64::from_str(&id).expect("Invalid ID");
    let round_address = round_pda(id).0;
    let round_pda = oil_api::state::round_pda(id);
    let account = rpc.get_account(&round_pda.0).await?;
    let round = get_round(rpc, id).await?;
    let rng = round.rng();
    
    // Check account size to determine if migrated
    let expected_size = 8 + std::mem::size_of::<Round>();
    let is_migrated = account.data.len() >= expected_size;
    
    println!("Round");
    println!("  Address: {}", round_address);
    println!("  account size: {} bytes (expected: {} bytes) {}", 
        account.data.len(), 
        expected_size,
        if is_migrated { "✅ Migrated" } else { "⚠️  Needs Migration" }
    );
    println!("  Count: {:?}", round.count);
    println!("  Deployed: {:?}", round.deployed);
    println!("  Expires at: {}", round.expires_at);
    println!("  Id: {:?}", round.id);
    if is_migrated {
        println!("  Gusher SOL: {} SOL", lamports_to_sol(round.gusher_sol));
    }
    println!("  Rent payer: {}", round.rent_payer);
    println!("  Slot hash: {:?}", round.slot_hash);
    println!("  Top miner: {:?}", round.top_miner);
    println!("  Top miner reward: {} OIL", amount_to_ui_amount(round.top_miner_reward, TOKEN_DECIMALS));
    println!("  Total deployed: {} SOL", lamports_to_sol(round.total_deployed));
    println!("  Total vaulted: {} SOL", lamports_to_sol(round.total_vaulted));
    println!("  Total winnings: {} SOL", lamports_to_sol(round.total_winnings));
    if let Some(rng) = rng {
        println!("  Winning square: {}", round.winning_square(rng));
    }
    // if round.slot_hash != [0; 32] {
    //     println!("  Winning square: {}", get_winning_square(&round.slot_hash));
    // }
    println!("  Pool members: {}", round.pool_members);
    println!("  Pool rewards SOL: {} SOL", lamports_to_sol(round.pool_rewards_sol));
    println!("  Pool rewards OIL: {} OIL", amount_to_ui_amount(round.pool_rewards_oil, TOKEN_DECIMALS));
    println!("  Total pooled: {} SOL", lamports_to_sol(round.total_pooled));
    println!("  Pool cumulative: {:?}", round.pool_cumulative);
    Ok(())
}

async fn inspect_round(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let address_str = std::env::var("ADDRESS").expect("Missing ADDRESS env var");
    let address = Pubkey::from_str(&address_str).expect("Invalid address");
    
    let account = rpc.get_account(&address).await?;
    let account_balance = account.lamports;
    let expected_size = 8 + std::mem::size_of::<Round>();
    
    println!("Round Account Inspection");
    println!("  Address: {}", address);
    println!("  Balance: {} SOL ({} lamports)", lamports_to_sol(account_balance), account_balance);
    println!("  Account size: {} bytes (expected: {} bytes)", account.data.len(), expected_size);
    
    // Check if account is empty
    if account.data.is_empty() {
        println!("  Status: ❌ Account is empty (does not exist)");
        return Ok(());
    }
    
    // Check rent exemption
    let rent = Rent::default();
    let min_rent = rent.minimum_balance(account.data.len());
    let is_rent_exempt = account_balance >= min_rent;
    
    println!("  Minimum rent: {} SOL ({} lamports)", lamports_to_sol(min_rent), min_rent);
    println!("  Rent exempt: {}", if is_rent_exempt { "✅ Yes" } else { "❌ No (INSUFFICIENT FUNDS FOR RENT)" });
    
    if !is_rent_exempt {
        let deficit = min_rent.saturating_sub(account_balance);
        println!("  Rent deficit: {} SOL ({} lamports)", lamports_to_sol(deficit), deficit);
    }
    
    // Try to deserialize as Round
    match Round::try_from_bytes(&account.data) {
        Ok(round) => {
            println!("  Status: ✅ Successfully deserialized as Round");
            println!("  Round ID: {}", round.id);
            println!("  Rent payer: {}", round.rent_payer);
            println!("  Expires at: {}", round.expires_at);
            println!("  Total deployed: {} SOL", lamports_to_sol(round.total_deployed));
            println!("  Total vaulted: {} SOL", lamports_to_sol(round.total_vaulted));
            println!("  Total winnings: {} SOL", lamports_to_sol(round.total_winnings));
        }
        Err(e) => {
            println!("  Status: ❌ Failed to deserialize as Round: {}", e);
            println!("  This account may be in an invalid state or corrupted");
        }
    }
    
    Ok(())
}

async fn verify_migration(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    println!("🔍 Verifying account migration status...\n");
    
    // Check Treasury
    let treasury_pda = oil_api::state::treasury_pda();
    let treasury_account = rpc.get_account(&treasury_pda.0).await?;
    let expected_treasury_size = 8 + std::mem::size_of::<Treasury>();
    let treasury_migrated = treasury_account.data.len() >= expected_treasury_size;
    
    println!("📊 Treasury Account:");
    println!("   Address: {}", treasury_pda.0);
    println!("   Current size: {} bytes", treasury_account.data.len());
    println!("   Expected size: {} bytes", expected_treasury_size);
    println!("   Status: {}", if treasury_migrated { "✅ Migrated" } else { "⚠️  Needs Migration" });
    
    if treasury_migrated {
        let treasury = get_treasury(rpc).await?;
        println!("   gusher_sol: {} SOL", lamports_to_sol(treasury.gusher_sol));
    }
    println!();
    
    // Check Board to get current round
    let board = get_board(rpc).await?;
    println!("📊 Board:");
    println!("   Current round_id: {}", board.round_id);
    println!();
    
    // Check current Round if it exists
    let round_pda = oil_api::state::round_pda(board.round_id);
    if let Ok(round_account) = rpc.get_account(&round_pda.0).await {
        let expected_round_size = 8 + std::mem::size_of::<Round>();
        let round_migrated = round_account.data.len() >= expected_round_size;
        
        println!("📊 Round {} Account:", board.round_id);
        println!("   Address: {}", round_pda.0);
        println!("   Current size: {} bytes", round_account.data.len());
        println!("   Expected size: {} bytes", expected_round_size);
        println!("   Status: {}", if round_migrated { "✅ Migrated" } else { "⚠️  Needs Migration" });
        
        if round_migrated {
            let round = get_round(rpc, board.round_id).await?;
            println!("   gusher_sol: {} SOL", lamports_to_sol(round.gusher_sol));
        }
        println!();
    } else {
        println!("📊 Round {} Account:", board.round_id);
        println!("   Status: ❌ Account not found (round not started yet)");
        println!();
    }
    
    // Summary
    println!("📋 Migration Summary:");
    if treasury_migrated {
        println!("   ✅ Treasury: Migrated");
    } else {
        println!("   ⚠️  Treasury: Needs Migration");
    }
    
    if let Ok(round_account) = rpc.get_account(&round_pda.0).await {
        let expected_round_size = 8 + std::mem::size_of::<Round>();
        let round_migrated = round_account.data.len() >= expected_round_size;
        if round_migrated {
            println!("   ✅ Round {}: Migrated", board.round_id);
        } else {
            println!("   ⚠️  Round {}: Needs Migration", board.round_id);
        }
    } else {
        println!("   ℹ️  Round {}: Not started (no migration needed)", board.round_id);
    }
    
    Ok(())
}

/// Migrate: Extend Treasury struct with liquidity field.
async fn migrate(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let treasury_address = oil_api::state::treasury_pda().0;

    println!("\n🔧 Treasury Migration");
    println!("   Treasury: {}", treasury_address);

    // Check current treasury
    let treasury_account = rpc.get_account(&treasury_address).await?;
    let expected_treasury_size = 128; // 8 discriminator + 120 data bytes (old was 120 total)
    
    println!("\n📊 Current State:");
    println!("   Treasury account size: {} bytes", treasury_account.data.len());
    println!("   Expected size: {} bytes", expected_treasury_size);
    
    let is_migrated = treasury_account.data.len() >= expected_treasury_size;
    
    if is_migrated {
        println!("   Status: ✅ Already migrated");
        println!("\n   ✅ No migration needed. Treasury already has liquidity field.");
        return Ok(());
    }
    
    println!("   Status: ⚠️  Needs migration");
    println!("\n   📦 Will extend Treasury struct with liquidity field (u64, 8 bytes)");
    
    // Build migrate instruction
    let migrate_ix = oil_api::sdk::migrate(payer.pubkey());
    
    // Debug: Print instruction details
    println!("\n   🔍 Instruction details:");
    println!("      Program ID: {}", migrate_ix.program_id);
    println!("      Accounts: {}", migrate_ix.accounts.len());
    for (i, account) in migrate_ix.accounts.iter().enumerate() {
        println!("        [{}] {} (signer: {}, writable: {})", 
            i, account.pubkey, account.is_signer, account.is_writable);
    }
    println!("      Data length: {} bytes", migrate_ix.data.len());
    println!("      Data (first 10 bytes): {:?}", &migrate_ix.data[..migrate_ix.data.len().min(10)]);
    
    // Simulate transaction first to catch errors
    println!("\n   🔍 Simulating transaction...");
    let blockhash = rpc.get_latest_blockhash().await?;
    let mut all_instructions = vec![
        ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
        ComputeBudgetInstruction::set_compute_unit_price(1_000_000),
    ];
    // Convert steel::Instruction to solana_sdk::Instruction for simulation
    let solana_ix = solana_sdk::instruction::Instruction {
        program_id: migrate_ix.program_id,
        accounts: migrate_ix.accounts.iter().map(|a| solana_sdk::instruction::AccountMeta {
            pubkey: a.pubkey,
            is_signer: a.is_signer,
            is_writable: a.is_writable,
        }).collect(),
        data: migrate_ix.data.clone(),
    };
    all_instructions.push(solana_ix);
    let transaction = Transaction::new_signed_with_payer(
        &all_instructions,
        Some(&payer.pubkey()),
        &[payer],
        blockhash,
    );
    
    match rpc.simulate_transaction(&transaction).await {
        Ok(sim_result) => {
            if let Some(err) = sim_result.value.err {
                println!("   ⚠️  Simulation error: {:?}", err);
                if let Some(logs) = sim_result.value.logs {
                    println!("   Logs:");
                    for log in logs.iter().take(20) {
                        println!("      {}", log);
                    }
                }
                return Err(anyhow::anyhow!("Transaction simulation failed: {:?}", err));
            }
            println!("   ✅ Simulation successful");
            println!("      Compute units used: {:?}", sim_result.value.units_consumed);
        }
        Err(e) => {
            println!("   ⚠️  Simulation request failed: {:?}", e);
            return Err(anyhow::anyhow!("Failed to simulate transaction: {}", e));
        }
    }
    
    // Submit transaction
    println!("\n   📤 Submitting migration transaction...");
    let sig = submit_transaction(rpc, payer, &[migrate_ix]).await?;
    println!("   ✅ Migration transaction submitted: {}", sig);
    
    // Verify migration
    println!("\n   🔍 Verifying migration...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await; // Wait for confirmation
    
    let treasury_account_after = rpc.get_account(&treasury_address).await?;
    let is_migrated_after = treasury_account_after.data.len() >= expected_treasury_size;
    
    println!("   Treasury account size after: {} bytes", treasury_account_after.data.len());
    
    if is_migrated_after {
        println!("   ✅ Treasury migration successful! liquidity field is now available.");
        println!("   Note: The liquidity field will be initialized to 0.");
    } else {
        println!("   ⚠️  Migration may have failed. Treasury size didn't increase as expected.");
        println!("   Check transaction logs for details.");
    }
    
    Ok(())
}

/// Create a referral account to become a referrer.
async fn create_referral(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let referral_address = oil_api::state::referral_pda(payer.pubkey()).0;
    
    println!("🔗 Creating Referral Account");
    println!("   Address: {}", referral_address);
    println!("   Authority: {}", payer.pubkey());
    
    // Check if already exists.
    if let Ok(_) = rpc.get_account(&referral_address).await {
        println!("   Status: ✅ Already exists");
        return Ok(());
    }
    
    println!("\n📤 Submitting transaction...");
    let ix = oil_api::sdk::create_referral(payer.pubkey());
    submit_transaction(rpc, payer, &[ix]).await?;
    
    println!("✅ Referral account created! Share your referral link.");
    println!("   Referral pubkey: {}", payer.pubkey());
    Ok(())
}

/// Claim pending referral rewards.
async fn claim_referral_cmd(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let referral_address = oil_api::state::referral_pda(payer.pubkey()).0;
    
    // Get referral account.
    let referral = get_referral(rpc, payer.pubkey()).await?;
    
    println!("💰 Claiming Referral Rewards");
    println!("   Address: {}", referral_address);
    println!("   Pending SOL: {} SOL", lamports_to_sol(referral.pending_sol));
    println!("   Pending OIL: {} OIL", amount_to_ui_amount(referral.pending_oil, TOKEN_DECIMALS));
    
    if referral.pending_sol == 0 && referral.pending_oil == 0 {
        println!("   Status: ℹ️  No pending rewards to claim");
        return Ok(());
    }
    
    println!("\n📤 Submitting transaction...");
    // For CLI, signer and authority are the same (regular wallet)
    let ix = oil_api::sdk::claim_referral(payer.pubkey(), payer.pubkey());
    submit_transaction(rpc, payer, &[ix]).await?;
    
    println!("✅ Referral rewards claimed!");
    Ok(())
}

/// Show referral account info.
async fn log_referral(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let authority = std::env::var("AUTHORITY").unwrap_or(payer.pubkey().to_string());
    let authority = Pubkey::from_str(&authority).expect("Invalid AUTHORITY");
    let referral_address = oil_api::state::referral_pda(authority).0;
    
    println!("Looking up referral account for authority: {}", authority);
    println!("Referral PDA address: {}", referral_address);
    
    match get_referral(rpc, authority).await {
        Ok(referral) => {
            println!("\n✅ Referral Account Found:");
            println!("  address: {}", referral_address);
            println!("  authority: {}", referral.authority);
            println!("  total_referred: {}", referral.total_referred);
            println!("  total_sol_earned: {} SOL", lamports_to_sol(referral.total_sol_earned));
            println!("  total_oil_earned: {} OIL", amount_to_ui_amount(referral.total_oil_earned, TOKEN_DECIMALS));
            println!("  pending_sol: {} SOL", lamports_to_sol(referral.pending_sol));
            println!("  pending_oil: {} OIL", amount_to_ui_amount(referral.pending_oil, TOKEN_DECIMALS));
        }
        Err(e) => {
            println!("\n❌ Referral account not found");
            println!("  Authority: {}", authority);
            println!("  Error: {}", e);
            println!("\n💡 Tips:");
            println!("  - If you created the referral with a different wallet, set AUTHORITY env var:");
            println!("    AUTHORITY=<your-wallet-pubkey> cargo run --bin oil-cli");
            println!("  - Or run 'create_referral' to create one for this authority");
        }
    }
    Ok(())
}

async fn get_referral(rpc: &RpcClient, authority: Pubkey) -> Result<Referral, anyhow::Error> {
    let referral_address = oil_api::state::referral_pda(authority).0;
    let account = rpc.get_account(&referral_address).await
        .map_err(|e| anyhow::anyhow!("Failed to fetch referral account at {}: {}. Make sure you're using the correct AUTHORITY (the pubkey that created the referral account).", referral_address, e))?;
    
    if account.data.is_empty() {
        return Err(anyhow::anyhow!("Referral account at {} is empty", referral_address));
    }
    
    // Check account owner
    if account.owner != oil_api::ID {
        return Err(anyhow::anyhow!("Account at {} is not owned by OIL program (owner: {})", referral_address, account.owner));
    }
    
    // Parse referral account (try_from_bytes handles discriminator automatically)
    let referral = Referral::try_from_bytes(&account.data)
        .map_err(|e| anyhow::anyhow!("Failed to parse referral account: {}. Account size: {} bytes, owner: {}", e, account.data.len(), account.owner))?;
    Ok(*referral)
}

async fn log_miner(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let authority = std::env::var("MINER_AUTHORITY")
        .or_else(|_| std::env::var("AUTHORITY"))
        .unwrap_or_else(|_| payer.pubkey().to_string());
    let authority = Pubkey::from_str(&authority).map_err(|e| anyhow::anyhow!("Invalid authority pubkey: {}", e))?;
    let miner_address = oil_api::state::miner_pda(authority).0;
    let miner = get_miner(&rpc, authority).await?;
    println!("Miner");
    println!("  address: {}", miner_address);
    println!("  authority: {}", authority);
    println!("  deployed: {:?}", miner.deployed);
    println!("  cumulative: {:?}", miner.cumulative);
    println!("  block_rewards_sol: {} SOL", lamports_to_sol(miner.block_rewards_sol));
    println!(
        "  block_rewards_oil: {} OIL",
        amount_to_ui_amount(miner.block_rewards_oil, TOKEN_DECIMALS)
    );
    println!(
        "  block_refined_oil: {} OIL",
        amount_to_ui_amount(miner.block_refined_oil, TOKEN_DECIMALS)
    );
    println!("  round_id: {}", miner.round_id);
    println!("  checkpoint_id: {}", miner.checkpoint_id);
    println!(
        "  lifetime_rewards_sol: {} SOL",
        lamports_to_sol(miner.lifetime_rewards_sol)
    );
    println!(
        "  lifetime_rewards_oil: {} OIL",
        amount_to_ui_amount(miner.lifetime_rewards_oil, TOKEN_DECIMALS)
    );
    if miner.referrer != Pubkey::default() {
        println!("  referrer: {}", miner.referrer);
    } else {
        println!("  referrer: (none)");
    }
    if miner.lifetime_deployed > 0 {
        println!("  lifetime_deployed: {} OIL", amount_to_ui_amount(miner.lifetime_deployed, TOKEN_DECIMALS));
    } else {
        println!("  lifetime_deployed: (none)");
    }
    if miner.pooled_deployed > 0 {
        println!("  pooled_deployed: {} OIL", amount_to_ui_amount(miner.pooled_deployed, TOKEN_DECIMALS));
    } else {
        println!("  pooled_deployed: (none)");
    }
    println!("  total_stake_score: {}", miner.total_stake_score);
    
    println!("\nAuction-based mining (from Miner account)");
    println!(
        "  auction_rewards_oil: {} OIL",
        amount_to_ui_amount(miner.auction_rewards_oil, TOKEN_DECIMALS)
    );
    println!(
        "  auction_rewards_sol: {} SOL",
        lamports_to_sol(miner.auction_rewards_sol)
    );
    println!(
        "  auction_refined_oil: {} OIL",
        amount_to_ui_amount(miner.auction_refined_oil, TOKEN_DECIMALS)
    );
    println!(
        "  lifetime_rewards_oil: {} OIL",
        amount_to_ui_amount(miner.lifetime_rewards_oil, TOKEN_DECIMALS)
    );
    println!(
        "  lifetime_rewards_sol: {} SOL",
        lamports_to_sol(miner.lifetime_rewards_sol)
    );
    
    Ok(())
}

async fn log_clock(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let clock = get_clock(&rpc).await?;
    println!("Clock");
    println!("  slot: {}", clock.slot);
    println!("  epoch_start_timestamp: {}", clock.epoch_start_timestamp);
    println!("  epoch: {}", clock.epoch);
    println!("  leader_schedule_epoch: {}", clock.leader_schedule_epoch);
    println!("  unix_timestamp: {}", clock.unix_timestamp);
    Ok(())
}

async fn log_config(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let config = get_config(&rpc).await?;
    println!("Config");
    println!("  admin: {}", config.admin);
    println!("  barrel_authority: {}", config.barrel_authority);
    println!("  fee_collector: {}", config.fee_collector);
    println!("  swap_program: {}", config.swap_program);
    println!("  var_address: {}", config.var_address);
    println!("  admin_fee: {}", config.admin_fee);
    println!("  emission_week: {}", config.emission_week);
    println!("  last_emission_week_update: {}", config.last_emission_week_update);
    println!("  tge_timestamp: {}", config.tge_timestamp);
    if config.tge_timestamp > 0 {
        use std::time::{SystemTime, UNIX_EPOCH};
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        if now < config.tge_timestamp {
            let remaining = config.tge_timestamp - now;
            let hours = remaining / 3600;
            let minutes = (remaining % 3600) / 60;
            println!("  pre-mine status: 🟢 Active (TGE in {}h {}m)", hours, minutes);
        } else {
            println!("  pre-mine status: 🔴 Inactive (TGE has passed)");
        }
    } else {
        println!("  pre-mine status: 🔴 Disabled");
    }
    Ok(())
}

async fn log_well(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    println!("Well (Auction Wells)");
    println!("\n  Well Details (0-3):");
    
    for well_id in 0..4 {
        let (well_address, _) = oil_api::state::well_pda(well_id);
        
        match get_well(rpc, well_id).await {
            Ok(well) => {
                println!("    Well {}:", well_id);
                println!("      address: {}", well_address);
                println!("      well_id: {}", well.well_id);
                println!("      epoch_id: {}", well.epoch_id);
                println!("      current_bidder: {}", well.current_bidder);
                println!("      init_price: {} SOL", lamports_to_sol(well.init_price));
                println!("      mps: {} OIL/s", amount_to_ui_amount(well.mps, TOKEN_DECIMALS));
                println!("      epoch_start_time: {}", well.epoch_start_time);
                println!("      accumulated_oil: {} OIL", amount_to_ui_amount(well.accumulated_oil, TOKEN_DECIMALS));
                println!("      last_update_time: {}", well.last_update_time);
                println!("      halving_count: {}", well.halving_count);
                println!("      lifetime_oil_mined: {} OIL", amount_to_ui_amount(well.lifetime_oil_mined, TOKEN_DECIMALS));
                println!("      operator_total_oil_mined: {} OIL", amount_to_ui_amount(well.operator_total_oil_mined, TOKEN_DECIMALS));
            }
            Err(e) => {
                println!("    Well {}: not found (not initialized yet)", well_id);
                println!("      Error: {}", e);
            }
        }
    }
    Ok(())
}

async fn log_bid(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let authority = std::env::var("AUTHORITY").unwrap_or(payer.pubkey().to_string());
    let authority = Pubkey::from_str(&authority).expect("Invalid AUTHORITY");
    let well_id = std::env::var("WELL_ID")
        .or_else(|_| std::env::var("SQUARE_ID")) // Support old env var name
        .ok()
        .and_then(|s| u64::from_str(&s).ok())
        .unwrap_or(0);
    let epoch_id = std::env::var("EPOCH_ID")
        .ok()
        .and_then(|s| u64::from_str(&s).ok())
        .unwrap_or(0);
    
    let (bid_address, _) = oil_api::state::bid_pda(authority, well_id, epoch_id);
    
    match get_bid(rpc, authority, well_id, epoch_id).await {
        Ok(bid) => {
            println!("Bid (Auction Pool Contribution)");
            println!("  address: {}", bid_address);
            println!("  authority: {}", bid.authority);
            println!("  well_id: {}", bid.well_id);
            println!("  epoch_id: {}", bid.epoch_id);
            println!("  contribution: {} SOL", lamports_to_sol(bid.contribution));
            println!("  created_at: {}", bid.created_at);
        }
        Err(e) => {
            println!("Bid account not found");
            println!("  Authority: {}", authority);
            println!("  Well ID: {}", well_id);
            println!("  Epoch ID: {}", epoch_id);
            println!("  Error: {}", e);
        }
    }
    Ok(())
}

async fn log_auction(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let auction_address = oil_api::state::auction_pda().0;
    
    match get_auction(rpc).await {
        Ok(auction) => {
            println!("Auction (Configuration)");
            println!("  address: {}", auction_address);
            
            // Time-based halving information
            println!("\n  Time-Based Halving:");
            println!("    halving_period_seconds: {} ({} days)", 
                auction.halving_period_seconds,
                auction.halving_period_seconds / (24 * 60 * 60)
            );
            println!("    last_halving_time: {}", auction.last_halving_time);
            if auction.last_halving_time > 0 {
                println!("      (Unix timestamp: {})", auction.last_halving_time);
            } else {
                println!("      (Not set - will use initialization time)");
            }
            
            let next_halving_time = auction.next_halving_time();
            println!("    next_halving_time: {}", next_halving_time);
            if next_halving_time > 0 {
                println!("      (Unix timestamp: {})", next_halving_time);
            }
            
            // Calculate time remaining until next halving
            let current_time = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let time_remaining = if next_halving_time > current_time {
                next_halving_time - current_time
            } else {
                0
            };
            
            if time_remaining > 0 {
                let days = time_remaining / (24 * 60 * 60);
                let hours = (time_remaining % (24 * 60 * 60)) / (60 * 60);
                let minutes = (time_remaining % (60 * 60)) / 60;
                println!("    time_remaining_until_halving: {}d {}h {}m ({} seconds)", 
                    days, hours, minutes, time_remaining
                );
            } else {
                println!("    time_remaining_until_halving: Now (halving should be applied)");
            }
            
            // Auction configuration
            println!("\n  Auction Configuration:");
            println!("    auction_duration_seconds: {} ({} hours)",
                auction.auction_duration_seconds,
                auction.auction_duration_seconds / 3600
            );
            // Note: min_pool_contribution was removed and is now buffer_b
            
            println!("\n  Base Mining Rates (OIL/s):");
            for i in 0..4 {
                println!("    Well {}: {} OIL/s", 
                    i, 
                    amount_to_ui_amount(auction.base_mining_rates[i], TOKEN_DECIMALS)
                );
            }
            println!("\n  Starting Prices (SOL):");
            for i in 0..4 {
                println!("    Well {}: {} SOL", 
                    i, 
                    lamports_to_sol(auction.starting_prices[i])
                );
            }
            
            // Note about repurposed fields (for reference)
            println!("\n  Note: halving_period_seconds and last_halving_time are repurposed fields");
            println!("    (kept for account layout compatibility, values may be 0)");
        }
        Err(e) => {
            println!("Auction account not found (not initialized yet)");
            println!("  Error: {}", e);
        }
    }
    Ok(())
}

async fn place_bid(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
) -> Result<(), anyhow::Error> {
    let well_id = std::env::var("WELL_ID")
        .or_else(|_| std::env::var("SQUARE_ID")) // Support old env var name
        .ok()
        .and_then(|s| u64::from_str(&s).ok())
        .unwrap_or(0);
    
    // Get config to fetch fee_collector
    let config = get_config(rpc).await?;
    
    // Get well to find current_bidder and epoch_id
    let well = get_well(rpc, well_id).await?;
    
    // Derive previous owner miner if previous owner exists
    let (previous_owner_miner, previous_owner) = if well.current_bidder != Pubkey::default() {
        let (miner_pda, _) = oil_api::state::miner_pda(well.current_bidder);
        (Some(miner_pda), Some(well.current_bidder))
    } else {
        (None, None)
    };
    
    println!("Placing bid on Well {}", well_id);
    println!("  Current epoch: {}", well.epoch_id);
    println!("  Current bidder: {}", well.current_bidder);
    if let Some(prev_owner) = previous_owner {
        println!("  Previous owner: {}", prev_owner);
    }
    
    // Build and submit transaction
    let ix = oil_api::sdk::place_bid(
        payer.pubkey(),        // signer
        payer.pubkey(),        // authority (same as signer for CLI)
        well_id,               // square_id
        config.fee_collector,  // fee_collector
        previous_owner_miner, // previous_owner_miner
        previous_owner,        // previous_owner
        None,                  // referrer (no referrer for CLI bids)
    );
    
    submit_transaction(rpc, payer, &[ix]).await?;
    println!("✅ Bid placed successfully!");
    Ok(())
}

async fn log_var(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let config = get_config(&rpc).await?;
    let clock = get_clock(&rpc).await?;
    let var_address = config.var_address;
    
    match get_var(&rpc, var_address).await {
        Ok(var) => {
            println!("Var");
            println!("  address: {}", var_address);
            println!("  authority: {}", var.authority);
            println!("  id: {}", var.id);
            println!("  provider: {}", var.provider);
            println!("  commit: {:?}", var.commit);
            println!("  seed: {:?}", var.seed);
            println!("  slot_hash: {:?}", var.slot_hash);
            println!("  value: {:?}", var.value);
            println!("  samples: {}", var.samples);
            println!("  is_auto: {}", var.is_auto);
            println!("  start_at: {}", var.start_at);
            println!("  end_at: {}", var.end_at);
            println!("  current_slot: {}", clock.slot);
            println!("  end_at > current_slot: {}", var.end_at > clock.slot);
            println!("  slot_hash ready: {}", var.slot_hash != [0; 32]);
            println!("  seed ready: {}", var.seed != [0; 32]);
            println!("  value ready: {}", var.value != [0; 32]);
            println!("  fully ready: {}", var.slot_hash != [0; 32] && var.seed != [0; 32] && var.value != [0; 32]);
        }
        Err(e) => {
            println!("Var");
            println!("  address: {}", var_address);
            println!("  error: {}", e);
            println!("  (Var account may not exist or is closed)");
        }
    }
    Ok(())
}

async fn log_board(rpc: &RpcClient) -> Result<(), anyhow::Error> {
    let board = get_board(&rpc).await?;
    let clock = get_clock(&rpc).await?;
    print_board(board, &clock);
    Ok(())
}

fn print_board(board: Board, clock: &Clock) {
    let current_slot = clock.slot;
    println!("Board");
    println!("  Id: {:?}", board.round_id);
    println!("  Start slot: {}", board.start_slot);
    println!("  End slot: {}", board.end_slot);
    println!(
        "  Time remaining: {} sec",
        (board.end_slot.saturating_sub(current_slot) as f64) * 0.4
    );
}

async fn get_automation(rpc: &RpcClient, address: Pubkey) -> Result<Automation, anyhow::Error> {
    let account = rpc.get_account(&address).await?;
    let automation = Automation::try_from_bytes(&account.data)?;
    Ok(*automation)
}

async fn get_automations(rpc: &RpcClient) -> Result<Vec<(Pubkey, Automation)>, anyhow::Error> {
    const REGOLITH_EXECUTOR: Pubkey = pubkey!("BoT3qYmE6xePWPU96Kf2QeuJr1pDgQ3gLWbA6kSyjzV");
    let filter = RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
        56,
        &REGOLITH_EXECUTOR.to_bytes(),
    ));
    let automations = get_program_accounts::<Automation>(rpc, oil_api::ID, vec![filter]).await?;
    Ok(automations)
}

// async fn get_meteora_pool(rpc: &RpcClient, address: Pubkey) -> Result<Pool, anyhow::Error> {
//     let data = rpc.get_account_data(&address).await?;
//     let pool = Pool::from_bytes(&data)?;
//     Ok(pool)
// }

// async fn get_meteora_vault(rpc: &RpcClient, address: Pubkey) -> Result<Vault, anyhow::Error> {
//     let data = rpc.get_account_data(&address).await?;
//     let vault = Vault::from_bytes(&data)?;
//     Ok(vault)
// }

async fn get_board(rpc: &RpcClient) -> Result<Board, anyhow::Error> {
    let board_pda = oil_api::state::board_pda();
    let account = rpc.get_account(&board_pda.0).await?;
    let board = Board::try_from_bytes(&account.data)?;
    Ok(*board)
}

async fn get_var(rpc: &RpcClient, address: Pubkey) -> Result<Var, anyhow::Error> {
    let account = rpc.get_account(&address).await?;
    let var = Var::try_from_bytes(&account.data)?;
    Ok(*var)
}

async fn get_round(rpc: &RpcClient, id: u64) -> Result<Round, anyhow::Error> {
    let round_pda = oil_api::state::round_pda(id);
    let account = rpc.get_account(&round_pda.0).await?;
    let round = Round::try_from_bytes(&account.data)?;
    Ok(*round)
}

async fn get_treasury(rpc: &RpcClient) -> Result<Treasury, anyhow::Error> {
    let treasury_pda = oil_api::state::treasury_pda();
    let account = rpc.get_account(&treasury_pda.0).await?;
    let treasury = Treasury::try_from_bytes(&account.data)?;
    Ok(*treasury)
}

async fn get_config(rpc: &RpcClient) -> Result<Config, anyhow::Error> {
    let config_pda = oil_api::state::config_pda();
    let account = rpc.get_account(&config_pda.0).await?;
    let config = Config::try_from_bytes(&account.data)?;
    Ok(*config)
}

async fn get_miner(rpc: &RpcClient, authority: Pubkey) -> Result<Miner, anyhow::Error> {
    let miner_pda = oil_api::state::miner_pda(authority);
    let account = rpc.get_account(&miner_pda.0)
        .await
        .map_err(|e| anyhow::anyhow!("Miner account not found for authority {} (address: {}): {}", authority, miner_pda.0, e))?;
    let miner = Miner::try_from_bytes(&account.data)
        .map_err(|e| anyhow::anyhow!("Failed to deserialize miner account: {}", e))?;
    Ok(*miner)
}

async fn get_clock(rpc: &RpcClient) -> Result<Clock, anyhow::Error> {
    let data = rpc.get_account_data(&solana_sdk::sysvar::clock::ID).await?;
    let clock = bincode::deserialize::<Clock>(&data)?;
    Ok(clock)
}

async fn get_stake(rpc: &RpcClient, authority: Pubkey) -> Result<Stake, anyhow::Error> {
    let stake_pda = oil_api::state::stake_pda(authority);
    let account = rpc.get_account(&stake_pda.0).await?;
    let stake = Stake::try_from_bytes(&account.data)?;
    Ok(*stake)
}

async fn get_well(rpc: &RpcClient, well_id: u64) -> Result<Well, anyhow::Error> {
    let (well_pda, _) = oil_api::state::well_pda(well_id);
    let account = rpc.get_account(&well_pda).await?;
    if account.data.is_empty() {
        return Err(anyhow::anyhow!("Well account not found (not initialized yet)"));
    }
    let well = Well::try_from_bytes(&account.data)?;
    Ok(*well)
}

async fn get_bid(rpc: &RpcClient, authority: Pubkey, well_id: u64, epoch_id: u64) -> Result<Bid, anyhow::Error> {
    let (bid_pda, _) = oil_api::state::bid_pda(authority, well_id, epoch_id);
    let account = rpc.get_account(&bid_pda).await?;
    if account.data.is_empty() {
        return Err(anyhow::anyhow!("Bid account not found"));
    }
    let bid = Bid::try_from_bytes(&account.data)?;
    Ok(*bid)
}

async fn get_auction(rpc: &RpcClient) -> Result<Auction, anyhow::Error> {
    let auction_pda = oil_api::state::auction_pda();
    let account = rpc.get_account(&auction_pda.0).await?;
    if account.data.is_empty() {
        return Err(anyhow::anyhow!("Auction account not found (not initialized yet)"));
    }
    let auction = Auction::try_from_bytes(&account.data)?;
    Ok(*auction)
}

async fn get_rounds(rpc: &RpcClient) -> Result<Vec<(Pubkey, Round)>, anyhow::Error> {
    let rounds = get_program_accounts::<Round>(rpc, oil_api::ID, vec![]).await?;
    Ok(rounds)
}

#[allow(dead_code)]
async fn get_miners(rpc: &RpcClient) -> Result<Vec<(Pubkey, Miner)>, anyhow::Error> {
    let miners = get_program_accounts::<Miner>(rpc, oil_api::ID, vec![]).await?;
    Ok(miners)
}

async fn get_miners_participating(
    rpc: &RpcClient,
    round_id: u64,
) -> Result<Vec<(Pubkey, Miner)>, anyhow::Error> {
    let filter = RpcFilterType::Memcmp(Memcmp::new_base58_encoded(512, &round_id.to_le_bytes()));
    let miners = get_program_accounts::<Miner>(rpc, oil_api::ID, vec![filter]).await?;
    Ok(miners)
}

// fn get_winning_square(slot_hash: &[u8]) -> u64 {
//     // Use slot hash to generate a random u64
//     let r1 = u64::from_le_bytes(slot_hash[0..8].try_into().unwrap());
//     let r2 = u64::from_le_bytes(slot_hash[8..16].try_into().unwrap());
//     let r3 = u64::from_le_bytes(slot_hash[16..24].try_into().unwrap());
//     let r4 = u64::from_le_bytes(slot_hash[24..32].try_into().unwrap());
//     let r = r1 ^ r2 ^ r3 ^ r4;
//     // Returns a value in the range [0, 24] inclusive
//     r % 25
// }

#[allow(dead_code)]
async fn simulate_transaction(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
    instructions: &[solana_sdk::instruction::Instruction],
) {
    let blockhash = rpc.get_latest_blockhash().await.unwrap();
    let x = rpc
        .simulate_transaction(&Transaction::new_signed_with_payer(
            instructions,
            Some(&payer.pubkey()),
            &[payer],
            blockhash,
        ))
        .await;
    println!("Simulation result: {:?}", x);
}

#[allow(dead_code)]
async fn simulate_transaction_with_address_lookup_tables(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
    instructions: &[solana_sdk::instruction::Instruction],
    address_lookup_table_accounts: Vec<AddressLookupTableAccount>,
) {
    let blockhash = rpc.get_latest_blockhash().await.unwrap();
    let tx = VersionedTransaction {
        signatures: vec![Signature::default()],
        message: VersionedMessage::V0(
            Message::try_compile(
                &payer.pubkey(),
                instructions,
                &address_lookup_table_accounts,
                blockhash,
            )
            .unwrap(),
        ),
    };
    let s = tx.sanitize();
    println!("Sanitize result: {:?}", s);
    s.unwrap();
    let x = rpc.simulate_transaction(&tx).await;
    println!("Simulation result: {:?}", x);
}

#[allow(unused)]
async fn submit_transaction_batches(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
    mut ixs: Vec<solana_sdk::instruction::Instruction>,
    batch_size: usize,
) -> Result<(), anyhow::Error> {
    // Batch and submit the instructions.
    while !ixs.is_empty() {
        let batch = ixs
            .drain(..std::cmp::min(batch_size, ixs.len()))
            .collect::<Vec<Instruction>>();
        submit_transaction_no_confirm(rpc, payer, &batch).await?;
    }
    Ok(())
}

#[allow(unused)]
async fn simulate_transaction_batches(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
    mut ixs: Vec<solana_sdk::instruction::Instruction>,
    batch_size: usize,
) -> Result<(), anyhow::Error> {
    // Batch and submit the instructions.
    while !ixs.is_empty() {
        let batch = ixs
            .drain(..std::cmp::min(batch_size, ixs.len()))
            .collect::<Vec<Instruction>>();
        simulate_transaction(rpc, payer, &batch).await;
    }
    Ok(())
}

async fn submit_transaction(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
    instructions: &[solana_sdk::instruction::Instruction],
) -> Result<solana_sdk::signature::Signature, anyhow::Error> {
    let blockhash = rpc.get_latest_blockhash().await?;
    let mut all_instructions = vec![
        ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
        ComputeBudgetInstruction::set_compute_unit_price(1_000_000),
    ];
    all_instructions.extend_from_slice(instructions);
    let transaction = Transaction::new_signed_with_payer(
        &all_instructions,
        Some(&payer.pubkey()),
        &[payer],
        blockhash,
    );

    // Try send_and_confirm_transaction first, fall back to send_transaction if not supported
    match rpc.send_and_confirm_transaction(&transaction).await {
        Ok(signature) => {
            println!("Transaction submitted: {:?}", signature);
            Ok(signature)
        }
        Err(e) => {
            // If RPC doesn't support sendAndConfirmTransaction, try send_transaction instead
            if let solana_client::client_error::ClientErrorKind::RpcError(rpc_err) = &e.kind {
                // Check if error message contains "UNKNOWN" or error code is -32601 (method not found)
                let error_str = format!("{:?}", rpc_err);
                if error_str.contains("UNKNOWN") || error_str.contains("-32601") {
                    // Method not found - try send_transaction instead
                    println!("RPC doesn't support sendAndConfirmTransaction, using send_transaction instead...");
                    let signature = rpc.send_transaction(&transaction).await?;
                    println!("Transaction submitted (not confirmed): {:?}", signature);
                    println!("Please check the transaction status manually");
                    return Ok(signature);
                }
            }
            println!("Error submitting transaction: {:?}", e);
            Err(e.into())
        }
    }
}

async fn submit_transaction_no_confirm(
    rpc: &RpcClient,
    payer: &solana_sdk::signer::keypair::Keypair,
    instructions: &[solana_sdk::instruction::Instruction],
) -> Result<solana_sdk::signature::Signature, anyhow::Error> {
    let blockhash = rpc.get_latest_blockhash().await?;
    let mut all_instructions = vec![
        ComputeBudgetInstruction::set_compute_unit_limit(1_400_000),
        ComputeBudgetInstruction::set_compute_unit_price(1_000_000),
    ];
    all_instructions.extend_from_slice(instructions);
    let transaction = Transaction::new_signed_with_payer(
        &all_instructions,
        Some(&payer.pubkey()),
        &[payer],
        blockhash,
    );

    match rpc.send_transaction(&transaction).await {
        Ok(signature) => {
            println!("Transaction submitted: {:?}", signature);
            Ok(signature)
        }
        Err(e) => {
            println!("Error submitting transaction: {:?}", e);
            Err(e.into())
        }
    }
}

pub async fn get_program_accounts<T>(
    client: &RpcClient,
    program_id: Pubkey,
    filters: Vec<RpcFilterType>,
) -> Result<Vec<(Pubkey, T)>, anyhow::Error>
where
    T: AccountDeserialize + Discriminator + Clone,
{
    let mut all_filters = vec![RpcFilterType::Memcmp(Memcmp::new_base58_encoded(
        0,
        &T::discriminator().to_le_bytes(),
    ))];
    all_filters.extend(filters);
    let result = client
        .get_program_accounts_with_config(
            &program_id,
            RpcProgramAccountsConfig {
                filters: Some(all_filters),
                account_config: RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64),
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .await;

    match result {
        Ok(accounts) => {
            let accounts = accounts
                .into_iter()
                .filter_map(|(pubkey, account)| {
                    if let Ok(account) = T::try_from_bytes(&account.data) {
                        Some((pubkey, account.clone()))
                    } else {
                        None
                    }
                })
                .collect();
            Ok(accounts)
        }
        Err(err) => match err.kind {
            ClientErrorKind::Reqwest(err) => {
                if let Some(status_code) = err.status() {
                    if status_code == StatusCode::GONE {
                        panic!(
                                "\n{} Your RPC provider does not support the getProgramAccounts endpoint, needed to execute this command. Please use a different RPC provider.\n",
                                "ERROR"
                            );
                    }
                }
                return Err(anyhow::anyhow!("Failed to get program accounts: {}", err));
            }
            _ => return Err(anyhow::anyhow!("Failed to get program accounts: {}", err)),
        },
    }
}
