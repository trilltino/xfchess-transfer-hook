// Packs the canonical ExtraAccountMetaList bytes a hook declares, so Token-2022 passes the right accounts to Execute.

use anchor_lang::prelude::*;
use core::mem::{offset_of, size_of};
use spl_discriminator::SplDiscriminate;
use spl_tlv_account_resolution::account::ExtraAccountMeta;
use spl_transfer_hook_interface::{
    get_extra_account_metas_address_and_bump_seed, instruction::ExecuteInstruction,
};

use crate::error::TransferHookError;

/// PDA seed used for a hook's canonical extra-account metadata list.
pub const EXTRA_ACCOUNT_METAS_SEED: &[u8] = b"extra-account-metas";
/// Eight-byte SPL discriminator identifying Execute metadata in the TLV list.
pub const EXECUTE_DISCRIMINATOR: [u8; 8] = {
    let mut discriminator = [0; 8];
    discriminator.copy_from_slice(EXECUTE_DISCRIMINATOR_SLICE);
    discriminator
};
/// Slice form of [`EXECUTE_DISCRIMINATOR`], as defined by the SPL interface.
pub const EXECUTE_DISCRIMINATOR_SLICE: &[u8] =
    <ExecuteInstruction as SplDiscriminate>::SPL_DISCRIMINATOR_SLICE;

const EXTRA_ACCOUNT_META_LEN: usize = size_of::<ExtraAccountMeta>();
const TLV_LENGTH_OFFSET: usize = EXECUTE_DISCRIMINATOR.len();
const ENTRY_COUNT_OFFSET: usize = TLV_LENGTH_OFFSET + size_of::<u32>();
const EXTRA_ACCOUNT_META_LIST_HEADER_LEN: usize = ENTRY_COUNT_OFFSET + size_of::<u32>();
const EXTRA_ACCOUNT_META_IS_WRITABLE_OFFSET: usize = offset_of!(ExtraAccountMeta, is_writable);

const ENTRY_OWN_PDA: u8 = 1;
const ENTRY_EXTERNAL_PDA: u8 = 1 << 7;
const ADDRESS_CONFIG_END: usize = 1 + 32;
const SEED_LITERAL: u8 = 1;
const SEED_ACCOUNT_KEY: u8 = 3;
const SEED_ACCOUNT_DATA: u8 = 4;
/// Byte offset of the token-account owner in the base Token-2022 account layout.
pub const TOKEN_ACCOUNT_OWNER_OFFSET: u8 = 32;
const PUBKEY_LEN: u8 = 32;

