use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer, Mint, Token, TokenAccount, Transfer},
};
use constant_product_curve::{ConstantProduct, LiquidityPair};

use crate::{error::AmmError, state::AmmConfig};

#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    pub mint_x: Account<'info, Mint>,
    pub mint_y: Account<'info, Mint>,

    #[account(
        has_one = mint_x,
        has_one = mint_y,
        has_one = treasury,
        seeds = [b"amm_config", config.seed.to_le_bytes().as_ref()],
        bump = config.bump,
    )]
    pub config: Account<'info, AmmConfig>,

    #[account(seeds = [b"lp", config.key().as_ref()], bump)]
    pub lp_mint: Account<'info, Mint>,

    #[account(mut, associated_token::mint = mint_x, associated_token::authority = config)]
    pub vault_x: Account<'info, TokenAccount>,

    #[account(mut, associated_token::mint = mint_y, associated_token::authority = config)]
    pub vault_y: Account<'info, TokenAccount>,

    #[account(mut, associated_token::mint = mint_x, associated_token::authority = user)]
    pub user_x: Account<'info, TokenAccount>,

    #[account(mut, associated_token::mint = mint_y, associated_token::authority = user)]
    pub user_y: Account<'info, TokenAccount>,

    /// CHECK: fee-receiving wallet, checked against config.treasury via has_one above
    pub treasury: UncheckedAccount<'info>,

    #[account(init_if_needed, payer = user, associated_token::mint = mint_x, associated_token::authority = treasury)]
    pub treasury_x: Account<'info, TokenAccount>,

    #[account(init_if_needed, payer = user, associated_token::mint = mint_y, associated_token::authority = treasury)]
    pub treasury_y: Account<'info, TokenAccount>,

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

impl<'info> Swap<'info> {
    pub fn swap(&mut self, is_x: bool, amount: u64, min: u64) -> Result<()> {
        require!(amount > 0, AmmError::InvalidAmount);
        require!(!self.config.locked, AmmError::PoolLocked);

        // protocol cut skimmed off the input BEFORE it hits the curve
        let protocol_cut = (amount as u128)
            .checked_mul(self.config.protocol_fee as u128)
            .unwrap()
            .checked_div(10_000)
            .unwrap() as u64;
        let swap_amount = amount.checked_sub(protocol_cut).ok_or(AmmError::InvalidAmount)?;

        let mut curve = ConstantProduct::init(
            self.vault_x.amount,
            self.vault_y.amount,
            self.lp_mint.supply,
            self.config.fee,
            Some(6),
        )
        .map_err(|_| AmmError::InvalidAmount)?;

        let pair = if is_x { LiquidityPair::X } else { LiquidityPair::Y };
        let swap_result = curve.swap(pair, swap_amount, min).map_err(|_| AmmError::SlippageExceeded)?;

        if protocol_cut > 0 {
            self.pay_treasury(is_x, protocol_cut)?;
        }

        self.deposit_tokens(is_x, swap_result.deposit)?;
        self.withdraw_tokens(is_x, swap_result.withdraw)?;
        Ok(())
    }

    fn pay_treasury(&self, is_x: bool, amount: u64) -> Result<()> {
        let (from, to) = if is_x {
            (self.user_x.to_account_info(), self.treasury_x.to_account_info())
        } else {
            (self.user_y.to_account_info(), self.treasury_y.to_account_info())
        };
        transfer(CpiContext::new(self.token_program.key(), Transfer { from, to, authority: self.user.to_account_info() }), amount)
    }

    fn deposit_tokens(&self, is_x: bool, amount: u64) -> Result<()> {
        let (from, to) = if is_x {
            (self.user_x.to_account_info(), self.vault_x.to_account_info())
        } else {
            (self.user_y.to_account_info(), self.vault_y.to_account_info())
        };
        transfer(CpiContext::new(self.token_program.key(), Transfer { from, to, authority: self.user.to_account_info() }), amount)
    }

    fn withdraw_tokens(&self, is_x: bool, amount: u64) -> Result<()> {
        let (from, to) = if is_x {
            (self.vault_y.to_account_info(), self.user_y.to_account_info())
        } else {
            (self.vault_x.to_account_info(), self.user_x.to_account_info())
        };
        let seed = self.config.seed.to_le_bytes();
        let signer_seeds: &[&[&[u8]]] = &[&[b"amm_config", seed.as_ref(), &[self.config.bump]]];
        transfer(
            CpiContext::new_with_signer(self.token_program.key(), Transfer { from, to, authority: self.config.to_account_info() }, signer_seeds),
            amount,
        )
    }
}