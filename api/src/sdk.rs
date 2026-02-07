use solana_program::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address;
use steel::*;

use crate::{
    consts::{AUCTION, BOARD, MINT_ADDRESS, SOL_MINT, TREASURY_ADDRESS},
    instruction::{self, *},
    state::*,
};

pub fn log(signer: Pubkey, msg: &[u8]) -> Instruction {
    let mut data = Log {}.to_bytes();
    data.extend_from_slice(msg);
    Instruction {
        program_id: crate::ID,
        accounts: vec![AccountMeta::new(signer, true)],
        data: data,
    }
}

pub fn program_log(accounts: &[AccountInfo], msg: &[u8]) -> Result<(), ProgramError> {
    // Derive Board PDA to use as signer for log instruction
    let (board_address, _) = board_pda();
    invoke_signed(&log(board_address, msg), accounts, &crate::ID, &[BOARD])
}

/// Log event for auction-based instructions (uses Auction PDA instead of Board)
pub fn auction_program_log(accounts: &[AccountInfo], msg: &[u8]) -> Result<(), ProgramError> {
    // Derive Auction PDA to use as signer for log instruction
    let (auction_address, _) = auction_pda();
    invoke_signed(&log(auction_address, msg), accounts, &crate::ID, &[AUCTION])
}

// let [signer_info, board_info, config_info, mint_info, treasury_info, treasury_tokens_info, system_program, token_program, associated_token_program] = accounts else {
// pub fn initialize(
//     signer: Pubkey,
//     barrel_authority: Pubkey,
//     fee_collector: Pubkey,
//     swap_program: Pubkey,
//     var_address: Pubkey,
//     admin_fee: u64,
// ) -> Instruction {
//     let board_address = board_pda().0;
//     let config_address = config_pda().0;
//     let mint_address = MINT_ADDRESS;
//     let treasury_address = TREASURY_ADDRESS;
//     let treasury_tokens_address = treasury_tokens_address();
//     Instruction {
//         program_id: crate::ID,
//         accounts: vec![
//             AccountMeta::new(signer, true),
//             AccountMeta::new(board_address, false),
//             AccountMeta::new(config_address, false),
//             AccountMeta::new(mint_address, false),
//             AccountMeta::new(treasury_address, false),
//             AccountMeta::new(treasury_tokens_address, false),
//             AccountMeta::new_readonly(system_program::ID, false),
//             AccountMeta::new_readonly(spl_token::ID, false),
//             AccountMeta::new_readonly(spl_associated_token_account::ID, false),
//         ],
//         data: Initialize {
//             barrel_authority: barrel_authority.to_bytes(),
//             fee_collector: fee_collector.to_bytes(),
//             swap_program: swap_program.to_bytes(),
//             var_address: var_address.to_bytes(),
//             admin_fee: admin_fee.to_le_bytes(),
//         }
//         .to_bytes(),
//     }
// }

// let [signer_info, automation_info, executor_info, miner_info, system_program] = accounts else {

/// Set up automation for a miner. If the miner doesn't exist yet, pass a referrer to set it.
/// If a referrer is provided and the miner is new, the referral account must be included.
pub fn automate(
    signer: Pubkey,
    authority: Pubkey,
    amount: u64,
    deposit: u64,
    executor: Pubkey,
    fee: u64,
    mask: u64,
    strategy: u8,
    reload: bool,
    referrer: Option<Pubkey>,
    pooled: bool,
    is_new_miner: bool,
) -> Instruction {
    let automation_address = automation_pda(authority).0;
    let miner_address = miner_pda(authority).0;
    let config_address = config_pda().0;
    let referrer_pk = referrer.unwrap_or(Pubkey::default());
    
    let mut accounts = vec![
            AccountMeta::new(signer, true), // 0: signer (payer)
            AccountMeta::new(authority, false), // 1: authority (user's wallet)
            AccountMeta::new(automation_address, false), // 2: automation
            AccountMeta::new(executor, false), // 3: executor
            AccountMeta::new(miner_address, false), // 4: miner
            AccountMeta::new_readonly(system_program::ID, false), // 5: system_program
            AccountMeta::new_readonly(crate::ID, false), // 6: oil_program
            AccountMeta::new_readonly(config_address, false), // 7: config
    ];
    
    // Token accounts (user_wrapped_sol, automation_wrapped_sol, token_program, program_signer (optional), payer (optional), mint, ata_program)
    // These are added by the client, not here in the SDK
    
    // Add referral account if referrer is provided and miner is new (for incrementing total_referred)
    if is_new_miner && referrer.is_some() && referrer_pk != Pubkey::default() {
        let referral_address = referral_pda(referrer_pk).0;
        accounts.push(AccountMeta::new(referral_address, false));
    }
    
    Instruction {
        program_id: crate::ID,
        accounts,
        data: Automate {
            amount: amount.to_le_bytes(),
            deposit: deposit.to_le_bytes(),
            fee: fee.to_le_bytes(),
            mask: mask.to_le_bytes(),
            strategy: strategy as u8,
            reload: (reload as u64).to_le_bytes(),
            referrer: referrer_pk.to_bytes(),
            pooled: pooled as u8,
        }
        .to_bytes(),
    }
}

