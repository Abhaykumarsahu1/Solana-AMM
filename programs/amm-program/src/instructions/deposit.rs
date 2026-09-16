use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount},
};

use crate::state::AmmConfig;
use constant_product_curve::ConstantProduct;

use crate::{
    error::AmmError,
    state::AmmConfig,
};

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

// pub fn handle_deposit(ctx : Context<Deposit>,amount: u64, max_x:u64, max_y:u64)->Result<()>{

//     require!(!ctx.accounts.config.locked, AmmError::PoolLocked); //Pool shouldn't be locked
//     require!(amount != 0, AmmError::InvalidAmount);

//     /* Initial pool:
// mint_lp.supply = 0

// After someone receives 100 LP:
// mint_lp.supply = 100

// After another 50 LP are minted:
// mint_lp.supply = 150*/

//     let (x_amount, y_amount) = if ctx.accounts.lp_mint.supply == 0 {
//         let x_amount = max_x;
//         let y_amount  = max_y;

//         let lp_amount = ((x_amount as u128) * (y_amount as u128)).integer_sqrt() as u64;

//         require!(lp_amount >= amount, AmmError::InvalidAmount);
//     }
//     else{

//     };
//     Ok(())
// }


pub fn handle_deposit(
    ctx: Context<Deposit>,
    amount: u64,
    max_x: u64,
    max_y: u64,
) -> Result<()> {
    require!(!ctx.accounts.config.locked, AmmError::PoolLocked);
    require!(amount != 0, AmmError::InvalidAmount);

    let (x_amount, y_amount) =
        if ctx.accounts.mint_lp.supply == 0
            && ctx.accounts.vault_x.amount == 0
            && ctx.accounts.vault_y.amount == 0
        {
            (max_x, max_y)
        } else {
            let amounts = ConstantProduct::xy_deposit_amounts_from_l(
                ctx.accounts.vault_x.amount,
                ctx.accounts.vault_y.amount,
                ctx.accounts.mint_lp.supply,
                amount,
                6,
            )
            .unwrap();

            require!(
                amounts.x <= max_x && amounts.y <= max_y,
                AmmError::SlippageExceeded
            );

            (amounts.x, amounts.y)
        };

    transfer_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.user_x,
        &ctx.accounts.vault_x,
        &ctx.accounts.user,
        x_amount,
    )?;

    transfer_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.user_y,
        &ctx.accounts.vault_y,
        &ctx.accounts.user,
        y_amount,
    )?;

    mint_lp_tokens(
        &ctx.accounts.token_program,
        &ctx.accounts.mint_lp,
        &ctx.accounts.user_lp,
        &ctx.accounts.config,
        amount,
    )?;

    Ok(())
}

fn transfer_tokens<'info>(
    token_program: &Program<'info, Token>,
    from: &Account<'info, TokenAccount>,
    to: &Account<'info, TokenAccount>,
    authority: &Signer<'info>,
    amount: u64,
) -> Result<()> {
    let cpi_accounts = Transfer {
        from: from.to_account_info(),
        to: to.to_account_info(),
        authority: authority.to_account_info(),
    };

    let cpi_ctx = CpiContext::new(
        token_program.to_account_info(),
        cpi_accounts,
    );

    transfer(cpi_ctx, amount)
}

fn mint_lp_tokens<'info>(
    token_program: &Program<'info, Token>,
    mint_lp: &Account<'info, Mint>,
    user_lp: &Account<'info, TokenAccount>,
    config: &Account<'info, AmmConfig>,
    amount: u64,
) -> Result<()> {
    let cpi_accounts = MintTo {
        mint: mint_lp.to_account_info(),
        to: user_lp.to_account_info(),
        authority: config.to_account_info(),
    };

    let seed = config.seed.to_le_bytes();

    let signer_seeds: &[&[&[u8]]] = &[&[
        b"config",
        seed.as_ref(),
        &[config.bump],
    ]];

    let cpi_ctx = CpiContext::new_with_signer(
        token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );

    mint_to(cpi_ctx, amount)
}