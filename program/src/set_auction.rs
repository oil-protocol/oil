use oil_api::prelude::*;
use solana_program::log::sol_log;
use steel::*;

/// Sets the auction state (admin only)
pub fn process_set_auction(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    sol_log("🔧 Set Auction instruction started");
    
    // Parse data
    let args = SetAuction::try_from_bytes(data)?;
    let well_id = u64::from_le_bytes(args.well_id) as usize;
    
    // Load accounts: [signer, config, auction, well?]
    if accounts.len() < 3 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    let [signer_info, config_info, auction_info] =
        &accounts[0..3]
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    
    signer_info.is_signer()?;
    
    // Verify signer is admin
    config_info
        .as_account::<Config>(&oil_api::ID)?
        .assert(|c| c.admin == *signer_info.key)?;
    
    // Load and update auction account
    auction_info
        .is_writable()?
        .has_seeds(&[AUCTION], &oil_api::ID)?;
    let auction = auction_info.as_account_mut::<Auction>(&oil_api::ID)?;
    
    // Update auction account with new base_mining_rates
    auction.base_mining_rates = [
        u64::from_le_bytes(args.base_mining_rates[0]),
        u64::from_le_bytes(args.base_mining_rates[1]),
        u64::from_le_bytes(args.base_mining_rates[2]),
        u64::from_le_bytes(args.base_mining_rates[3]),
    ];
    auction.auction_duration_seconds = u64::from_le_bytes(args.auction_duration_seconds);
    auction.starting_prices = [
        u64::from_le_bytes(args.starting_prices[0]),
        u64::from_le_bytes(args.starting_prices[1]),
        u64::from_le_bytes(args.starting_prices[2]),
        u64::from_le_bytes(args.starting_prices[3]),
    ];
    
    // Initialize halving_period_seconds and last_halving_time if they're 0 (not yet set)
    let clock = Clock::get()?;
    let current_timestamp = clock.unix_timestamp as u64;
    
    // Read halving_period_seconds from instruction data (for initialization if needed)
    let halving_period_seconds_from_args = u64::from_le_bytes(args.halving_period_seconds);
    
    if auction.halving_period_seconds == 0 {
        // Initialize to value from instruction (should be 28 days = 2,419,200 seconds)
        auction.halving_period_seconds = if halving_period_seconds_from_args > 0 {
            halving_period_seconds_from_args
        } else {
            28 * 24 * 60 * 60 // 28 days default
        };
        sol_log(&format!("ℹ️  Initialized halving_period_seconds to {} ({} days)", 
            auction.halving_period_seconds,
            auction.halving_period_seconds / (24 * 60 * 60)));
    }
    
    if auction.last_halving_time == 0 {
        // Always initialize to current timestamp (from Clock) when it's 0
        auction.last_halving_time = current_timestamp;
        sol_log(&format!("ℹ️  Initialized last_halving_time to current timestamp: {}", current_timestamp));
    }
    
    sol_log(&format!("✅ Auction account updated: base_mining_rates={:?}, auction_duration={}", 
        auction.base_mining_rates, auction.auction_duration_seconds));
    sol_log(&format!("   Time-based halving: period={}s ({} days), last_halving={}, next_halving={}", 
        auction.halving_period_seconds,
        auction.halving_period_seconds / (24 * 60 * 60),
        auction.last_halving_time,
        auction.next_halving_time()));
    
    // Sync well mps if well_id < 4 and well account is provided
    if well_id < 4 && accounts.len() >= 4 {
        let well_info = &accounts[3];
        let well_id_u64 = well_id as u64;
        
        // Load well account
        match well_info
            .is_writable()
            .and_then(|_| well_info.has_seeds(&[WELL, &well_id_u64.to_le_bytes()], &oil_api::ID))
            .and_then(|_| well_info.as_account_mut::<Well>(&oil_api::ID))
        {
            Ok(well) => {
                // Update well mps to new base rate
                let new_base_mps = auction.base_mining_rates[well_id];
                well.mps = new_base_mps;
                
                // Update accumulated OIL and apply halvings
                well.update_accumulated_oil(&clock);
                well.check_and_apply_halving(auction, &clock);
                
                sol_log(&format!("✅ Synced well {} mps to {} (after halvings: {})", 
                    well_id, 
                    new_base_mps,
                    well.mps));
            }
            Err(e) => {
                sol_log(&format!("⚠️  Failed to load well {} account: {:?}", well_id, e));
            }
        }
    } else if well_id < 4 {
        sol_log(&format!("ℹ️  Well {} account not provided, skipping mps sync", well_id));
    }
    
    Ok(())
}

