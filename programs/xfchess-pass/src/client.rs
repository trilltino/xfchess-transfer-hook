// Host-side instruction builders for XFChess Pass, used by clients, tests and examples.

use anchor_lang::{
    InstructionData, ToAccountMetas, prelude::*, solana_program::instruction::Instruction,
};

use crate::{
    rule::EligibilityPolicy,
    state::{eligibility_record_address, meta_list_address, tournament_pass_config_address},
};

#[must_use]
pub fn initialize_instruction(
    program_id: &Pubkey,
    mint: &Pubkey,
    transfer_hook_authority: &Pubkey,
    payer: &Pubkey,
    tournament_id: u64,
    registrar: &Pubkey,
    policy: EligibilityPolicy,
) -> Instruction {
    Instruction {
        program_id: *program_id,
        accounts: crate::accounts::Initialize {
            tournament_pass_config: tournament_pass_config_address(mint, program_id).0,
            extra_account_meta_list: meta_list_address(mint, program_id).0,
            mint: *mint,
            transfer_hook_authority: *transfer_hook_authority,
            payer: *payer,
            system_program: System::id(),
        }
        .to_account_metas(None),
        data: crate::instruction::Initialize {
            tournament_id,
            registrar: *registrar,
            policy,
        }
        .data(),
    }
}

#[must_use]
pub fn issue_eligibility_instruction(
    program_id: &Pubkey,
    mint: &Pubkey,
    registrar: &Pubkey,
    payer: &Pubkey,
    player: &Pubkey,
    verified_elo: u32,
) -> Instruction {
    Instruction {
        program_id: *program_id,
        accounts: crate::accounts::IssueEligibility {
            tournament_pass_config: tournament_pass_config_address(mint, program_id).0,
            eligibility_record: eligibility_record_address(mint, player, program_id).0,
            player: *player,
            registrar: *registrar,
            payer: *payer,
            system_program: System::id(),
        }
        .to_account_metas(None),
        data: crate::instruction::IssueEligibility { verified_elo }.data(),
    }
}

#[must_use]
pub fn refresh_eligibility_instruction(
    program_id: &Pubkey,
    mint: &Pubkey,
    registrar: &Pubkey,
    player: &Pubkey,
    verified_elo: u32,
) -> Instruction {
    Instruction {
        program_id: *program_id,
        accounts: crate::accounts::RefreshEligibility {
            tournament_pass_config: tournament_pass_config_address(mint, program_id).0,
            eligibility_record: eligibility_record_address(mint, player, program_id).0,
            registrar: *registrar,
        }
        .to_account_metas(None),
        data: crate::instruction::RefreshEligibility { verified_elo }.data(),
    }
}

#[must_use]
pub fn revoke_eligibility_instruction(
    program_id: &Pubkey,
    mint: &Pubkey,
    registrar: &Pubkey,
    player: &Pubkey,
) -> Instruction {
    Instruction {
        program_id: *program_id,
        accounts: crate::accounts::RevokeEligibility {
            tournament_pass_config: tournament_pass_config_address(mint, program_id).0,
            eligibility_record: eligibility_record_address(mint, player, program_id).0,
            registrar: *registrar,
        }
        .to_account_metas(None),
        data: crate::instruction::RevokeEligibility {}.data(),
    }
}

