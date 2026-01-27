use oil_api::{
    consts::{MINER, SEEKER},
    state::{Miner, Seeker},
};
use solana_program::pubkey;
use steel::*;

use spl_token_2022::{
    extension::{
        metadata_pointer::MetadataPointer, BaseStateWithExtensions, PodStateWithExtensions,
    },
    pod::{PodCOption, PodMint},
};

/// Claims ORE for seeker device.
pub fn process_claim_seeker(accounts: &[AccountInfo<'_>], _data: &[u8]) -> ProgramResult {
    // Load accounts.
    let [signer_info, miner_info, mint_info, seeker_info, token_account_info, system_program] =
        accounts
    else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };
    signer_info.is_signer()?;
    miner_info.is_writable()?;
    mint_info.has_owner(&spl_token_2022::ID)?;
    seeker_info.is_writable()?.is_empty()?;
    token_account_info
        .as_associated_token_account(signer_info.key, mint_info.key)?
        .assert(|t| t.amount() == 1)?;
    system_program.is_program(&system_program::ID)?;

    // Open seeker account.
    // Each genesis token can only be claimed once.
    create_program_account::<Seeker>(
        seeker_info,
        system_program,
        signer_info,
        &oil_api::ID,
        &[SEEKER, &mint_info.key.to_bytes()],
    )?;
    let seeker = seeker_info.as_account_mut::<Seeker>(&oil_api::ID)?;
    seeker.mint = *mint_info.key;

    // Open miner account.
    let miner = if miner_info.data_is_empty() {
        create_program_account::<Miner>(
            miner_info,
            system_program,
            signer_info,
            &oil_api::ID,
            &[MINER, &signer_info.key.to_bytes()],
        )?;
        let miner = miner_info.as_account_mut::<Miner>(&oil_api::ID)?;
        miner.authority = *signer_info.key;
        miner.deployed = [0; 25];
        miner.cumulative = [0; 25];
        miner.checkpoint_fee = 0;
        miner.checkpoint_id = 0;
        miner.last_claim_block_oil_at = 0;
        miner.last_claim_block_sol_at = 0;
        miner.block_rewards_factor = Numeric::ZERO;
        miner.block_rewards_sol = 0;
        miner.block_rewards_oil = 0;
        miner.block_refined_oil = 0;
        miner.round_id = 0;
        miner.pooled_deployed = 0;
        miner.is_seeker = 0;
        miner.buffer_a = 0;
        // Auction-based mining fields
        miner.auction_rewards_oil = 0;
        miner.auction_rewards_sol = 0;
        miner.auction_rewards_factor = Numeric::ZERO;
        miner.auction_refined_oil = 0;
        miner.last_claim_auction_oil_at = 0;
        miner.last_claim_auction_sol_at = 0;
        // Lifetime fields
        miner.lifetime_rewards_sol = 0;
        miner.lifetime_rewards_oil = 0;
        miner.lifetime_deployed = 0;
        miner.lifetime_bid = 0;
        miner.referrer = Pubkey::default();
        miner.total_stake_score = 0;
        miner
    } else {
        miner_info
            .as_account_mut::<Miner>(&oil_api::ID)?
            .assert_mut(|m| m.authority == *signer_info.key)?
    };

    // Load mint.
    let mint_data = mint_info.try_borrow_data()?;
    let mint = PodStateWithExtensions::<PodMint>::unpack(&mint_data)?;

    // Check mint authority.
    assert!(
        mint.base.mint_authority
            == PodCOption::some(pubkey!("GT2zuHVaZQYZSyQMgJPLzvkmyztfyXg2NJunqFp4p3A4")),
        "mint authority mismatch"
    );

    // Check metadata pointer.
    let ext = mint.get_extension::<MetadataPointer>()?;
    assert!(
        ext.authority.0 == pubkey!("GT2zuHVaZQYZSyQMgJPLzvkmyztfyXg2NJunqFp4p3A4"),
        "metadata authority mismatch"
    );
    assert!(
        ext.metadata_address.0 == pubkey!("GT22s89nU4iWFkNXj1Bw6uYhJJWDRPpShHt4Bk8f99Te"),
        "metadata address mismatch"
    );

    // Give miner a Seeker designation.
    miner.is_seeker = 1;

    Ok(())
}