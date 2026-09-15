use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct AmmConfig{
    pub seed : u64,
    pub authority: Option<PubKey>,
    pub mint_y: Pubkey,
    pub mint_x: Pubkey,
    pub lp_mint: Pubkey, //represent my ownership share of the pool
    pub locked : bool,
    pub fee: u64,
    pub bump: u8,
}
