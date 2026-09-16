use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{transfer, Mint, Token, TokenAccount, Transfer},
};
use constant_product_curve::{ConstantProduct, LiquidityPair};

use crate::{error::AmmError, state::AmmConfig};

#[derive(Accounts)]
pub struct Deposit<'info>{

    #[account(mut)] //coz depositor token will be deposited and also lp tokens will credit
    pub user: Signer<'info>,

    //these are the mint account will represent 2 tokens their metadata(authority, decimals etc..)  
    pub mint_x: Account<'info, Mint>,
    pub mint_y: Account<'info, Mint>,

    #[account(
        has_one = mint_x,
        has_one = mint_y,
        seeds = [b"amm_config", config.seed.to_le_bytes().as_ref()], //verifying that config pda has same in deposit as intialie or not
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
    pub vault_x : Account<'info, TokenAccount>,

    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority = config,
    )]
    pub vault_y : Account<'info, TokenAccount>,

    //the below user token ata coz to deposit the token firs u have to hold it
    #[account(
        mut,
        associated_token::mint = mint_x,
        associated_token::authority=user,
    )]
    pub user_x: Account<'info, TokenAccount>,
    
    #[account(
        mut,
        associated_token::mint = mint_y,
        associated_token::authority=user,
    )]
    pub user_y: Account<'info, TokenAccount>,

    //now we are creating ata for lp token coz when user will deposit he will get the LP tokens in his ata

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

impl<'info> Swap<'info> {
    pub fn swap(
        &mut self,
        is_x: bool,
        amount: u64,
        min: u64,
    ) -> Result<()> {
        require!(amount > 0, AmmError::InvalidAmount);
        require!(!self.config.locked, AmmError::PoolLocked);

        let mut curve = ConstantProduct::init(
            self.vault_x.amount,
            self.vault_y.amount,
            self.lp_mint.supply,
            self.config.fee,
            Some(6),
        )
        .map_err(|_| AmmError::InvalidAmount)?;

        let pair = match is_x {
            true => LiquidityPair::X,
            false => LiquidityPair::Y,
        };

        let swap_result = curve
            .swap(pair, amount, min)
            .map_err(|_| AmmError::SlippageExceeded)?;

        self.deposit_tokens(
            is_x,
            swap_result.deposit,
        )?;

        self.withdraw_tokens(
            is_x,
            swap_result.withdraw,
        )?;

        Ok(())
    }

    fn deposit_tokens(
        &mut self,
        is_x: bool,
        amount: u64,
    ) -> Result<()> {
        let (from, to) = match is_x {
            true => (
                self.user_x.to_account_info(),
                self.vault_x.to_account_info(),
            ),
            false => (
                self.user_y.to_account_info(),
                self.vault_y.to_account_info(),
            ),
        };

        transfer(
            CpiContext::new(
                self.token_program.to_account_info(),
                Transfer {
                    from,
                    to,
                    authority: self.user.to_account_info(),
                },
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
                self.vault_y.to_account_info(),
                self.user_y.to_account_info(),
            ),
            false => (
                self.vault_x.to_account_info(),
                self.user_x.to_account_info(),
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
                self.token_program.to_account_info(),
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