// Registrar instruction that attests a wallet's rating by creating its eligibility record.

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
pub struct IssueEligibility<'info> {
    #[account(
        seeds = [TOURNAMENT_PASS_CONFIG_SEED, tournament_pass_config.mint.as_ref()],
        bump = tournament_pass_config.bump,
        has_one = registrar @ TournamentPassError::UnauthorizedRegistrar,
    )]
    pub tournament_pass_config: Account<'info, TournamentPassConfig>,

    #[account(
        init,
        payer = payer,
        space = 8 + EligibilityRecord::INIT_SPACE,
        seeds = [
            ELIGIBILITY_RECORD_SEED,
            tournament_pass_config.mint.as_ref(),
            player.key().as_ref(),
        ],
        bump,
    )]
    pub eligibility_record: Account<'info, EligibilityRecord>,

    /// CHECK: the wallet that will own the receiving token account; identity data, not a signer.
    pub player: UncheckedAccount<'info>,

    pub registrar: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn issue_eligibility(ctx: Context<IssueEligibility>, verified_elo: u32) -> Result<()> {
    let tournament_pass_config = &ctx.accounts.tournament_pass_config;
    rule::validate_rating(&tournament_pass_config.policy, verified_elo)?;
    ctx.accounts
        .eligibility_record
        .set_inner(EligibilityRecord {
            bump: ctx.bumps.eligibility_record,
            mint: tournament_pass_config.mint,
            player: ctx.accounts.player.key(),
            verified_elo,
            verified_at_slot: Clock::get()?.slot,
            is_revoked: false,
        });
    Ok(())
}