/// Claim SOL rewards with single-tier referral system.
/// 
/// If the miner has a referrer, 1.0% of the claim goes to the referrer.
/// 
/// Account structure:
/// - Base: signer, miner, system_program
/// - If miner has referrer (required): [miner_referrer, referral_referrer]
pub fn claim_sol(
    signer: Pubkey,
    referrer_miner: Option<Pubkey>, // Referrer's miner PDA (if miner has referrer)
    referrer_referral: Option<Pubkey>, // Referrer's referral PDA (if miner has referrer)
) -> Instruction {
    let miner_address = miner_pda(signer).0;
    
    let mut accounts = vec![
        AccountMeta::new(signer, true),
        AccountMeta::new(miner_address, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];
    
    // Add referrer accounts if provided (required if miner has referrer)
    if let (Some(miner_pubkey), Some(referral_pubkey)) = (referrer_miner, referrer_referral) {
        accounts.push(AccountMeta::new(miner_pubkey, false));
        accounts.push(AccountMeta::new(referral_pubkey, false));
    }
    
    Instruction {
        program_id: crate::ID,
        accounts,
        data: ClaimSOL {}.to_bytes(),
    }
}

// let [signer_info, miner_info, mint_info, recipient_info, treasury_info, treasury_tokens_info, system_program, token_program, associated_token_program] =

/// Claim OIL rewards with single-tier referral system.
/// 
/// If the miner has a referrer, 1.0% of the claim goes to the referrer.
/// 
/// Account structure:
/// - Base: signer, miner, mint, recipient, treasury, treasury_tokens, system_program, token_program, associated_token_program
/// - If miner has referrer (required): [miner_referrer, referral_referrer, referral_referrer_oil_ata]
pub fn claim_oil(
    signer: Pubkey,
    referrer_miner: Option<Pubkey>, // Referrer's miner PDA (if miner has referrer)
    referrer_referral: Option<Pubkey>, // Referrer's referral PDA (if miner has referrer)
    referrer_referral_oil_ata: Option<Pubkey>, // Referrer's referral OIL ATA (if miner has referrer)
) -> Instruction {
    let miner_address = miner_pda(signer).0;
    let treasury_address = treasury_pda().0;
    let treasury_tokens_address = get_associated_token_address(&treasury_address, &MINT_ADDRESS);
    let recipient_address = get_associated_token_address(&signer, &MINT_ADDRESS);
    
    let mut accounts = vec![
        AccountMeta::new(signer, true),
        AccountMeta::new(miner_address, false),
        AccountMeta::new(MINT_ADDRESS, false),
        AccountMeta::new(recipient_address, false),
        AccountMeta::new(treasury_address, false),
        AccountMeta::new(treasury_tokens_address, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(spl_associated_token_account::ID, false),
    ];
    
    // Add referrer accounts if provided (required if miner has referrer)
    if let (Some(miner_pubkey), Some(referral_pubkey), Some(oil_ata_pubkey)) = 
        (referrer_miner, referrer_referral, referrer_referral_oil_ata) {
        accounts.push(AccountMeta::new(miner_pubkey, false));
        accounts.push(AccountMeta::new(referral_pubkey, false));
        accounts.push(AccountMeta::new(oil_ata_pubkey, false));
    }
    
    Instruction {
        program_id: crate::ID,
        accounts,
        data: ClaimOIL {}.to_bytes(),
    }
}


pub fn close(signer: Pubkey, round_id: u64, rent_payer: Pubkey) -> Instruction {
    let board_address = board_pda().0;
    let round_address = round_pda(round_id).0;
    let treasury_address = TREASURY_ADDRESS;
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(board_address, false),
            AccountMeta::new(rent_payer, false),
            AccountMeta::new(round_address, false),
            AccountMeta::new(treasury_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: Close {}.to_bytes(),
    }
}

/// Deploy SOL to prospect on squares.
/// 
/// This function uses native SOL transfers and is used for:
/// - Regular wallets (signer == authority)
/// - Automations (bot-executed deploys, signer != authority, using native SOL from automation account balance)
/// 
/// Pass a referrer pubkey for new miners to set up referral.
/// Set `pooled` to true to join the mining pool (rewards shared proportionally).
pub fn deploy(
    signer: Pubkey,
    authority: Pubkey,
    amount: u64,
    round_id: u64,
    squares: [bool; 25],
    referrer: Option<Pubkey>,
    pooled: bool,
) -> Instruction {
    let automation_address = automation_pda(authority).0;
    let board_address = board_pda().0;
    let miner_address = miner_pda(authority).0;
    let round_address = round_pda(round_id).0;
    let entropy_var_address = entropy_rng_api::state::var_pda(board_address, 0).0;

    let mut mask: u32 = 0;
    for (i, &square) in squares.iter().enumerate() {
        if square {
            mask |= 1 << i;
        }
    }
    
    let referrer_pubkey = referrer.unwrap_or(Pubkey::default());
    let referrer_bytes = referrer_pubkey.to_bytes();
    // Match program logic: has_referrer = referrer != Pubkey::default() && referrer != authority
    let has_referrer = referrer_pubkey != Pubkey::default() && referrer_pubkey != authority;

    // Build accounts list - must match program structure:
    // Oil accounts: base (8) + optional referral (1) = 8-9
    // Entropy accounts: var + program = 2 (always exactly 2)
    let mut accounts = vec![
        AccountMeta::new(signer, true), // 0: signer
        AccountMeta::new(authority, false), // 1: authority
        AccountMeta::new(automation_address, false), // 2: automation
        AccountMeta::new(board_address, false), // 3: board
        AccountMeta::new(miner_address, false), // 4: miner
        AccountMeta::new(round_address, false), // 5: round
        AccountMeta::new_readonly(system_program::ID, false), // 6: system_program
        AccountMeta::new_readonly(crate::ID, false), // 7: oil_program
    ];
    
    // Add referral account if referrer is provided and not equal to authority (matches program logic)
    if has_referrer {
        let referral_address = referral_pda(referrer_pubkey).0;
        accounts.push(AccountMeta::new(referral_address, false)); // referral (optional, in oil_accounts)
    }
    
    // Entropy accounts (always exactly 2, come after all oil_accounts)
    accounts.push(AccountMeta::new(entropy_var_address, false)); // entropy_var
    accounts.push(AccountMeta::new_readonly(entropy_rng_api::ID, false)); // entropy_program

    Instruction {
        program_id: crate::ID,
        accounts,
        data: Deploy {
            amount: amount.to_le_bytes(),
            squares: mask.to_le_bytes(),
            referrer: referrer_bytes,
            pooled: if pooled { 1 } else { 0 },
        }
        .to_bytes(),
    }
}


// let [pool, user_source_token, user_destination_token, a_vault, b_vault, a_token_vault, b_token_vault, a_vault_lp_mint, b_vault_lp_mint, a_vault_lp, b_vault_lp, protocol_token_fee, user_key, vault_program, token_program] =

pub fn wrap(signer: Pubkey, use_liquidity: bool, amount: u64) -> Instruction {
    let config_address = config_pda().0;
    let treasury_address = TREASURY_ADDRESS;
    let treasury_sol_address = get_associated_token_address(&treasury_address, &SOL_MINT);
    let data = Wrap {
        use_liquidity: if use_liquidity { 1 } else { 0 },
        amount: amount.to_le_bytes(),
    }
    .to_bytes();
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new_readonly(config_address, false),
            AccountMeta::new(treasury_address, false),
            AccountMeta::new(treasury_sol_address, false),
            AccountMeta::new_readonly(solana_program::system_program::ID, false),
        ],
        data,
    }
}

