// Registrar instruction that pauses or resumes every transfer of the pass.

use anchor_lang::prelude::*;

use crate::{
    error::TournamentPassError,
    state::{TOURNAMENT_PASS_CONFIG_SEED, TournamentPassConfig},
};

#[derive(Accounts)]
pub struct SetPaused<'info> {
    #[account(
        mut,
        seeds = [TOURNAMENT_PASS_CONFIG_SEED, tournament_pass_config.mint.as_ref()],
        bump = tournament_pass_config.bump,
        has_one = registrar @ TournamentPassError::UnauthorizedRegistrar,
    )]
    pub tournament_pass_config: Account<'info, TournamentPassConfig>,

    pub registrar: Signer<'info>,
}

pub fn set_paused(ctx: Context<SetPaused>, is_paused: bool) -> Result<()> {
    ctx.accounts.tournament_pass_config.is_paused = is_paused;
    Ok(())
}