#[must_use]
/// Returns the serialized byte length for a list containing `extra_account_count` entries.
pub const fn meta_list_len(extra_account_count: usize) -> usize {
    EXTRA_ACCOUNT_META_LIST_HEADER_LEN + EXTRA_ACCOUNT_META_LEN * extra_account_count
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A seed component used to derive an extra-account PDA.
pub enum ExtraAccountMetaSeed<'a> {
    /// Literal bytes included directly in the PDA seeds.
    Literal(&'a [u8]),
    /// The public key of an earlier account in the Execute account list.
    AccountKey {
        /// Zero-based account index in the list being resolved.
        index: u8,
    },
    /// A byte range read from an earlier account's data.
    AccountData {
        /// Zero-based account index whose data supplies the seed bytes.
        account_index: u8,
        /// Starting byte offset in that account's data.
        data_index: u8,
        /// Number of bytes to read.
        length: u8,
    },
}

#[must_use]
/// Builds metadata for a PDA owned by this hook program using a literal and an account key.
pub const fn pda_meta(
    literal: &[u8],
    account_index: u8,
    is_writable: bool,
) -> [u8; EXTRA_ACCOUNT_META_LEN] {
    pack_seeds(
        ENTRY_OWN_PDA,
        &[
            ExtraAccountMetaSeed::Literal(literal),
            ExtraAccountMetaSeed::AccountKey {
                index: account_index,
            },
        ],
        is_writable,
    )
}

#[must_use]
/// Builds metadata for a hook PDA seeded by a literal, mint key and token-account owner.
pub const fn owner_pda_meta(
    literal: &[u8],
    mint_index: u8,
    token_account_index: u8,
    is_writable: bool,
) -> [u8; EXTRA_ACCOUNT_META_LEN] {
    pack_seeds(
        ENTRY_OWN_PDA,
        &[
            ExtraAccountMetaSeed::Literal(literal),
            ExtraAccountMetaSeed::AccountKey { index: mint_index },
            ExtraAccountMetaSeed::AccountData {
                account_index: token_account_index,
                data_index: TOKEN_ACCOUNT_OWNER_OFFSET,
                length: PUBKEY_LEN,
            },
        ],
        is_writable,
    )
}

#[must_use]
/// Builds metadata for a PDA derived under another program in the account list.
pub const fn external_pda_meta(
    program_account_index: u8,
    seeds: &[ExtraAccountMetaSeed],
    is_writable: bool,
) -> [u8; EXTRA_ACCOUNT_META_LEN] {
    assert!(
        program_account_index < ENTRY_EXTERNAL_PDA,
        "program account index too large for a packed meta"
    );
    pack_seeds(
        ENTRY_EXTERNAL_PDA + program_account_index,
        seeds,
        is_writable,
    )
}

#[must_use]
/// Builds metadata for an extra account at a fixed public-key address.
pub const fn fixed_meta(address: &[u8; 32], is_writable: bool) -> [u8; EXTRA_ACCOUNT_META_LEN] {
    let mut meta = [0u8; EXTRA_ACCOUNT_META_LEN];
    let mut address_index = 0;
    while address_index < 32 {
        meta[1 + address_index] = address[address_index];
        address_index += 1;
    }
    meta[EXTRA_ACCOUNT_META_IS_WRITABLE_OFFSET] = is_writable as u8;
    meta
}

const fn pack_seeds(
    discriminator: u8,
    seeds: &[ExtraAccountMetaSeed],
    is_writable: bool,
) -> [u8; EXTRA_ACCOUNT_META_LEN] {
    let mut meta = [0u8; EXTRA_ACCOUNT_META_LEN];
    meta[0] = discriminator;
    let mut offset = 1;
    let mut seed_index = 0;
    while seed_index < seeds.len() {
        let packed_len = match seeds[seed_index] {
            ExtraAccountMetaSeed::Literal(bytes) => 2 + bytes.len(),
            ExtraAccountMetaSeed::AccountKey { .. } => 2,
            ExtraAccountMetaSeed::AccountData { .. } => 4,
        };
        assert!(
            offset + packed_len <= ADDRESS_CONFIG_END,
            "seeds too long for a packed meta"
        );
        match seeds[seed_index] {
            ExtraAccountMetaSeed::Literal(bytes) => {
                meta[offset] = SEED_LITERAL;
                meta[offset + 1] = bytes.len() as u8;
                let mut byte_index = 0;
                while byte_index < bytes.len() {
                    meta[offset + 2 + byte_index] = bytes[byte_index];
                    byte_index += 1;
                }
            }
            ExtraAccountMetaSeed::AccountKey { index } => {
                meta[offset] = SEED_ACCOUNT_KEY;
                meta[offset + 1] = index;
            }
            ExtraAccountMetaSeed::AccountData {
                account_index,
                data_index,
                length,
            } => {
                meta[offset] = SEED_ACCOUNT_DATA;
                meta[offset + 1] = account_index;
                meta[offset + 2] = data_index;
                meta[offset + 3] = length;
            }
        }
        offset += packed_len;
        seed_index += 1;
    }
    meta[EXTRA_ACCOUNT_META_IS_WRITABLE_OFFSET] = is_writable as u8;
    meta
}

#[must_use]
/// Packs the discriminator, TLV header and entries into canonical SPL metadata bytes.
pub const fn canonical_meta_list<const E: usize, const N: usize>(
    metas: [[u8; EXTRA_ACCOUNT_META_LEN]; E],
) -> [u8; N] {
    assert!(N == meta_list_len(E), "N must be meta_list_len(E)");
    let mut extra_account_meta_list = [0u8; N];
    let mut byte_index = 0;
    while byte_index < EXECUTE_DISCRIMINATOR.len() {
        extra_account_meta_list[byte_index] = EXECUTE_DISCRIMINATOR[byte_index];
        byte_index += 1;
    }
    let tlv_len = ((size_of::<u32>() + EXTRA_ACCOUNT_META_LEN * E) as u32).to_le_bytes();
    let entry_count = (E as u32).to_le_bytes();
    let mut byte_index = 0;
    while byte_index < size_of::<u32>() {
        extra_account_meta_list[TLV_LENGTH_OFFSET + byte_index] = tlv_len[byte_index];
        extra_account_meta_list[ENTRY_COUNT_OFFSET + byte_index] = entry_count[byte_index];
        byte_index += 1;
    }
    let mut meta_index = 0;
    while meta_index < E {
        let mut meta_byte_index = 0;
        while meta_byte_index < EXTRA_ACCOUNT_META_LEN {
            extra_account_meta_list[EXTRA_ACCOUNT_META_LIST_HEADER_LEN
                + meta_index * EXTRA_ACCOUNT_META_LEN
                + meta_byte_index] = metas[meta_index][meta_byte_index];
            meta_byte_index += 1;
        }
        meta_index += 1;
    }
    extra_account_meta_list
}

#[must_use]
/// Returns the entry count implied by a correctly shaped metadata-list byte slice.
pub(crate) const fn declared_extra_account_count(extra_account_meta_list: &[u8]) -> Option<usize> {
    let list_len = extra_account_meta_list.len();
    if list_len < EXTRA_ACCOUNT_META_LIST_HEADER_LEN
        || !(list_len - EXTRA_ACCOUNT_META_LIST_HEADER_LEN).is_multiple_of(EXTRA_ACCOUNT_META_LEN)
    {
        return None;
    }
    Some((list_len - EXTRA_ACCOUNT_META_LIST_HEADER_LEN) / EXTRA_ACCOUNT_META_LEN)
}

#[must_use]
/// Returns whether the declared entry at `extra_index` is writable.
pub(crate) fn is_declared_writable(extra_account_meta_list: &[u8], extra_index: usize) -> bool {
    extra_account_meta_list.get(
        EXTRA_ACCOUNT_META_LIST_HEADER_LEN
            + extra_index * EXTRA_ACCOUNT_META_LEN
            + EXTRA_ACCOUNT_META_IS_WRITABLE_OFFSET,
    ) == Some(&1)
}

#[must_use]
/// Derives the canonical metadata-list PDA and bump for a mint and hook program.
pub fn meta_list_address(mint: &Pubkey, program_id: &Pubkey) -> (Pubkey, u8) {
    get_extra_account_metas_address_and_bump_seed(mint, program_id)
}

/// Copies canonical metadata bytes into a preallocated metadata-list account.
///
/// The caller is responsible for validating the account address, owner and privileges.
pub fn write_meta_list(
    extra_account_meta_list: &AccountInfo,
    canonical_meta_list: &[u8],
) -> Result<()> {
    let mut list_data = extra_account_meta_list.try_borrow_mut_data()?;
    if list_data.len() != canonical_meta_list.len() {
        return err!(TransferHookError::InvalidExtraAccountMetaList);
    }
    list_data.copy_from_slice(canonical_meta_list);
    Ok(())
}