pub fn buyback(signer: Pubkey, swap_accounts: &[AccountMeta], swap_data: &[u8]) -> Instruction {
    let board_address = board_pda().0;
    let mint_address = MINT_ADDRESS;
    let treasury_address = TREASURY_ADDRESS;
    let treasury_oil_address = get_associated_token_address(&treasury_address, &MINT_ADDRESS);
    let treasury_sol_address = get_associated_token_address(&treasury_address, &SOL_MINT);
    let mut accounts = vec![
        AccountMeta::new(signer, true),
        AccountMeta::new(board_address, false),
        AccountMeta::new(mint_address, false),
        AccountMeta::new(treasury_address, false),
        AccountMeta::new(treasury_oil_address, false),
        AccountMeta::new(treasury_sol_address, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];
    for account in swap_accounts.iter() {
        let mut acc_clone = account.clone();
        acc_clone.is_signer = false;
        accounts.push(acc_clone);
    }
    let mut data = Buyback {}.to_bytes();
    data.extend_from_slice(swap_data);
    Instruction {
        program_id: crate::ID,
        accounts,
        data,
    }
}

pub fn barrel(signer: Pubkey, amount: u64) -> Instruction {
    let board_address = board_pda().0;
    let mint_address = MINT_ADDRESS;
    let treasury_address = TREASURY_ADDRESS;
    let sender_oil_address = get_associated_token_address(&signer, &MINT_ADDRESS);
    let treasury_oil_address = get_associated_token_address(&treasury_address, &MINT_ADDRESS);
    let data = Barrel {
        amount: amount.to_le_bytes(),
    }
    .to_bytes();
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(sender_oil_address, false),
            AccountMeta::new(board_address, false),
            AccountMeta::new(mint_address, false),
            AccountMeta::new(treasury_address, false),
            AccountMeta::new(treasury_oil_address, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(crate::ID, false),
        ],
        data,
    }
}


// let [signer_info, board_info, config_info, fee_collector_info, mint_info, round_info, round_next_info, top_miner_info, treasury_info, treasury_tokens_info, system_program, token_program, oil_program, slot_hashes_sysvar] =

pub fn reset(
    signer: Pubkey,
    fee_collector: Pubkey,
    round_id: u64,
    top_miner: Pubkey,
    var_address: Pubkey,
) -> Instruction {
    reset_with_miners(signer, fee_collector, round_id, top_miner, var_address, &[])
}

pub fn reset_with_miners(
    signer: Pubkey,
    fee_collector: Pubkey,
    round_id: u64,
    top_miner: Pubkey,
    var_address: Pubkey,
    miner_accounts: &[Pubkey],
) -> Instruction {
    let board_address = board_pda().0;
    let config_address = config_pda().0;
    let mint_address = MINT_ADDRESS;
    let round_address = round_pda(round_id).0;
    let round_next_address = round_pda(round_id + 1).0;
    let top_miner_address = miner_pda(top_miner).0;
    let treasury_address = TREASURY_ADDRESS;
    let treasury_tokens_address = treasury_tokens_address();
    let pool_address = pool_pda().0;
    let mint_authority_address = oil_mint_api::state::authority_pda().0;
    let mut reset_instruction = Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(board_address, false),
            AccountMeta::new(config_address, false),
            AccountMeta::new(fee_collector, false),
            AccountMeta::new(mint_address, false),
            AccountMeta::new(round_address, false),
            AccountMeta::new(round_next_address, false),
            AccountMeta::new(top_miner_address, false),
            AccountMeta::new(treasury_address, false),
            AccountMeta::new(pool_address, false),
            AccountMeta::new(treasury_tokens_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(crate::ID, false),
            AccountMeta::new_readonly(sysvar::slot_hashes::ID, false),
            AccountMeta::new_readonly(SOL_MINT, false),
            // Entropy accounts (these are in "other_accounts" after the split)
            AccountMeta::new(var_address, false),
            AccountMeta::new_readonly(entropy_rng_api::ID, false),
            // Mint accounts.
            AccountMeta::new(mint_authority_address, false),
            AccountMeta::new_readonly(oil_mint_api::ID, false),
        ],
        data: Reset {}.to_bytes(),
    };
    
    // Add miner accounts for seeker rewards (optional)
    for miner_pubkey in miner_accounts {
        reset_instruction.accounts.push(AccountMeta::new(
            miner_pda(*miner_pubkey).0,
            false,
        ));
    }
    
    reset_instruction
}
    
// let [signer_info, automation_info, board_info, miner_info, round_info, treasury_info, system_program] =

pub fn checkpoint(signer: Pubkey, authority: Pubkey, round_id: u64) -> Instruction {
    let miner_address = miner_pda(authority).0;
    let board_address = board_pda().0;
    let config_address = config_pda().0;
    let round_address = round_pda(round_id).0;
    let treasury_address = TREASURY_ADDRESS;
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true), // signer (used as authority by program)
            AccountMeta::new(board_address, false),
            AccountMeta::new(config_address, false), // config (needed for premine check)
            AccountMeta::new(miner_address, false),
            AccountMeta::new(round_address, false),
            AccountMeta::new(treasury_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: Checkpoint {}.to_bytes(),
    }
}

pub fn set_admin(signer: Pubkey, admin: Pubkey) -> Instruction {
    let config_address = config_pda().0;
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(config_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: SetAdmin {
            admin: admin.to_bytes(),
        }
        .to_bytes(),
    }
}

pub fn set_admin_fee(signer: Pubkey, admin_fee: u64) -> Instruction {
    let config_address = config_pda().0;
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(config_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: SetAdminFee {
            admin_fee: admin_fee.to_le_bytes(),
        }
        .to_bytes(),
    }
}

pub fn set_fee_collector(signer: Pubkey, fee_collector: Pubkey) -> Instruction {
    let config_address = config_pda().0;
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(config_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: SetFeeCollector {
            fee_collector: fee_collector.to_bytes(),
        }
        .to_bytes(),
    }
}

/// Sets the TGE (Token Generation Event) timestamp.
/// If current time < tge_timestamp, pre-mine is active.
/// Set to 0 to disable pre-mine.
/// Admin-only instruction.
pub fn set_tge_timestamp(signer: Pubkey, tge_timestamp: i64) -> Instruction {
    let config_address = config_pda().0;
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(config_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: SetTgeTimestamp {
            tge_timestamp: tge_timestamp.to_le_bytes(),
        }
        .to_bytes(),
    }
}

pub fn set_auction(
    signer: Pubkey,
    halving_period_seconds: u64,
    last_halving_time: u64,
    base_mining_rates: [u64; 4],
    auction_duration_seconds: u64,
    starting_prices: [u64; 4],
    _well_id: u64, // Kept for backwards compatibility, but not used (always updates auction only)
) -> Instruction {
    let config_address = config_pda().0;
    let auction_address = auction_pda().0;
    
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new_readonly(config_address, false),
            AccountMeta::new(auction_address, false),
        ],
        data: SetAuction {
            halving_period_seconds: halving_period_seconds.to_le_bytes(),
            last_halving_time: last_halving_time.to_le_bytes(),
            base_mining_rates: [
                base_mining_rates[0].to_le_bytes(),
                base_mining_rates[1].to_le_bytes(),
                base_mining_rates[2].to_le_bytes(),
                base_mining_rates[3].to_le_bytes(),
            ],
            auction_duration_seconds: auction_duration_seconds.to_le_bytes(),
            starting_prices: [
                starting_prices[0].to_le_bytes(),
                starting_prices[1].to_le_bytes(),
                starting_prices[2].to_le_bytes(),
                starting_prices[3].to_le_bytes(),
            ],
            well_id: 4u64.to_le_bytes(), // Always use 4 to indicate auction-only update
        }
        .to_bytes(),
    }
}

