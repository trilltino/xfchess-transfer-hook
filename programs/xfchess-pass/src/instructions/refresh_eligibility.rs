// Registrar instruction that re-attests a wallet's eligibility from the current slot.

use anchor_lang::prelude::*;

use crate::{
    error::TournamentPassError,
    rule,
    state::{
        ELIGIBILITY_RECORD_SEED, EligibilityRecord, TOURNAMENT_PASS_CONFIG_SEED,
        TournamentPassConfig,
    },
};

#[derive(Accounts)]
pub struct RefreshEligibility<'info> {
    #[account(
        seeds = [TOURNAMENT_PASS_CONFIG_SEED, tournament_pass_config.mint.as_ref()],
        bump = tournament_pass_config.bump,
        has_one = registrar @ TournamentPassError::UnauthorizedRegistrar,
    )]
    pub tournament_pass_config: Account<'info, TournamentPassConfig>,

    #[account(
        mut,
        seeds = [
            ELIGIBILITY_RECORD_SEED,
            tournament_pass_config.mint.as_ref(),
            eligibility_record.player.as_ref(),
        ],
        bump = eligibility_record.bump,
    )]
    pub eligibility_record: Account<'info, EligibilityRecord>,

    pub registrar: Signer<'info>,
}

pub fn refresh_eligibility(ctx: Context<RefreshEligibility>, verified_elo: u32) -> Result<()> {
    rule::validate_rating(&ctx.accounts.tournament_pass_config.policy, verified_elo)?;
    let eligibility_record = &mut ctx.accounts.eligibility_record;
    eligibility_record.verified_elo = verified_elo;
    eligibility_record.verified_at_slot = Clock::get()?.slot;
    eligibility_record.is_revoked = false;
    Ok(())
}
