use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{
        burn,
        transfer,
        Burn,
        Mint,
        Token,
        TokenAccount,
        Transfer,
    },
};
use constant_product_curve::ConstantProduct;

use crate::{error::AmmError, state::AmmConfig};

#[derive(Accounts)]
pub struct Withdraw<'info> {
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

    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = config,
    )]
    pub vault_x: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = config,
    )]
    pub vault_y: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority = user,
    )]
    pub user_x: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = user,
    )]
    pub user_y: Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = lp_mint,
        associated_token::authority = user,
    )]
    pub user_lp: Account<'info, TokenAccount>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(
        &mut self,
        amount: u64,
        min_x: u64,
        min_y: u64,
    ) -> Result<()> {
        require!(!self.config.locked, AmmError::PoolLocked);
        require!(amount > 0, AmmError::InvalidAmount);

        require!(
            self.user_lp.amount >= amount,
            AmmError::InsufficientFunds
        );

        let amounts = ConstantProduct::xy_withdraw_amounts_from_l(
            self.vault_x.amount,
            self.vault_y.amount,
            self.lp_mint.supply,
            amount,
            6,
        )
        .map_err(|_| AmmError::InvalidAmount)?;

        require!(
            amounts.x >= min_x && amounts.y >= min_y,
            AmmError::SlippageExceeded
        );

        self.burn_lp_tokens(amount)?;

        self.withdraw_tokens(true, amounts.x)?;
        self.withdraw_tokens(false, amounts.y)?;

        Ok(())
    }

    fn burn_lp_tokens(&mut self, amount: u64) -> Result<()> {
        let cpi_accounts = Burn {
            mint: self.lp_mint.to_account_info(),
            from: self.user_lp.to_account_info(),
            authority: self.user.to_account_info(),
        };

        burn(
            CpiContext::new(
                self.token_program.key(),
                cpi_accounts,
            ),
            amount,
        )
    }

    fn withdraw_tokens(
        &mut self,
        is_x: bool,
        amount: u64,
    ) -> Result<()> {
        let (from, to) = match is_x {
            true => (
                self.vault_x.to_account_info(),
                self.user_x.to_account_info(),
            ),
            false => (
                self.vault_y.to_account_info(),
                self.user_y.to_account_info(),
            ),
        };

        let seed = self.config.seed.to_le_bytes();

        let signer_seeds: &[&[&[u8]]] = &[&[
            b"amm_config",
            seed.as_ref(),
            &[self.config.bump],
        ]];

        transfer(
            CpiContext::new_with_signer(
                self.token_program.key(),
                Transfer {
                    from,
                    to,
                    authority: self.config.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )
    }
}