// let [signer_info, mint_info, sender_info, stake_info, stake_tokens_info, treasury_info, system_program, token_program, associated_token_program] =

pub fn deposit(signer: Pubkey, authority: Pubkey, amount: u64, lock_duration_days: u64, stake_id: u64) -> Instruction {
    let mint_address = MINT_ADDRESS;
    let stake_address = stake_pda_with_id(authority, stake_id).0; // Derive from authority, not signer
    let stake_tokens_address = get_associated_token_address(&stake_address, &MINT_ADDRESS);
    let sender_address = get_associated_token_address(&authority, &MINT_ADDRESS); // Authority's ATA
    let pool_address = pool_pda().0;
    let pool_tokens_address = pool_tokens_address();
    let miner_address = miner_pda(authority).0; // Derive from authority
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true), // payer (session payer or regular wallet, pays fees)
            AccountMeta::new(authority, true), // authority (user's wallet, signs token transfer and used for PDA derivation)
            AccountMeta::new(mint_address, false),
            AccountMeta::new(sender_address, false),
            AccountMeta::new(stake_address, false),
            AccountMeta::new(stake_tokens_address, false),
            AccountMeta::new(pool_address, false),
            AccountMeta::new(pool_tokens_address, false),
            AccountMeta::new(miner_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        ],
        data: Deposit {
            amount: amount.to_le_bytes(),
            lock_duration_days: lock_duration_days.to_le_bytes(),
            stake_id: stake_id.to_le_bytes(),
        }
        .to_bytes(),
    }
}

// let [signer_info, mint_info, recipient_info, stake_info, stake_tokens_info, treasury_info, system_program, token_program, associated_token_program] =

pub fn withdraw(signer: Pubkey, authority: Pubkey, amount: u64, stake_id: u64) -> Instruction {
    let stake_address = stake_pda_with_id(authority, stake_id).0; // Derive from authority, not signer
    let stake_tokens_address = get_associated_token_address(&stake_address, &MINT_ADDRESS);
    let mint_address = MINT_ADDRESS;
    let recipient_address = get_associated_token_address(&authority, &MINT_ADDRESS); // Authority's ATA
    let pool_address = pool_pda().0;
    let pool_tokens_address = pool_tokens_address();
    let miner_address = miner_pda(authority).0; // Derive from authority
    let treasury_address = treasury_pda().0;
    let treasury_tokens_address = treasury_tokens_address();
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true), // payer (session payer or regular wallet)
            AccountMeta::new(authority, false), // authority (user's wallet, for PDA derivation)
            AccountMeta::new(mint_address, false),
            AccountMeta::new(recipient_address, false),
            AccountMeta::new(stake_address, false),
            AccountMeta::new(stake_tokens_address, false),
            AccountMeta::new(pool_address, false),
            AccountMeta::new(pool_tokens_address, false),
            AccountMeta::new(miner_address, false),
            AccountMeta::new(treasury_address, false), // Treasury account (writable, signed by PDA)
            AccountMeta::new(treasury_tokens_address, false), // Treasury OIL token account (writable, signed by PDA)
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        ],
        data: Withdraw {
            amount: amount.to_le_bytes(),
            stake_id: stake_id.to_le_bytes(),
        }
        .to_bytes(),
    }
}

// let [signer_info, automation_info, miner_info, system_program] = accounts else {

/// Reload SOL from miner account to automation balance with single-tier referral system.
/// 
/// If the miner has a referrer, 1.0% of the claim goes to the referrer.
/// 
/// Account structure:
/// - Base: signer, automation, miner, system_program
/// - If miner has referrer (required): [miner_referrer, referral_referrer]
pub fn reload_sol(
    signer: Pubkey,
    authority: Pubkey,
    referrer_miner: Option<Pubkey>,
    referrer_referral: Option<Pubkey>,
) -> Instruction {
    let automation_address = automation_pda(authority).0;
    let miner_address = miner_pda(authority).0;
    
    let mut accounts = vec![
        AccountMeta::new(signer, true),
        AccountMeta::new(automation_address, false),
        AccountMeta::new(miner_address, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];
    
    // Add referral accounts if provided (required when miner has referrer)
    if let (Some(miner_ref), Some(referral_ref)) = (referrer_miner, referrer_referral) {
        accounts.push(AccountMeta::new(miner_ref, false));
        accounts.push(AccountMeta::new(referral_ref, false));
    }
    
    Instruction {
        program_id: crate::ID,
        accounts,
        data: ReloadSOL {}.to_bytes(),
    }
}

// let [signer_info, mint_info, recipient_info, stake_info, treasury_info, treasury_tokens_info, system_program, token_program, associated_token_program] =

/// Claim SOL yield from staking. Stakers earn SOL rewards (2% of round winnings), not OIL.
pub fn claim_yield(signer: Pubkey, amount: u64, stake_id: u64) -> Instruction {
    let stake_address = stake_pda_with_id(signer, stake_id).0;
    let pool_address = pool_pda().0;
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true), // signer and writable for receiving SOL
            AccountMeta::new(stake_address, false),
            AccountMeta::new(pool_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: ClaimYield {
            amount: amount.to_le_bytes(),
        }
        .to_bytes(),
    }
}

pub fn new_var(
    signer: Pubkey,
    provider: Pubkey,
    id: u64,
    commit: [u8; 32],
    samples: u64,
) -> Instruction {
    let board_address = board_pda().0;
    let config_address = config_pda().0;
    let var_address = entropy_rng_api::state::var_pda(board_address, id).0;
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(board_address, false),
            AccountMeta::new(config_address, false),
            AccountMeta::new(provider, false),
            AccountMeta::new(var_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(entropy_rng_api::ID, false),
        ],
        data: NewVar {
            id: id.to_le_bytes(),
            commit: commit,
            samples: samples.to_le_bytes(),
        }
        .to_bytes(),
    }
}

pub fn set_swap_program(signer: Pubkey, new_program: Pubkey) -> Instruction {
    let config_address = config_pda().0;
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(config_address, false),
            AccountMeta::new_readonly(new_program, false),
        ],
        data: SetSwapProgram {}.to_bytes(),
    }
}

pub fn set_var_address(signer: Pubkey, new_var_address: Pubkey) -> Instruction {
    let board_address = board_pda().0;
    let config_address = config_pda().0;
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(board_address, false),
            AccountMeta::new(config_address, false),
            AccountMeta::new(new_var_address, false),
        ],
        data: SetVarAddress {}.to_bytes(),
    }
}