#[must_use]
pub fn set_paused_instruction(
    program_id: &Pubkey,
    mint: &Pubkey,
    registrar: &Pubkey,
    is_paused: bool,
) -> Instruction {
    Instruction {
        program_id: *program_id,
        accounts: crate::accounts::SetPaused {
            tournament_pass_config: tournament_pass_config_address(mint, program_id).0,
            registrar: *registrar,
        }
        .to_account_metas(None),
        data: crate::instruction::SetPaused { is_paused }.data(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anchor_lang::Discriminator;

    const POLICY: EligibilityPolicy = EligibilityPolicy {
        min_elo: 1_500,
        max_elo: 1_800,
        max_eligibility_age_slots: 100,
    };

    fn privileges(instruction: &Instruction) -> Vec<(Pubkey, bool, bool)> {
        instruction
            .accounts
            .iter()
            .map(|meta| (meta.pubkey, meta.is_signer, meta.is_writable))
            .collect()
    }

    #[test]
    fn discriminators_match_anchor_hashes() {
        assert_eq!(
            crate::instruction::Initialize::DISCRIMINATOR,
            [175, 175, 109, 31, 13, 152, 155, 237]
        );
        assert_eq!(
            crate::instruction::IssueEligibility::DISCRIMINATOR,
            [29, 88, 236, 137, 74, 17, 193, 144]
        );
        assert_eq!(
            crate::instruction::RefreshEligibility::DISCRIMINATOR,
            [18, 227, 118, 61, 5, 162, 20, 209]
        );
        assert_eq!(
            crate::instruction::RevokeEligibility::DISCRIMINATOR,
            [86, 156, 4, 240, 195, 239, 201, 42]
        );
        assert_eq!(
            crate::instruction::SetPaused::DISCRIMINATOR,
            [91, 60, 125, 192, 176, 225, 166, 218]
        );
    }

    #[test]
    fn execute_uses_spl_discriminator() {
        assert_eq!(
            crate::instruction::Execute::DISCRIMINATOR,
            hook_kit::EXECUTE_DISCRIMINATOR_SLICE
        );
        assert_eq!(
            crate::instruction::Execute { amount: 9 }.data(),
            spl_transfer_hook_interface::instruction::TransferHookInstruction::Execute {
                amount: 9
            }
            .pack()
        );
    }

    #[test]
    fn initialize_args_and_accounts() {
        let [mint, transfer_hook_authority, payer, registrar] =
            std::array::from_fn(|_| Pubkey::new_unique());
        let instruction = initialize_instruction(
            &crate::ID,
            &mint,
            &transfer_hook_authority,
            &payer,
            7,
            &registrar,
            POLICY,
        );
        let mut expected_data = crate::instruction::Initialize::DISCRIMINATOR.to_vec();
        expected_data.extend(7u64.to_le_bytes());
        expected_data.extend(registrar.to_bytes());
        expected_data.extend(1_500u32.to_le_bytes());
        expected_data.extend(1_800u32.to_le_bytes());
        expected_data.extend(100u64.to_le_bytes());
        assert_eq!(instruction.data, expected_data);
        assert_eq!(
            privileges(&instruction),
            vec![
                (
                    tournament_pass_config_address(&mint, &crate::ID).0,
                    false,
                    true
                ),
                (meta_list_address(&mint, &crate::ID).0, false, true),
                (mint, false, false),
                (transfer_hook_authority, true, false),
                (payer, true, true),
                (System::id(), false, false),
            ]
        );
    }

    #[test]
    fn issue_needs_registrar_and_payer_not_player() {
        let [mint, registrar, payer, player] = std::array::from_fn(|_| Pubkey::new_unique());
        let instruction =
            issue_eligibility_instruction(&crate::ID, &mint, &registrar, &payer, &player, 1_600);
        let mut expected_data = crate::instruction::IssueEligibility::DISCRIMINATOR.to_vec();
        expected_data.extend(1_600u32.to_le_bytes());
        assert_eq!(instruction.data, expected_data);
        assert_eq!(
            privileges(&instruction),
            vec![
                (
                    tournament_pass_config_address(&mint, &crate::ID).0,
                    false,
                    false
                ),
                (
                    eligibility_record_address(&mint, &player, &crate::ID).0,
                    false,
                    true
                ),
                (player, false, false),
                (registrar, true, false),
                (payer, true, true),
                (System::id(), false, false),
            ]
        );
    }

    #[test]
    fn refresh_revoke_pause_are_registrar_signed() {
        let [mint, registrar, player] = std::array::from_fn(|_| Pubkey::new_unique());
        let config = tournament_pass_config_address(&mint, &crate::ID).0;
        let record = eligibility_record_address(&mint, &player, &crate::ID).0;

        let refresh =
            refresh_eligibility_instruction(&crate::ID, &mint, &registrar, &player, 1_700);
        assert_eq!(
            &refresh.data[..8],
            crate::instruction::RefreshEligibility::DISCRIMINATOR
        );
        assert_eq!(&refresh.data[8..], &1_700u32.to_le_bytes());
        let revoke = revoke_eligibility_instruction(&crate::ID, &mint, &registrar, &player);
        assert_eq!(
            revoke.data,
            crate::instruction::RevokeEligibility::DISCRIMINATOR
        );
        for instruction in [&refresh, &revoke] {
            assert_eq!(
                privileges(instruction),
                vec![
                    (config, false, false),
                    (record, false, true),
                    (registrar, true, false)
                ]
            );
        }

        for is_paused in [false, true] {
            let pause = set_paused_instruction(&crate::ID, &mint, &registrar, is_paused);
            assert_eq!(
                &pause.data[..8],
                crate::instruction::SetPaused::DISCRIMINATOR
            );
            assert_eq!(&pause.data[8..], &[u8::from(is_paused)]);
            assert_eq!(
                privileges(&pause),
                vec![(config, false, true), (registrar, true, false)]
            );
        }
    }
}
