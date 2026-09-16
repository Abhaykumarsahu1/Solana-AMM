use anchor_lang::prelude::*;

#[error_code]
pub enum AmmError {
    #[msg("Amount cannot be zero")]
    InvalidAmount,
    #[msg("Pool is locked")]
    PoolLocked,
    #[msg("Slippage tolerance exceeded")]
    SlippageExceeded,
    #[msg("Insufficient LP token balance")]
    InsufficientFunds,
}