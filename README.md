# XFChess Transfer Hook

A [Token-2022 Transfer Hook](https://solana.com/docs/tokens/extensions/transfer-hook) written with Anchor. It controls who may receive a **one-unit pass token**: a transfer succeeds only if the receiving wallet has a current, in-range rating record issued by the pass's registrar. The original use is a chess tournament pass, rated by Elo, but the rule is a general "registrar-attested eligibility with expiry and revocation".

**Unofficial and unaudited.** Not affiliated with Raydium or any exchange, and not checked against any AMM. Do not use it with real funds without your own review and audit.

This program was extracted from the author's Transfer Hook Kit workspace (source commit `787e47f`). `programs/xfchess-pass/` is unchanged from that commit, and `hook-kit/` is a verbatim copy of the kit's shared validation crate, vendored so this repository builds on its own.

## What it enforces

For every `TransferChecked` of the pass, Token-2022 calls this program's `execute`, which refuses the transfer unless all of these hold:

- the pass is not paused;
- exactly one unit moves, and the destination holds exactly one afterwards;
- the destination wallet has an eligibility record for this mint that is not revoked, is no older than `max_eligibility_age_slots`, and holds a rating within `min_elo..=max_elo`;
- the destination token account has the `ImmutableOwner` extension, so its owner (the key the record is looked up by) cannot change later;
- the shared Transfer Hook checks pass (genuine Token-2022 transfer of this mint, canonical `ExtraAccountMetaList`, exact extra accounts).

| Instruction | Signer | Effect |
| --- | --- | --- |
| `initialize` | mint's Transfer Hook authority | Creates the per-mint config and the `ExtraAccountMetaList`. |
| `issue_eligibility` | registrar | Creates a wallet's record with its verified rating. |
| `refresh_eligibility` | registrar | Updates the rating, resets the verification slot and clears any revocation. |
| `revoke_eligibility` | registrar | Marks a record revoked. |
| `set_paused` | registrar | Pauses or resumes every pass transfer. |
| `execute` | none (called by Token-2022) | The transfer check above. |

`initialize` requires a mint with 0 decimals, this program as its hook, a **revoked mint authority** (nobody can mint more passes), and no confidential-transfer extension.

PDAs: config `["pass-config", mint]`, record `["eligibility", mint, wallet]`. Error codes are `0xD001`-`0xD00D`. The interface is in [`idl/xfchess_pass.json`](idl/xfchess_pass.json); off-chain instruction builders are in [`client.rs`](programs/xfchess-pass/src/client.rs). The pure rule is [`rule.rs`](programs/xfchess-pass/src/rule.rs).

## Prerequisites

- Rust 1.99.1 with `rustfmt` and `clippy` (selected by `rust-toolchain.toml`).
- To compile for Solana (SBF): `cargo-build-sbf` 4.4.0 with platform tools v1.58.
- To regenerate the IDL: Anchor CLI 1.2.1.

No wallet or validator is needed for the tests.

## Build and test

```bash
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace

# SBF build (writes target/deploy/xfchess_pass.so)
cargo build-sbf --arch v0 --tools-version v1.58 --manifest-path programs/xfchess-pass/Cargo.toml --sbf-out-dir "$PWD/target/deploy" -- --locked

# The committed IDL should match the source
anchor idl build --program-name xfchess_pass --out target/idl/xfchess_pass.json -- --locked
diff target/idl/xfchess_pass.json idl/xfchess_pass.json
```

The tests are native unit tests: the rule (accepted case, each rejection, rating and age boundaries), account layouts and discriminators, the `ExtraAccountMetaList` bytes against `spl-tlv-account-resolution`, the instruction builders, error-code uniqueness, and that the IDL and `Anchor.toml` carry the declared program ID. They do **not** run a real Token-2022 transfer. Delegate transfers, whole-transaction rollback and compute cost still need a local validator or Devnet run.

## Using it with a mint

1. Create a Token-2022 mint with 0 decimals and the Transfer Hook extension pointing at this program, then revoke its mint authority.
2. Call `initialize` as the hook authority, naming the registrar and the rating/age policy.
3. The registrar calls `issue_eligibility` for each wallet that may hold the pass.
4. Holders use token accounts with `ImmutableOwner`. Transfers must go through `TransferChecked` with the hook's extra accounts resolved (the config PDA, then the destination wallet's record PDA); a client that uses the SPL transfer-hook helpers does this automatically.

## Program ID and deployment

`declare_id!` is `4sFBJCjU3QvoWZCfxdHJcGBCPjToTy2BzNK23wmRPaw9`, the ID of an existing **Devnet** deployment of this program. This repository contains no keypair for it and does not deploy anything.

- Only the holder of that program's keypair or upgrade authority can deploy or upgrade it. Check the current authority with `solana program show <id> --url devnet`.
- The Devnet binary is upgradeable, so the code running there is whatever its authority last deployed, not necessarily this source.
- To run your own copy, change `declare_id!` in `programs/xfchess-pass/src/lib.rs`, `Anchor.toml` and `idl/xfchess_pass.json` to a keypair you control, then rebuild and regenerate the IDL.
- Do not use this ID or this code on mainnet without an independent audit and a decision on who holds the upgrade authority.

## Security assumptions and limitations

- **The registrar is trusted.** The hook cannot know a rating is true. A compromised or dishonest registrar can issue records to anyone, revoke anyone, or pause all transfers.
- **Only the receiver is checked.** The sender needs no record, so an eligible holder can pass the token to any other eligible wallet. The hook does not see who is acting or why.
- **Mint and burn are outside the hook.** The hook does not run on mint, burn or `SetAuthority`. That is why `initialize` requires a revoked mint authority; the holder can still burn their pass.
- **Eligibility is time-limited by slots.** A record older than `max_eligibility_age_slots` is refused until the registrar refreshes it.
- **One config per mint, no on-chain upgrade path for its layout.** Seeds, account layouts, discriminators and error codes are treated as frozen; the tests pin them.
- **Untested against real programs.** AMM or marketplace compatibility is unverified: such a program must forward the hook's extra accounts on every transfer it makes.
- Layout and field names (`tournament_*`, `elo`) reflect the original chess use and are kept to preserve the on-chain interface.

## Licence

[MIT](LICENSE).
