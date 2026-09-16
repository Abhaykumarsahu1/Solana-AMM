use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct AmmConfig {
    pub seed: u64,
    pub authority: Option<Pubkey>,
    pub mint_x: Pubkey,
    pub mint_y: Pubkey,
    pub lp_mint: Pubkey,
    pub treasury: Pubkey,
    pub locked: bool,
    pub fee: u16,
    pub protocol_fee: u16, // bps of `fee` skimmed to treasury on swap
    pub bump: u8,
}