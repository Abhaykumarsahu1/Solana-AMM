use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{mint_to, transfer, Mint, MintTo, Token, TokenAccount, Transfer},
};
use constant_product_curve::ConstantProduct;

use crate::{error::AmmError, state::AmmConfig};

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub mint_x: Account<'info, Mint>,
    pub mint_y: Account<'info, Mint>,

    #[account(
        has_one = mint_x,
        has_one = mint_y,
        seeds = [b"amm_config", config.seed.to_le_bytes().as_ref()],
        bump = config.bump,
    )]
    pub config: Account<'info, AmmConfig>,

    #[account(
        mut,
        seeds = [b"lp", config.key().as_ref()],
        bump,
    )]
    pub lp_mint: Account<'info, Mint>,

    #[account(mut, associated_token::mint = mint_x, associated_token::authority = config)]
    pub vault_x: Account<'info, TokenAccount>,

    #[account(mut, associated_token::mint = mint_y, associated_token::authority = config)]
    pub vault_y: Account<'info, TokenAccount>,

    #[account(mut, associated_token::mint = mint_x, associated_token::authority = user)]
    pub user_x: Account<'info, TokenAccount>,

    #[account(mut, associated_token::mint = mint_y, associated_token::authority = user)]
    pub user_y: Account<'info, TokenAccount>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = lp_mint,
        associated_token::authority = user,
    )]
    pub user_lp: Account<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handle_deposit(ctx: Context<Deposit>, amount: u64, max_x: u64, max_y: u64) -> Result<()> {
    require!(!ctx.accounts.config.locked, AmmError::PoolLocked);
    require!(amount != 0, AmmError::InvalidAmount);

    let (x_amount, y_amount) = if ctx.accounts.lp_mint.supply == 0
        && ctx.accounts.vault_x.amount == 0
        && ctx.accounts.vault_y.amount == 0
    {
        (max_x, max_y)
    } else {
        let amounts = ConstantProduct::xy_deposit_amounts_from_l(
            ctx.accounts.vault_x.amount,
            ctx.accounts.vault_y.amount,
            ctx.accounts.lp_mint.supply,
            amount,
            6,
        )
        .map_err(|_| AmmError::InvalidAmount)?;

        require!(amounts.x <= max_x && amounts.y <= max_y, AmmError::SlippageExceeded);
        (amounts.x, amounts.y)
    };

    transfer_tokens(&ctx.accounts.token_program, &ctx.accounts.user_x, &ctx.accounts.vault_x, &ctx.accounts.user, x_amount)?;
    transfer_tokens(&ctx.accounts.token_program, &ctx.accounts.user_y, &ctx.accounts.vault_y, &ctx.accounts.user, y_amount)?;
    mint_lp_tokens(&ctx.accounts.token_program, &ctx.accounts.lp_mint, &ctx.accounts.user_lp, &ctx.accounts.config, amount)?;

    Ok(())
}

fn transfer_tokens<'info>(
    token_program: &Program<'info, Token>,
    from: &Account<'info, TokenAccount>,
    to: &Account<'info, TokenAccount>,
    authority: &Signer<'info>,
    amount: u64,
) -> Result<()> {
    let cpi_accounts = Transfer { from: from.to_account_info(), to: to.to_account_info(), authority: authority.to_account_info() };
    transfer(CpiContext::new(token_program.key(), cpi_accounts), amount)
}

fn mint_lp_tokens<'info>(
    token_program: &Program<'info, Token>,
    lp_mint: &Account<'info, Mint>,
    user_lp: &Account<'info, TokenAccount>,
    config: &Account<'info, AmmConfig>,
    amount: u64,
) -> Result<()> {
    let cpi_accounts = MintTo { mint: lp_mint.to_account_info(), to: user_lp.to_account_info(), authority: config.to_account_info() };
    let seed = config.seed.to_le_bytes();
    let signer_seeds: &[&[&[u8]]] = &[&[b"amm_config", seed.as_ref(), &[config.bump]]];
    let cpi_ctx = CpiContext::new_with_signer(token_program.key(), cpi_accounts, signer_seeds);
    mint_to(cpi_ctx, amount)
}