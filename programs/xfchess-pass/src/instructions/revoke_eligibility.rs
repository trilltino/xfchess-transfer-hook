// Registrar instruction that revokes a wallet's eligibility record.

use anchor_lang::prelude::*;

use crate::{
    error::TournamentPassError,
    state::{
        ELIGIBILITY_RECORD_SEED, EligibilityRecord, TOURNAMENT_PASS_CONFIG_SEED,
        TournamentPassConfig,
    },
};

#[derive(Accounts)]
pub struct RevokeEligibility<'info> {
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

pub fn revoke_eligibility(ctx: Context<RevokeEligibility>) -> Result<()> {
    ctx.accounts.eligibility_record.is_revoked = true;
    Ok(())
}
