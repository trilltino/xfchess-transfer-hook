// Seeds, the canonical ExtraAccountMetaList and PDA helpers for XFChess Pass's state.

mod eligibility_record;
mod tournament_pass_config;

use anchor_lang::prelude::*;
use hook_kit::{canonical_meta_list, meta_list_len, owner_pda_meta, pda_meta};

pub use eligibility_record::EligibilityRecord;
pub use hook_kit::meta_list_address;
pub use tournament_pass_config::TournamentPassConfig;

pub const TOURNAMENT_PASS_CONFIG_SEED: &[u8] = b"pass-config";
pub const ELIGIBILITY_RECORD_SEED: &[u8] = b"eligibility";
pub const TOURNAMENT_PASS_CONFIG_VERSION: u8 = 1;

pub const EXTRA_ACCOUNT_META_LIST: [u8; meta_list_len(2)] = canonical_meta_list([
    pda_meta(TOURNAMENT_PASS_CONFIG_SEED, 1, false),
    owner_pda_meta(ELIGIBILITY_RECORD_SEED, 1, 2, false),
]);

#[must_use]
pub fn tournament_pass_config_address(mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[TOURNAMENT_PASS_CONFIG_SEED, mint.as_ref()], program_id)
}

#[must_use]
pub fn eligibility_record_address(
    mint: &Pubkey,
    player: &Pubkey,
    program_id: &Pubkey,
) -> (Pubkey, u8) {
    Pubkey::find_program_address(
        &[ELIGIBILITY_RECORD_SEED, mint.as_ref(), player.as_ref()],
        program_id,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rule::EligibilityPolicy;
    use anchor_lang::{AccountDeserialize, AccountSerialize, Discriminator, Space};
    use spl_tlv_account_resolution::{
        account::ExtraAccountMeta, seeds::Seed, state::ExtraAccountMetaList,
    };
    use spl_transfer_hook_interface::instruction::ExecuteInstruction;

    fn tournament_pass_config() -> TournamentPassConfig {
        TournamentPassConfig {
            version: TOURNAMENT_PASS_CONFIG_VERSION,
            bump: 254,
            extra_account_meta_list_bump: 253,
            mint: Pubkey::new_unique(),
            tournament_id: 7,
            registrar: Pubkey::new_unique(),
            policy: EligibilityPolicy {
                min_elo: 1_500,
                max_elo: 1_800,
                max_eligibility_age_slots: 100,
            },
            is_paused: true,
        }
    }

    fn eligibility_record() -> EligibilityRecord {
        EligibilityRecord {
            bump: 252,
            mint: Pubkey::new_unique(),
            player: Pubkey::new_unique(),
            verified_elo: 1_600,
            verified_at_slot: u64::MAX,
            is_revoked: true,
        }
    }

    #[test]
    fn list_matches_spl_for_both_metas() {
        let config_meta = ExtraAccountMeta::new_with_seeds(
            &[
                Seed::Literal {
                    bytes: TOURNAMENT_PASS_CONFIG_SEED.to_vec(),
                },
                Seed::AccountKey { index: 1 },
            ],
            false,
            false,
        )
        .expect("config meta");
        let record_meta = ExtraAccountMeta::new_with_seeds(
            &[
                Seed::Literal {
                    bytes: ELIGIBILITY_RECORD_SEED.to_vec(),
                },
                Seed::AccountKey { index: 1 },
                Seed::AccountData {
                    account_index: 2,
                    data_index: 32,
                    length: 32,
                },
            ],
            false,
            false,
        )
        .expect("record meta");
        let mut spl_list = vec![0u8; ExtraAccountMetaList::size_of(2).expect("list size")];
        ExtraAccountMetaList::init::<ExecuteInstruction>(
            &mut spl_list,
            &[config_meta, record_meta],
        )
        .expect("list");
        assert_eq!(EXTRA_ACCOUNT_META_LIST.to_vec(), spl_list);
        assert_eq!(EXTRA_ACCOUNT_META_LIST.len(), 86);
    }

    #[test]
    fn config_round_trips_and_rejects_bad_data() {
        assert_eq!(8 + TournamentPassConfig::INIT_SPACE, 100);
        let config = tournament_pass_config();
        let mut config_data = Vec::new();
        config.try_serialize(&mut config_data).expect("serialize");
        assert_eq!(config_data.len(), 100);
        assert_eq!(&config_data[..8], TournamentPassConfig::DISCRIMINATOR);
        assert_eq!(
            TournamentPassConfig::try_deserialize(&mut config_data.as_slice()).expect("config"),
            config
        );
        assert!(TournamentPassConfig::try_deserialize(&mut &config_data[..99]).is_err());
        config_data[0] ^= 1;
        assert!(TournamentPassConfig::try_deserialize(&mut config_data.as_slice()).is_err());
    }

    #[test]
    fn record_round_trips_and_is_not_a_config() {
        assert_eq!(8 + EligibilityRecord::INIT_SPACE, 86);
        let record = eligibility_record();
        let mut record_data = Vec::new();
        record.try_serialize(&mut record_data).expect("serialize");
        assert_eq!(record_data.len(), 86);
        assert_eq!(&record_data[..8], EligibilityRecord::DISCRIMINATOR);
        assert_eq!(
            EligibilityRecord::try_deserialize(&mut record_data.as_slice()).expect("record"),
            record
        );
        assert!(EligibilityRecord::try_deserialize(&mut &record_data[..85]).is_err());
        assert!(TournamentPassConfig::try_deserialize(&mut record_data.as_slice()).is_err());
    }

    #[test]
    fn discriminators_match_anchor_hashes() {
        assert_eq!(
            TournamentPassConfig::DISCRIMINATOR,
            [55, 121, 229, 33, 132, 84, 144, 61]
        );
        assert_eq!(
            EligibilityRecord::DISCRIMINATOR,
            [63, 254, 108, 207, 86, 176, 56, 170]
        );
    }

    #[test]
    fn field_order_is_frozen() {
        let mut config_data = Vec::new();
        TournamentPassConfig {
            version: 2,
            bump: 3,
            extra_account_meta_list_bump: 5,
            mint: Pubkey::new_from_array([11; 32]),
            tournament_id: 13,
            registrar: Pubkey::new_from_array([17; 32]),
            policy: EligibilityPolicy {
                min_elo: 19,
                max_elo: 23,
                max_eligibility_age_slots: 29,
            },
            is_paused: true,
        }
        .try_serialize(&mut config_data)
        .expect("serialize");
        let mut expected = TournamentPassConfig::DISCRIMINATOR.to_vec();
        expected.extend([2, 3, 5]);
        expected.extend([11; 32]);
        expected.extend(13u64.to_le_bytes());
        expected.extend([17; 32]);
        expected.extend(19u32.to_le_bytes());
        expected.extend(23u32.to_le_bytes());
        expected.extend(29u64.to_le_bytes());
        expected.push(1);
        assert_eq!(config_data, expected);
    }

    #[test]
    fn addresses_are_documented_pdas() {
        let (mint, player, program_id) = (
            Pubkey::new_unique(),
            Pubkey::new_unique(),
            Pubkey::new_unique(),
        );
        let (config, config_bump) = tournament_pass_config_address(&mint, &program_id);
        assert_eq!(
            Pubkey::create_program_address(
                &[b"pass-config", mint.as_ref(), &[config_bump]],
                &program_id
            ),
            Ok(config)
        );
        let (record, record_bump) = eligibility_record_address(&mint, &player, &program_id);
        assert_eq!(
            Pubkey::create_program_address(
                &[
                    b"eligibility",
                    mint.as_ref(),
                    player.as_ref(),
                    &[record_bump]
                ],
                &program_id
            ),
            Ok(record)
        );
        assert_ne!(
            record,
            eligibility_record_address(&mint, &Pubkey::new_unique(), &program_id).0
        );
    }
}