/// Migrate: Extend Treasury struct with liquidity field.
/// This migration ensures the Treasury account has the new liquidity field available.
/// Must be called by the admin.
/// Accounts: signer, config, treasury, system_program
pub fn migrate(signer: Pubkey) -> Instruction {
    let config_address = config_pda().0;
    let treasury_address = treasury_pda().0;
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(config_address, false),
            AccountMeta::new(treasury_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: Migrate {}.to_bytes(),
    }
}

/// Create a referral account to become a referrer.
pub fn create_referral(signer: Pubkey) -> Instruction {
    let referral_address = referral_pda(signer).0;
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(referral_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data: CreateReferral {}.to_bytes(),
    }
}

/// Creates a Whitelist account for a shared access code.
/// Admin-only instruction.
/// Accounts: signer (admin), config, whitelist, system_program
pub fn create_whitelist(
    signer: Pubkey,
    code_hash: [u8; 32],
) -> Instruction {
    let config_address = config_pda().0;
    let (whitelist_address, _) = Whitelist::pda(code_hash);
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true), // signer (admin)
            AccountMeta::new_readonly(config_address, false), // config
            AccountMeta::new(whitelist_address, false), // whitelist
            AccountMeta::new_readonly(system_program::ID, false), // system_program
        ],
        data: CreateWhitelist {
            code_hash,
        }
        .to_bytes(),
    }
}

/// Claim pending referral rewards (both SOL and OIL).
/// 
/// Account structure (for Fogo sessions):
/// - Base: signer (payer), authority (user's wallet), referral, referral_tokens, mint, recipient, system_program, token_program, associated_token_program
pub fn claim_referral(signer: Pubkey, authority: Pubkey) -> Instruction {
    let referral_address = referral_pda(authority).0;
    let referral_oil_address = get_associated_token_address(&referral_address, &MINT_ADDRESS);
    let recipient_oil_address = get_associated_token_address(&authority, &MINT_ADDRESS);
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true), // 0: signer (payer)
            AccountMeta::new(authority, false), // 1: authority (user's wallet, receives SOL)
            AccountMeta::new(referral_address, false), // 2: referral
            AccountMeta::new(referral_oil_address, false), // 3: referral_tokens (Referral account's OIL ATA)
            AccountMeta::new(MINT_ADDRESS, false), // 4: mint
            AccountMeta::new(recipient_oil_address, false), // 5: recipient (Recipient's OIL ATA - authority's wallet)
            AccountMeta::new_readonly(system_program::ID, false), // 6: system_program
            AccountMeta::new_readonly(spl_token::ID, false), // 7: token_program
            AccountMeta::new_readonly(spl_associated_token_account::ID, false), // 8: associated_token_program
        ],
        data: ClaimReferral {}.to_bytes(),
    }
}

