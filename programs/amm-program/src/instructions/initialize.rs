use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{Mint, Token, TokenAccount}
};
use crate::AmmConfig;
use crate::error::AmmError;
#[derive(Accounts)]
#[instruction(seed:u64)]
pub struct Initialize<'info>{

    #[account(mut)] //lamports deduction
    pub payer : Signer<'info>, //i am the one who will create the amm pool

    //these are the mint account will represent 2 tokens their metadata  
    pub mint_x: Account<'info, Mint>,
    pub mint_y: Account<'info, Mint>,

    /// CHECK: Treasury account is only used as the destination for protocol fees.
    pub treasury: UncheckedAccount<'info>,

    #[account(
        init,
        payer = payer,
        space = 8 + AmmConfig::INIT_SPACE,
        seeds = [b"amm_config", seed.to_le_bytes().as_ref()],
        bump
    )]
    pub config : Account<'info, AmmConfig>, //initializing the config/metadata for AMMpool

    #[account(
        init,
        payer=payer,
        associated_token::mint = mint_x,
        associated_token::authority = config, //amm pool will control the vault not me
    )]
    pub vault_x: Account<'info, TokenAccount>,
    
    #[account(
        init,
        payer=payer,
        associated_token::mint = mint_y,
        associated_token::authority = config,
    )]
    pub vault_y: Account<'info, TokenAccount>,

    #[account(
        init,
        payer=payer,
        seeds = [b"lp", config.key().as_ref()],
        bump,
        mint::decimals = 6,
        mint::authority = config,
    )]
    pub lp_mint : Account<'info, Mint>,

    

    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handle_initialize(
    ctx: Context<Initialize>,
    seed:u64,
    fee: u16,
    protocol_fee: u16,
)->Result<()>{
    require!(fee < 10_000, AmmError::InvalidAmount);
    let config = &mut ctx.accounts.config;//targeting the config accounts present in the struct intialize

    config.seed = seed;
    config.authority = Some(ctx.accounts.payer.key());
    config.mint_y = ctx.accounts.mint_y.key();
    config.mint_x = ctx.accounts.mint_x.key();
    config.lp_mint = ctx.accounts.lp_mint.key();
    config.treasury = ctx.accounts.treasury.key();
    config.fee = fee;
    config.protocol_fee = protocol_fee;
    config.locked = false;
    config.bump = ctx.bumps.config;

    Ok(())
}