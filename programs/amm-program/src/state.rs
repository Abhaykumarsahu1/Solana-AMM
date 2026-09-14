use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct AmmConfig{
    pub authority: Option<PubKey>,
    pub token_a: Pubkey,
    pub token_b: Pubkey,
    pub lp_mint: Pubkey,
    pub locked : bool,
    pub fee: u64,
    pub bump: u8,
}