/// Direct solo bid on an auction well (seize ownership).
/// The bid amount is calculated on-chain as current_price + 1 lamport.
/// User must have enough SOL in their wallet to cover the bid.
/// 
/// Account structure:
/// - Base: signer, authority, program_signer (optional), payer (optional), well, auction, treasury, treasury_tokens, mint, mint_authority, mint_program, staking_pool, fee_collector, config, token_program, system_program, oil_program
/// - If previous owner exists (optional): [previous_owner_miner, previous_owner]
/// - If referrer is provided (optional): [referral]
pub fn place_bid(
    signer: Pubkey,
    authority: Pubkey,
    square_id: u64,
    fee_collector: Pubkey,
    previous_owner_miner: Option<Pubkey>, // Previous owner's miner PDA (if previous owner exists)
    previous_owner: Option<Pubkey>, // Previous owner pubkey (if previous owner exists)
    referrer: Option<Pubkey>, // Optional referrer pubkey for new miners
) -> Instruction {
    let well_address = well_pda(square_id).0;
    let auction_address = auction_pda().0;
    let treasury_address = treasury_pda().0;
    let treasury_tokens_address = get_associated_token_address(&treasury_address, &MINT_ADDRESS);
    let staking_pool_address = pool_pda().0;
    let config_address = config_pda().0;
    let mint_authority_address = oil_mint_api::state::authority_pda().0;
    let bidder_miner_address = miner_pda(authority).0;
    
    let mut accounts = vec![
        AccountMeta::new(signer, true), // 0: signer
        AccountMeta::new(authority, false), // 1: authority
    ];
    
    // Add program_signer and payer for Fogo sessions (these are added by the client, not here)
    // For now, we'll add placeholders or the client will add them
    
    accounts.extend_from_slice(&[
        AccountMeta::new(well_address, false), // well
        AccountMeta::new(auction_address, false), // Must be writable for auction_program_log CPI
        AccountMeta::new(treasury_address, false),
        AccountMeta::new(treasury_tokens_address, false),
        AccountMeta::new(MINT_ADDRESS, false),
        AccountMeta::new(mint_authority_address, false),
        AccountMeta::new_readonly(oil_mint_api::ID, false),
        AccountMeta::new(staking_pool_address, false),
        AccountMeta::new(fee_collector, false),
        AccountMeta::new_readonly(config_address, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(crate::ID, false), // oil_program
        AccountMeta::new(bidder_miner_address, false), // bidder_miner
    ]);
    
    // Add previous owner accounts if provided
    if let (Some(miner_pubkey), Some(owner_pubkey)) = (previous_owner_miner, previous_owner) {
        accounts.push(AccountMeta::new(miner_pubkey, false)); // previous_owner_miner
        accounts.push(AccountMeta::new(owner_pubkey, false)); // previous_owner
    }
    
    // Add referral account if referrer is provided
    if let Some(referrer_pubkey) = referrer {
        let referral_address = referral_pda(referrer_pubkey).0;
        accounts.push(AccountMeta::new(referral_address, false)); // referral
    }
    
    // Note: Wrapped token accounts are added by the client:
    // - user_wrapped_sol (source)
    // - treasury_wrapped_sol (temp ATA for all wrapped SOL - will be closed to get native SOL)
    // - token_program, native_mint, ata_program
    // The program distributes native SOL from treasury to pool and fee_collector after closing the temp ATA.
    
    Instruction {
        program_id: crate::ID,
        accounts,
        data: instruction::PlaceBid {
            square_id: square_id.to_le_bytes(),
            referrer: referrer.unwrap_or(Pubkey::default()).to_bytes(),
        }
        .to_bytes(),
    }
}

/// Claim auction-based OIL rewards
/// - OIL rewards: from current ownership and previous ownership (pre-minted)
/// 
/// Account structure:
/// - Base: signer, miner, well accounts (one per well in mask), auction pool accounts (optional, one per well), auction, treasury, treasury_tokens, mint, mint_authority, mint_program, recipient, token_program, associated_token_program, system_program, oil_program
/// - Bid accounts (one per well in mask, required for pool contributors): [bid_0, bid_1, bid_2, bid_3] (must include epoch_id in PDA)
/// Claim auction-based OIL rewards
/// 
/// Account structure:
/// - Base: signer, miner, well_0, well_1, well_2, well_3, auction, treasury, treasury_tokens, mint, mint_authority, mint_program, recipient, token_program, associated_token_program, system_program, oil_program
/// - If miner has referrer (required): [miner_referrer, referral_referrer, referral_referrer_oil_ata]
pub fn claim_auction_oil(
    signer: Pubkey,
    well_mask: u8, // Bitmask: bit 0 = well 0, bit 1 = well 1, etc.
    referrer_miner: Option<Pubkey>, // Referrer's miner PDA (if miner has referrer)
    referrer_referral: Option<Pubkey>, // Referrer's referral PDA (if miner has referrer)
    referrer_referral_oil_ata: Option<Pubkey>, // Referrer's referral OIL ATA (if miner has referrer)
) -> Instruction {
    let miner_address = miner_pda(signer).0;
    let well_0_address = well_pda(0).0;
    let well_1_address = well_pda(1).0;
    let well_2_address = well_pda(2).0;
    let well_3_address = well_pda(3).0;
    let auction_address = auction_pda().0;
    let treasury_address = treasury_pda().0;
    let treasury_tokens_address = get_associated_token_address(&treasury_address, &MINT_ADDRESS);
    let recipient_address = get_associated_token_address(&signer, &MINT_ADDRESS);
    let mint_authority_address = oil_mint_api::state::authority_pda().0;
    
    let mut accounts = vec![
        AccountMeta::new(signer, true),
        AccountMeta::new(miner_address, false),
        AccountMeta::new(well_0_address, false),
        AccountMeta::new(well_1_address, false),
        AccountMeta::new(well_2_address, false),
        AccountMeta::new(well_3_address, false),
        AccountMeta::new(auction_address, false),
        AccountMeta::new(treasury_address, false),
        AccountMeta::new(treasury_tokens_address, false),
        AccountMeta::new(MINT_ADDRESS, false),
        AccountMeta::new(mint_authority_address, false),
        AccountMeta::new_readonly(oil_mint_api::ID, false),
        AccountMeta::new(recipient_address, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];
    
    // Add referrer accounts if provided (required if miner has referrer)
    if let (Some(miner_pubkey), Some(referral_pubkey), Some(oil_ata_pubkey)) = 
        (referrer_miner, referrer_referral, referrer_referral_oil_ata) {
        accounts.push(AccountMeta::new(miner_pubkey, false));
        accounts.push(AccountMeta::new(referral_pubkey, false));
        accounts.push(AccountMeta::new(oil_ata_pubkey, false));
    }
    
    Instruction {
        program_id: crate::ID,
        accounts,
        data: ClaimAuctionOIL {
            well_mask,
        }
        .to_bytes(),
    }
}

/// Claim auction-based SOL rewards
/// 
/// Account structure:
/// - Base: signer (writable), miner, treasury, auction, system_program, oil_program
/// - If miner has referrer (required): [miner_referrer, referral_referrer]
pub fn claim_auction_sol(
    signer: Pubkey,
    referrer_miner: Option<Pubkey>, // Referrer's miner PDA (if miner has referrer)
    referrer_referral: Option<Pubkey>, // Referrer's referral PDA (if miner has referrer)
) -> Instruction {
    let miner_address = miner_pda(signer).0;
    let (auction_address, _) = auction_pda();
    let treasury_address = treasury_pda().0;
    
    let mut accounts = vec![
        AccountMeta::new(signer, true), // signer and writable for receiving SOL
        AccountMeta::new(miner_address, false),
        AccountMeta::new(treasury_address, false),
        AccountMeta::new(auction_address, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];
    
    // Add referrer accounts if provided (required if miner has referrer)
    if let (Some(miner_pubkey), Some(referral_pubkey)) = (referrer_miner, referrer_referral) {
        accounts.push(AccountMeta::new(miner_pubkey, false));
        accounts.push(AccountMeta::new(referral_pubkey, false));
    }
    
    Instruction {
        program_id: crate::ID,
        accounts,
        data: ClaimAuctionSOL {
            _reserved: 0,
        }
        .to_bytes(),
    }
}

// ============================================================================
// FOGO Session SDK Functions
// ============================================================================

pub fn checkpoint_with_session(
    signer: Pubkey,
    authority: Pubkey,
    program_signer: Pubkey,
    round_id: u64,
) -> Instruction {
    let miner_address = miner_pda(authority).0;
    let board_address = board_pda().0;
    let config_address = config_pda().0;
    let round_address = round_pda(round_id).0;
    let treasury_address = TREASURY_ADDRESS;
    
    // Manually construct instruction data with CheckpointWithSession discriminator (52)
    // Checkpoint is an empty struct, so we just need the discriminator byte
    let data = vec![52u8]; // CheckpointWithSession = 52
    
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true), // signer (session account)
            AccountMeta::new(authority, false), // authority (user's wallet)
            AccountMeta::new_readonly(program_signer, false), // program_signer
            AccountMeta::new(board_address, false),
            AccountMeta::new(config_address, false), // config (needed for premine check)
            AccountMeta::new(miner_address, false),
            AccountMeta::new(round_address, false),
            AccountMeta::new(treasury_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
        ],
        data,
    }
}

pub fn deploy_with_session(
    signer: Pubkey,
    authority: Pubkey,
    program_signer: Pubkey,
    payer: Pubkey,
    amount: u64,
    round_id: u64,
    squares: [bool; 25],
    referrer: Option<Pubkey>,
    pooled: bool,
) -> Instruction {
    let automation_address = automation_pda(authority).0;
    let board_address = board_pda().0;
    let miner_address = miner_pda(authority).0;
    let round_address = round_pda(round_id).0;
    let entropy_var_address = entropy_rng_api::state::var_pda(board_address, 0).0;

    let mut mask: u32 = 0;
    for (i, &square) in squares.iter().enumerate() {
        if square {
            mask |= 1 << i;
        }
    }
    
    let referrer_pubkey = referrer.unwrap_or(Pubkey::default());
    // Match program logic: has_referrer = referrer != Pubkey::default() && referrer != authority
    let has_referrer = referrer_pubkey != Pubkey::default() && referrer_pubkey != authority;
    let user_wrapped_sol_ata = get_associated_token_address(&authority, &SOL_MINT);
    let round_wrapped_sol_ata = get_associated_token_address(&round_address, &SOL_MINT);

    let mut accounts = vec![
        AccountMeta::new(signer, true),
        AccountMeta::new(authority, false),
        AccountMeta::new_readonly(program_signer, false),
        AccountMeta::new(payer, false),
        AccountMeta::new(automation_address, false),
        AccountMeta::new(board_address, false),
        AccountMeta::new(miner_address, false),
        AccountMeta::new(round_address, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(crate::ID, false),
        AccountMeta::new(user_wrapped_sol_ata, false),
        AccountMeta::new(round_wrapped_sol_ata, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(SOL_MINT, false),
        AccountMeta::new_readonly(spl_associated_token_account::ID, false),
    ];
    
    if has_referrer {
        let referral_address = referral_pda(referrer_pubkey).0;
        accounts.push(AccountMeta::new(referral_address, false));
    }
    
    accounts.push(AccountMeta::new(entropy_var_address, false));
    accounts.push(AccountMeta::new_readonly(entropy_rng_api::ID, false));

    Instruction {
        program_id: crate::ID,
        accounts,
        data: Deploy {
            amount: amount.to_le_bytes(),
            squares: mask.to_le_bytes(),
            referrer: referrer_pubkey.to_bytes(),
            pooled: if pooled { 1 } else { 0 },
        }
        .to_bytes(),
    }
}

pub fn automate_with_session(
    signer: Pubkey,
    authority: Pubkey,
    program_signer: Pubkey,
    payer: Pubkey,
    amount: u64,
    deposit: u64,
    executor: Pubkey,
    fee: u64,
    mask: u64,
    strategy: u8,
    reload: bool,
    referrer: Option<Pubkey>,
    pooled: bool,
    is_new_miner: bool,
) -> Instruction {
    let automation_address = automation_pda(authority).0;
    let miner_address = miner_pda(authority).0;
    let referrer_pk = referrer.unwrap_or(Pubkey::default());
    let user_wrapped_sol_ata = get_associated_token_address(&authority, &SOL_MINT);
    let automation_wrapped_sol_ata = get_associated_token_address(&automation_address, &SOL_MINT);
    
    let mut accounts = vec![
        AccountMeta::new(signer, true),
        AccountMeta::new(authority, false),
        AccountMeta::new_readonly(program_signer, false),
        AccountMeta::new(payer, false),
        AccountMeta::new(automation_address, false),
        AccountMeta::new(executor, false),
        AccountMeta::new(miner_address, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(crate::ID, false),
        AccountMeta::new(user_wrapped_sol_ata, false),
        AccountMeta::new(automation_wrapped_sol_ata, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(SOL_MINT, false),
        AccountMeta::new_readonly(spl_associated_token_account::ID, false),
    ];
    
    if is_new_miner && referrer.is_some() && referrer_pk != Pubkey::default() {
        let referral_address = referral_pda(referrer_pk).0;
        accounts.push(AccountMeta::new(referral_address, false));
    }
    
    Instruction {
        program_id: crate::ID,
        accounts,
        data: Automate {
            amount: amount.to_le_bytes(),
            deposit: deposit.to_le_bytes(),
            fee: fee.to_le_bytes(),
            mask: mask.to_le_bytes(),
            strategy: strategy as u8,
            reload: (reload as u64).to_le_bytes(),
            referrer: referrer_pk.to_bytes(),
            pooled: pooled as u8,
        }
        .to_bytes(),
    }
}

pub fn place_bid_with_session(
    signer: Pubkey,
    authority: Pubkey,
    program_signer: Pubkey,
    payer: Pubkey,
    square_id: u64,
    fee_collector: Pubkey,
    previous_owner_miner: Option<Pubkey>,
    previous_owner: Option<Pubkey>,
    referrer: Option<Pubkey>,
) -> Instruction {
    let well_address = well_pda(square_id).0;
    let auction_address = auction_pda().0;
    let treasury_address = treasury_pda().0;
    let treasury_tokens_address = get_associated_token_address(&treasury_address, &MINT_ADDRESS);
    let staking_pool_address = pool_pda().0;
    let config_address = config_pda().0;
    let mint_authority_address = oil_mint_api::state::authority_pda().0;
    let bidder_miner_address = miner_pda(authority).0;
    let user_wrapped_sol_ata = get_associated_token_address(&authority, &SOL_MINT);
    let treasury_wrapped_sol_ata = get_associated_token_address(&treasury_address, &SOL_MINT);
    
    let mut accounts = vec![
        AccountMeta::new(signer, true),
        AccountMeta::new(authority, false),
        AccountMeta::new_readonly(program_signer, false),
        AccountMeta::new(payer, false),
        AccountMeta::new(well_address, false),
        AccountMeta::new(auction_address, false),
        AccountMeta::new(treasury_address, false),
        AccountMeta::new(treasury_tokens_address, false),
        AccountMeta::new(MINT_ADDRESS, false),
        AccountMeta::new(mint_authority_address, false),
        AccountMeta::new_readonly(oil_mint_api::ID, false),
        AccountMeta::new(staking_pool_address, false),
        AccountMeta::new(fee_collector, false),
        AccountMeta::new_readonly(config_address, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(crate::ID, false),
        AccountMeta::new(bidder_miner_address, false),
    ];
    
    if let (Some(miner_pubkey), Some(owner_pubkey)) = (previous_owner_miner, previous_owner) {
        accounts.push(AccountMeta::new(miner_pubkey, false));
        accounts.push(AccountMeta::new(owner_pubkey, false));
    }
    
    if let Some(referrer_pubkey) = referrer {
        let referral_address = referral_pda(referrer_pubkey).0;
        accounts.push(AccountMeta::new(referral_address, false));
    }
    
    accounts.extend_from_slice(&[
        AccountMeta::new(user_wrapped_sol_ata, false),
        AccountMeta::new(treasury_wrapped_sol_ata, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(SOL_MINT, false),
        AccountMeta::new_readonly(spl_associated_token_account::ID, false),
    ]);
    
    Instruction {
        program_id: crate::ID,
        accounts,
        data: instruction::PlaceBid {
            square_id: square_id.to_le_bytes(),
            referrer: referrer.unwrap_or(Pubkey::default()).to_bytes(),
        }
        .to_bytes(),
    }
}

pub fn claim_auction_oil_with_session(
    signer: Pubkey,
    authority: Pubkey,
    program_signer: Pubkey,
    payer: Pubkey,
    well_mask: u8,
    referrer_miner: Option<Pubkey>,
    referrer_referral: Option<Pubkey>,
    referrer_referral_oil_ata: Option<Pubkey>,
) -> Instruction {
    let miner_address = miner_pda(authority).0;
    let well_0_address = well_pda(0).0;
    let well_1_address = well_pda(1).0;
    let well_2_address = well_pda(2).0;
    let well_3_address = well_pda(3).0;
    let auction_address = auction_pda().0;
    let treasury_address = treasury_pda().0;
    let treasury_tokens_address = get_associated_token_address(&treasury_address, &MINT_ADDRESS);
    let recipient_address = get_associated_token_address(&authority, &MINT_ADDRESS);
    let mint_authority_address = oil_mint_api::state::authority_pda().0;
    
    let mut accounts = vec![
        AccountMeta::new(signer, true),
        AccountMeta::new(authority, false),
        AccountMeta::new_readonly(program_signer, false),
        AccountMeta::new(payer, false),
        AccountMeta::new(miner_address, false),
        AccountMeta::new(well_0_address, false),
        AccountMeta::new(well_1_address, false),
        AccountMeta::new(well_2_address, false),
        AccountMeta::new(well_3_address, false),
        AccountMeta::new(auction_address, false),
        AccountMeta::new(treasury_address, false),
        AccountMeta::new(treasury_tokens_address, false),
        AccountMeta::new(MINT_ADDRESS, false),
        AccountMeta::new(mint_authority_address, false),
        AccountMeta::new_readonly(oil_mint_api::ID, false),
        AccountMeta::new(recipient_address, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];
    
    if let (Some(miner_pubkey), Some(referral_pubkey), Some(oil_ata_pubkey)) = 
        (referrer_miner, referrer_referral, referrer_referral_oil_ata) {
        accounts.push(AccountMeta::new(miner_pubkey, false));
        accounts.push(AccountMeta::new(referral_pubkey, false));
        accounts.push(AccountMeta::new(oil_ata_pubkey, false));
    }
    
    Instruction {
        program_id: crate::ID,
        accounts,
        data: ClaimAuctionOIL {
            well_mask,
        }
        .to_bytes(),
    }
}

pub fn claim_auction_sol_with_session(
    signer: Pubkey,
    authority: Pubkey,
    program_signer: Pubkey,
    payer: Pubkey,
    referrer_miner: Option<Pubkey>,
    referrer_referral: Option<Pubkey>,
) -> Instruction {
    let miner_address = miner_pda(authority).0;
    let (auction_address, _) = auction_pda();
    let treasury_address = treasury_pda().0;
    
    let mut accounts = vec![
        AccountMeta::new(signer, true),
        AccountMeta::new(authority, false),
        AccountMeta::new_readonly(program_signer, false),
        AccountMeta::new(payer, false),
        AccountMeta::new(miner_address, false),
        AccountMeta::new(treasury_address, false),
        AccountMeta::new(auction_address, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];
    
    if let (Some(miner_pubkey), Some(referral_pubkey)) = (referrer_miner, referrer_referral) {
        accounts.push(AccountMeta::new(miner_pubkey, false));
        accounts.push(AccountMeta::new(referral_pubkey, false));
    }
    
    Instruction {
        program_id: crate::ID,
        accounts,
        data: ClaimAuctionSOL {
            _reserved: 0,
        }
        .to_bytes(),
    }
}

pub fn claim_sol_with_session(
    signer: Pubkey,
    authority: Pubkey,
    program_signer: Pubkey,
    payer: Pubkey,
    referrer_miner: Option<Pubkey>,
    referrer_referral: Option<Pubkey>,
) -> Instruction {
    let miner_address = miner_pda(authority).0;
    
    let mut accounts = vec![
        AccountMeta::new(signer, true),
        AccountMeta::new(authority, false),
        AccountMeta::new_readonly(program_signer, false),
        AccountMeta::new(payer, false),
        AccountMeta::new(miner_address, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];
    
    if let (Some(miner_pubkey), Some(referral_pubkey)) = (referrer_miner, referrer_referral) {
        accounts.push(AccountMeta::new(miner_pubkey, false));
        accounts.push(AccountMeta::new(referral_pubkey, false));
    }
    
    Instruction {
        program_id: crate::ID,
        accounts,
        data: ClaimSOL {}.to_bytes(),
    }
}

pub fn claim_oil_with_session(
    signer: Pubkey,
    authority: Pubkey,
    program_signer: Pubkey,
    payer: Pubkey,
    referrer_miner: Option<Pubkey>,
    referrer_referral: Option<Pubkey>,
    referrer_referral_oil_ata: Option<Pubkey>,
) -> Instruction {
    let miner_address = miner_pda(authority).0;
    let treasury_address = treasury_pda().0;
    let treasury_tokens_address = get_associated_token_address(&treasury_address, &MINT_ADDRESS);
    let recipient_address = get_associated_token_address(&authority, &MINT_ADDRESS);
    
    let mut accounts = vec![
        AccountMeta::new(signer, true),
        AccountMeta::new(authority, false),
        AccountMeta::new_readonly(program_signer, false),
        AccountMeta::new(payer, false),
        AccountMeta::new(miner_address, false),
        AccountMeta::new(MINT_ADDRESS, false),
        AccountMeta::new(recipient_address, false),
        AccountMeta::new(treasury_address, false),
        AccountMeta::new(treasury_tokens_address, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(spl_token::ID, false),
        AccountMeta::new_readonly(spl_associated_token_account::ID, false),
    ];
    
    if let (Some(miner_pubkey), Some(referral_pubkey), Some(oil_ata_pubkey)) = 
        (referrer_miner, referrer_referral, referrer_referral_oil_ata) {
        accounts.push(AccountMeta::new(miner_pubkey, false));
        accounts.push(AccountMeta::new(referral_pubkey, false));
        accounts.push(AccountMeta::new(oil_ata_pubkey, false));
    }
    
    Instruction {
        program_id: crate::ID,
        accounts,
        data: ClaimOIL {}.to_bytes(),
    }
}

pub fn withdraw_with_session(
    signer: Pubkey,
    authority: Pubkey,
    program_signer: Pubkey,
    payer: Pubkey,
    amount: u64,
    stake_id: u64,
) -> Instruction {
    let stake_address = stake_pda_with_id(authority, stake_id).0;
    let stake_tokens_address = get_associated_token_address(&stake_address, &MINT_ADDRESS);
    let mint_address = MINT_ADDRESS;
    let recipient_address = get_associated_token_address(&authority, &MINT_ADDRESS);
    let pool_address = pool_pda().0;
    let pool_tokens_address = pool_tokens_address();
    let miner_address = miner_pda(authority).0;
    let treasury_address = treasury_pda().0;
    let treasury_tokens_address = treasury_tokens_address();
    
    Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(signer, true),
            AccountMeta::new(authority, false),
            AccountMeta::new_readonly(program_signer, false),
            AccountMeta::new(payer, false),
            AccountMeta::new(mint_address, false),
            AccountMeta::new(recipient_address, false),
            AccountMeta::new(stake_address, false),
            AccountMeta::new(stake_tokens_address, false),
            AccountMeta::new(pool_address, false),
            AccountMeta::new(pool_tokens_address, false),
            AccountMeta::new(miner_address, false),
            AccountMeta::new(treasury_address, false),
            AccountMeta::new(treasury_tokens_address, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(spl_token::ID, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        ],
        data: Withdraw {
            amount: amount.to_le_bytes(),
            stake_id: stake_id.to_le_bytes(),
        }
        .to_bytes(),
    }
}
