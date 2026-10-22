// XFChess Pass's pure rule: a pass transfer needs a current, in-range rating on the receiving wallet.

use anchor_lang::prelude::{AnchorDeserialize, AnchorSerialize, InitSpace};
use hook_kit::TransferContext;

use crate::{error::TournamentPassError, state::EligibilityRecord};

#[derive(AnchorSerialize, AnchorDeserialize, InitSpace, Clone, Copy, Debug, PartialEq, Eq)]
pub struct EligibilityPolicy {
    pub min_elo: u32,
    pub max_elo: u32,
    pub max_eligibility_age_slots: u64,
}

pub fn validate_policy(policy: &EligibilityPolicy) -> Result<(), TournamentPassError> {
    if policy.min_elo == 0
        || policy.min_elo > policy.max_elo
        || policy.max_eligibility_age_slots == 0
    {
        return Err(TournamentPassError::InvalidPolicy);
    }
    Ok(())
}

pub fn validate_rating(
    policy: &EligibilityPolicy,
    verified_elo: u32,
) -> Result<(), TournamentPassError> {
    if !(policy.min_elo..=policy.max_elo).contains(&verified_elo) {
        return Err(TournamentPassError::RatingOutsideRange);
    }
    Ok(())
}

pub fn enforce_pass_transfer(
    policy: &EligibilityPolicy,
    is_paused: bool,
    eligibility: Option<&EligibilityRecord>,
    transfer_context: &TransferContext,
    current_slot: u64,
) -> Result<(), TournamentPassError> {
    if is_paused {
        return Err(TournamentPassError::PassPaused);
    }
    if transfer_context.amount != 1 {
        return Err(TournamentPassError::InvalidTransferAmount);
    }
    if transfer_context.destination_balance_after != 1 {
        return Err(TournamentPassError::InvalidDestinationBalance);
    }
    let Some(eligibility) = eligibility else {
        return Err(TournamentPassError::RecipientNotEligible);
    };
    if eligibility.is_revoked {
        return Err(TournamentPassError::EligibilityRevoked);
    }
    match current_slot.checked_sub(eligibility.verified_at_slot) {
        Some(age_slots) if age_slots <= policy.max_eligibility_age_slots => {}
        _ => return Err(TournamentPassError::EligibilityExpired),
    }
    validate_rating(policy, eligibility.verified_elo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::prelude::Pubkey;

    const POLICY: EligibilityPolicy = EligibilityPolicy {
        min_elo: 1_500,
        max_elo: 1_800,
        max_eligibility_age_slots: 100,
    };
    const VERIFIED_AT: u64 = 200;

    fn record() -> EligibilityRecord {
        EligibilityRecord {
            bump: 255,
            mint: Pubkey::new_unique(),
            player: Pubkey::new_unique(),
            verified_elo: 1_600,
            verified_at_slot: VERIFIED_AT,
            is_revoked: false,
        }
    }

    fn transfer_context(amount: u64, destination_balance_after: u64) -> TransferContext {
        TransferContext {
            amount,
            source_token_account: Pubkey::new_unique(),
            mint: Pubkey::new_unique(),
            destination_token_account: Pubkey::new_unique(),
            owner_or_delegate: Pubkey::new_unique(),
            source_balance_after: 0,
            destination_balance_after,
        }
    }

    fn decide(
        is_paused: bool,
        eligibility: Option<&EligibilityRecord>,
        current_slot: u64,
    ) -> Result<(), TournamentPassError> {
        enforce_pass_transfer(
            &POLICY,
            is_paused,
            eligibility,
            &transfer_context(1, 1),
            current_slot,
        )
    }

    #[test]
    fn eligible_recipient_receives_pass() {
        assert_eq!(decide(false, Some(&record()), VERIFIED_AT + 1), Ok(()));
    }

    #[test]
    fn missing_record_is_refused() {
        assert_eq!(
            decide(false, None, VERIFIED_AT),
            Err(TournamentPassError::RecipientNotEligible)
        );
    }

    #[test]
    fn rating_bounds_are_inclusive() {
        let mut eligibility = record();
        for verified_elo in [POLICY.min_elo, POLICY.max_elo] {
            eligibility.verified_elo = verified_elo;
            assert_eq!(decide(false, Some(&eligibility), VERIFIED_AT), Ok(()));
        }
        for verified_elo in [POLICY.min_elo - 1, POLICY.max_elo + 1, 0, u32::MAX] {
            eligibility.verified_elo = verified_elo;
            assert_eq!(
                decide(false, Some(&eligibility), VERIFIED_AT),
                Err(TournamentPassError::RatingOutsideRange)
            );
        }
    }

    #[test]
    fn record_valid_through_max_age() {
        let eligibility = record();
        let last_valid_slot = VERIFIED_AT + POLICY.max_eligibility_age_slots;
        assert_eq!(decide(false, Some(&eligibility), last_valid_slot), Ok(()));
        assert_eq!(
            decide(false, Some(&eligibility), last_valid_slot + 1),
            Err(TournamentPassError::EligibilityExpired)
        );
        assert_eq!(
            decide(false, Some(&eligibility), VERIFIED_AT - 1),
            Err(TournamentPassError::EligibilityExpired)
        );
    }

    #[test]
    fn record_age_cannot_overflow() {
        let mut eligibility = record();
        eligibility.verified_at_slot = 0;
        assert_eq!(
            decide(false, Some(&eligibility), u64::MAX),
            Err(TournamentPassError::EligibilityExpired)
        );
        eligibility.verified_at_slot = u64::MAX;
        assert_eq!(decide(false, Some(&eligibility), u64::MAX), Ok(()));
    }

    #[test]
    fn revoked_record_is_refused() {
        let mut eligibility = record();
        eligibility.is_revoked = true;
        assert_eq!(
            decide(false, Some(&eligibility), VERIFIED_AT),
            Err(TournamentPassError::EligibilityRevoked)
        );
    }

    #[test]
    fn pause_refuses_eligible_recipient() {
        assert_eq!(
            decide(true, Some(&record()), VERIFIED_AT),
            Err(TournamentPassError::PassPaused)
        );
    }

    #[test]
    fn transfer_leaves_one_unit_in_destination() {
        let eligibility = record();
        let check = |amount, destination_balance_after| {
            enforce_pass_transfer(
                &POLICY,
                false,
                Some(&eligibility),
                &transfer_context(amount, destination_balance_after),
                VERIFIED_AT,
            )
        };
        assert_eq!(check(1, 1), Ok(()));
        for amount in [0, 2, u64::MAX] {
            assert_eq!(
                check(amount, 1),
                Err(TournamentPassError::InvalidTransferAmount)
            );
        }
        for destination_balance_after in [0, 2] {
            assert_eq!(
                check(1, destination_balance_after),
                Err(TournamentPassError::InvalidDestinationBalance)
            );
        }
    }

    #[test]
    fn policy_needs_ordered_range_and_age() {
        assert_eq!(validate_policy(&POLICY), Ok(()));
        assert_eq!(
            validate_policy(&EligibilityPolicy {
                min_elo: 1_800,
                max_elo: 1_800,
                max_eligibility_age_slots: 1,
            }),
            Ok(())
        );
        for invalid in [
            EligibilityPolicy {
                min_elo: 0,
                ..POLICY
            },
            EligibilityPolicy {
                min_elo: 1_801,
                ..POLICY
            },
            EligibilityPolicy {
                max_eligibility_age_slots: 0,
                ..POLICY
            },
        ] {
            assert_eq!(
                validate_policy(&invalid),
                Err(TournamentPassError::InvalidPolicy)
            );
        }
    }
}
