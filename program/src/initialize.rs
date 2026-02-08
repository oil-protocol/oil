use oil_api::prelude::*;
use steel::*;

/// Initializes the program.
pub fn process_initialize(accounts: &[AccountInfo<'_>], data: &[u8]) -> ProgramResult {
    // Parse data.
    let args = Initialize::try_from_bytes(data)?;
    let barrel_authority = Pubkey::try_from(&args.barrel_authority[..]).map_err(|_| ProgramError::InvalidArgument)?;
    let fee_collector = Pubkey::try_from(&args.fee_collector[..]).map_err(|_| ProgramError::InvalidArgument)?;
    let swap_program = Pubkey::try_from(&args.swap_program[..]).map_err(|_| ProgramError::InvalidArgument)?;
    let var_address = Pubkey::try_from(&args.var_address[..]).map_err(|_| ProgramError::InvalidArgument)?;
    let admin_fee = u64::from_le_bytes(args.admin_fee);
    
    // Parse auction parameters
    let halving_period_seconds = u64::from_le_bytes(args.halving_period_seconds);
    let mut base_mining_rates = [0u64; 4];
    for i in 0..4 {
        base_mining_rates[i] = u64::from_le_bytes(args.base_mining_rates[i]);
    }
    let auction_duration_seconds = u64::from_le_bytes(args.auction_duration_seconds);
    let mut starting_prices = [0u64; 4];
    for i in 0..4 {
        starting_prices[i] = u64::from_le_bytes(args.starting_prices[i]);
    }
    
    if accounts.len() < 16 {
        return Err(ProgramError::NotEnoughAccountKeys);
    }
    
    let signer_info = &accounts[0];
    let board_info = &accounts[1];
    let config_info = &accounts[2];
    let mint_info = &accounts[3];
    let treasury_info = &accounts[4];
    let treasury_tokens_info = &accounts[5];
    let pool_info = &accounts[6];
    let pool_tokens_info = &accounts[7];
    let auction_info = &accounts[12];
    let system_program = &accounts[13];
    let token_program = &accounts[14];
    let associated_token_program = &accounts[15];
    
    signer_info.is_signer()?.has_address(&ADMIN_ADDRESS)?;
    board_info.has_seeds(&[BOARD], &oil_api::ID)?;
    config_info.has_seeds(&[CONFIG], &oil_api::ID)?;
    mint_info.has_address(&MINT_ADDRESS)?.as_mint()?;
    treasury_info.has_seeds(&[TREASURY], &oil_api::ID)?;
    // Only validate treasury_tokens_info address if account is not empty
    // This allows re-initialization when mint address changes
    if !treasury_tokens_info.data_is_empty() {
        treasury_tokens_info.has_address(&treasury_tokens_address())?;
    }
    pool_info.has_seeds(&[POOL], &oil_api::ID)?;
    // Only validate pool_tokens_info address if account is not empty
    // This allows re-initialization when mint address changes
    if !pool_tokens_info.data_is_empty() {
        pool_tokens_info.has_address(&pool_tokens_address())?;
    }
    auction_info.has_seeds(&[AUCTION], &oil_api::ID)?;
    system_program.is_program(&system_program::ID)?;
    token_program.is_program(&spl_token::ID)?;
    associated_token_program.is_program(&spl_associated_token_account::ID)?;

    // Create board account.
    if board_info.data_is_empty() {
        create_program_account::<Board>(
            board_info,
            system_program,
            signer_info,
            &oil_api::ID,
            &[BOARD],
        )?;
        let board = board_info.as_account_mut::<Board>(&oil_api::ID)?;
        board.round_id = 0;
        board.start_slot = 0;
        board.end_slot = u64::MAX; // Indicates waiting for first deploy
        board.epoch_id = 0;
    } else {
        board_info.as_account::<Board>(&oil_api::ID)?;
    }

    // Create config account.
    if config_info.data_is_empty() {
        create_program_account::<Config>(
            config_info,
            system_program,
            signer_info,
            &oil_api::ID,
            &[CONFIG],
        )?;
        let config = config_info.as_account_mut::<Config>(&oil_api::ID)?;
        config.admin = *signer_info.key;
        config.barrel_authority = barrel_authority;
        config.fee_collector = fee_collector;
        config.swap_program = swap_program;
        config.var_address = var_address;
        // Cap admin fee at 1% (100 basis points).
        config.admin_fee = admin_fee.min(100);
        // Initialize emission schedule (starts at week 0 = 50 OIL per round)
        config.emission_week = 0;
        config.last_emission_week_update = 0; // Will be set on first reset
        config.tge_timestamp = 0; // 0 = pre-mine disabled by default
    } else {
        config_info.as_account::<Config>(&oil_api::ID)?;
    }

    // Create treasury account.
    if treasury_info.data_is_empty() {
        create_program_account::<Treasury>(
            treasury_info,
            system_program,
            signer_info,
            &oil_api::ID,
            &[TREASURY],
        )?;
        let treasury = treasury_info.as_account_mut::<Treasury>(&oil_api::ID)?;
        treasury.balance = 0;
        treasury.gusher_sol = 0;
        treasury.block_rewards_factor = Numeric::ZERO;
        treasury.buffer_a = Numeric::ZERO;
        treasury.total_barrelled = 0;
        treasury.block_total_refined = 0;
        treasury.auction_rewards_sol = 0;
        treasury.block_total_unclaimed = 0;
        treasury.auction_rewards_factor = Numeric::ZERO;
        treasury.auction_total_unclaimed = 0;
        treasury.auction_total_refined = 0;
    } else {
        treasury_info.as_account::<Treasury>(&oil_api::ID)?;
    }

    // Create pool account.
    if pool_info.data_is_empty() {
        create_program_account::<Pool>(
            pool_info,
            system_program,
            signer_info,
            &oil_api::ID,
            &[POOL],
        )?;
        let pool = pool_info.as_account_mut::<Pool>(&oil_api::ID)?;
        pool.balance = 0;
        pool.stake_rewards_factor = Numeric::ZERO;
        pool.total_staked_score = 0;
        pool.total_staked = 0;
        pool.buffer_a = Numeric::ZERO;
        pool.total_burned_penalties = 0;
        pool.buffer_c = 0;
    } else {
        pool_info.as_account::<Pool>(&oil_api::ID)?;
    }

    // Create pool tokens account.
    // Only validate address if account exists - allows re-initialization when mint changes
    if pool_tokens_info.data_is_empty() {
        // Account doesn't exist - create it
        create_associated_token_account(
            signer_info,
            pool_info,
            pool_tokens_info,
            mint_info,
            system_program,
            token_program,
            associated_token_program,
        )?;
    } else {
        // Account exists - validate it's the correct ATA for current mint
        // If address doesn't match, this means old ATA exists - we can't fix it here
        // The caller must pass the correct (new) ATA address
        pool_tokens_info.has_address(&pool_tokens_address())?
            .as_associated_token_account(pool_info.key, mint_info.key)?;
    }

    // Initialize treasury token account.
    // Only validate address if account exists - allows re-initialization when mint changes
    if treasury_tokens_info.data_is_empty() {
        // Account doesn't exist - create it
        create_associated_token_account(
            signer_info,
            treasury_info,
            treasury_tokens_info,
            mint_info,
            system_program,
            token_program,
            associated_token_program,
        )?;
    } else {
        // Account exists - validate it's the correct ATA for current mint
        // If address doesn't match, this means old ATA exists - we can't fix it here
        // The caller must pass the correct (new) ATA address
        treasury_tokens_info.has_address(&treasury_tokens_address())?
            .as_associated_token_account(treasury_info.key, mint_info.key)?;
    }

    // Get current timestamp for initialization
    let clock = Clock::get()?;
    let current_timestamp = clock.unix_timestamp as u64;

    // Create Auction account (idempotent - only if empty)
    if auction_info.data_is_empty() {
        create_program_account::<Auction>(
            auction_info,
            system_program,
            signer_info,
            &oil_api::ID,
            &[AUCTION],
        )?;
        let auction = auction_info.as_account_mut::<Auction>(&oil_api::ID)?;
        // Time-based halving: use halving_period_seconds from instruction (defaults to 28 days = 2,419,200 seconds)
        auction.halving_period_seconds = if halving_period_seconds > 0 {
            halving_period_seconds
        } else {
            28 * 24 * 60 * 60 // Default to 28 days if not provided
        };
        auction.last_halving_time = current_timestamp; // Set to current time (first halving will be halving_period_seconds from now)
        auction.base_mining_rates = base_mining_rates;
        auction.auction_duration_seconds = auction_duration_seconds;
        auction.starting_prices = starting_prices;
        auction.buffer_a = Numeric::ZERO;
        // Buffer fields (for future use)
        auction.buffer_b = 0;
        auction.buffer_c = 0;
        auction.buffer_d = 0;
    } else {
        auction_info.as_account::<Auction>(&oil_api::ID)?;
    }

    // Create Well accounts for each well (idempotent - only if empty)
    for well_id in 0u64..4u64 {
        let well_id_usize: usize = well_id.try_into().unwrap_or(0); // Safe: well_id is 0-3
        let well_info = &accounts[8 + well_id_usize];
        well_info
            .has_seeds(&[WELL, &well_id.to_le_bytes()], &oil_api::ID)?;
        
        if well_info.data_is_empty() {
            create_program_account::<Well>(
                well_info,
                system_program,
                signer_info,
                &oil_api::ID,
                &[WELL, &well_id.to_le_bytes()],
            )?;
            let well = well_info.as_account_mut::<Well>(&oil_api::ID)?;
            well.well_id = well_id;
            well.epoch_id = 0; // Start at epoch 0
            well.current_bidder = Pubkey::default(); // Unowned
            well.init_price = starting_prices[well_id_usize];
            well.mps = base_mining_rates[well_id_usize];
            well.epoch_start_time = current_timestamp;
            well.accumulated_oil = 0;
            well.last_update_time = current_timestamp;
            well.halving_count = 0;
            well.lifetime_oil_mined = 0;
            well.operator_total_oil_mined = 0;
            well.buffer_c = 0;
            well.total_contributed = 0;
            well.pool_bid_cost = 0;
        } else {
            well_info.as_account::<Well>(&oil_api::ID)?;
        }
    }

    Ok(())
}