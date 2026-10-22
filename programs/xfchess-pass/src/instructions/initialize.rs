// Setup instruction that stores the pass policy and registrar and writes its ExtraAccountMetaList.

use anchor_lang::prelude::*;
use anchor_spl::token_interface::Mint;
use hook_kit::EXTRA_ACCOUNT_METAS_SEED;

use crate::{
    error::TournamentPassError,
    rule::{self, EligibilityPolicy},
    state::{
        EXTRA_ACCOUNT_META_LIST, TOURNAMENT_PASS_CONFIG_SEED, TOURNAMENT_PASS_CONFIG_VERSION,
        TournamentPassConfig,
    },
};

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = payer,
        space = 8 + TournamentPassConfig::INIT_SPACE,
        seeds = [TOURNAMENT_PASS_CONFIG_SEED, mint.key().as_ref()],
        bump,
    )]
    pub tournament_pass_config: Account<'info, TournamentPassConfig>,

    /// CHECK: canonical SPL TLV list, created and written here; no Anchor discriminator.
    #[account(
        init,
        payer = payer,
        space = EXTRA_ACCOUNT_META_LIST.len(),
        seeds = [EXTRA_ACCOUNT_METAS_SEED, mint.key().as_ref()],
        bump,
    )]
    pub extra_account_meta_list: UncheckedAccount<'info>,

    pub mint: Box<InterfaceAccount<'info, Mint>>,

    pub transfer_hook_authority: Signer<'info>,

    #[account(mut)]
    pub payer: Signer<'info>,

    pub system_program: Program<'info, System>,
}

pub fn initialize(
    ctx: Context<Initialize>,
    tournament_id: u64,
    registrar: Pubkey,
    policy: EligibilityPolicy,
) -> Result<()> {
    let mint_info = ctx.accounts.mint.to_account_info();
    let transfer_hook_mint = hook_kit::validate_hook_setup(
        ctx.program_id,
        &mint_info,
        &ctx.accounts.transfer_hook_authority.to_account_info(),
    )?;
    require_eq!(
        transfer_hook_mint.decimals,
        0,
        TournamentPassError::MintNotIndivisible
    );
    // Passes are tickets: nobody may mint more once the pass is live.
    transfer_hook_mint.require_mint_authority_revoked()?;
    require!(
        !hook_kit::has_confidential_extension(&mint_info)?,
        TournamentPassError::UnsupportedMintExtension
    );
    rule::validate_policy(&policy)?;
    require_keys_neq!(
        registrar,
        Pubkey::default(),
        TournamentPassError::InvalidPolicy
    );

    ctx.accounts
        .tournament_pass_config
        .set_inner(TournamentPassConfig {
            version: TOURNAMENT_PASS_CONFIG_VERSION,
            bump: ctx.bumps.tournament_pass_config,
            extra_account_meta_list_bump: ctx.bumps.extra_account_meta_list,
            mint: ctx.accounts.mint.key(),
            tournament_id,
            registrar,
            policy,
            is_paused: false,
        });
    hook_kit::write_meta_list(
        &ctx.accounts.extra_account_meta_list,
        &EXTRA_ACCOUNT_META_LIST,
    )
}
