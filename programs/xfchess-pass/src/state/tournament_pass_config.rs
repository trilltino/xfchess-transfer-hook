// Account layout for the per-mint tournament pass config.

use anchor_lang::prelude::*;

use crate::rule::EligibilityPolicy;

#[account]
#[derive(InitSpace, Debug, PartialEq, Eq)]
pub struct TournamentPassConfig {
    pub version: u8,
    pub bump: u8,
    pub extra_account_meta_list_bump: u8,
    pub mint: Pubkey,
    pub tournament_id: u64,
    pub registrar: Pubkey,
    pub policy: EligibilityPolicy,
    pub is_paused: bool,
}
