// Account layout for a wallet's attested eligibility record.

use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace, Debug, PartialEq, Eq)]
pub struct EligibilityRecord {
    pub bump: u8,
    pub mint: Pubkey,
    pub player: Pubkey,
    pub verified_elo: u32,
    pub verified_at_slot: u64,
    pub is_revoked: bool,
}
