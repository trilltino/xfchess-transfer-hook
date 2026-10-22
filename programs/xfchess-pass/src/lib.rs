// Program entrypoint for XFChess Pass: declares its program ID and routes each instruction to its handler.

#![deny(unsafe_code)]
#![allow(clippy::diverging_sub_expression)]

use anchor_lang::prelude::*;

#[cfg(not(target_os = "solana"))]
pub mod client;
pub mod error;
pub mod instructions;
pub mod rule;
pub mod state;

pub use error::TournamentPassError;
pub use hook_kit::TransferContext;
pub use instructions::*;
pub use rule::EligibilityPolicy;
pub use state::*;

declare_id!("4sFBJCjU3QvoWZCfxdHJcGBCPjToTy2BzNK23wmRPaw9");

#[program]
pub mod xfchess_pass {
    use super::*;

    pub fn initialize(
        ctx: Context<Initialize>,
        tournament_id: u64,
        registrar: Pubkey,
        policy: EligibilityPolicy,
    ) -> Result<()> {
        instructions::initialize::initialize(ctx, tournament_id, registrar, policy)
    }

    pub fn issue_eligibility(ctx: Context<IssueEligibility>, verified_elo: u32) -> Result<()> {
        instructions::issue_eligibility::issue_eligibility(ctx, verified_elo)
    }

    pub fn refresh_eligibility(ctx: Context<RefreshEligibility>, verified_elo: u32) -> Result<()> {
        instructions::refresh_eligibility::refresh_eligibility(ctx, verified_elo)
    }

    pub fn revoke_eligibility(ctx: Context<RevokeEligibility>) -> Result<()> {
        instructions::revoke_eligibility::revoke_eligibility(ctx)
    }

    pub fn set_paused(ctx: Context<SetPaused>, is_paused: bool) -> Result<()> {
        instructions::set_paused::set_paused(ctx, is_paused)
    }

    #[instruction(discriminator = hook_kit::EXECUTE_DISCRIMINATOR_SLICE)]
    pub fn execute(ctx: Context<Execute>, amount: u64) -> Result<()> {
        instructions::execute::execute(ctx, amount)
    }
}
