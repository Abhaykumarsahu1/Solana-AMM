pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("3LG67VXRN5xibaJdXNfzW1RBqTdQr6nF2NMGtuHiYexF");

#[program]
pub mod amm_program {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>,seed: u64, fee:u16, protocol_fee: u16) -> Result<()> {
        crate::instructions::initialize::handle_initialize(ctx, seed, fee, protocol_fee)
    }

    pub fn deposit(ctx: Context<Deposit>,amount: u64,max_x: u64,max_y: u64,) -> Result<()> {
    crate::instructions::deposit::handle_deposit(ctx,amount,max_x,max_y)
    }
    
    pub fn swap(ctx: Context<Swap>,is_x: bool, amount: u64, min: u64,) -> Result<()> {
    ctx.accounts.swap(is_x, amount, min)
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount: u64, min_x: u64, min_y: u64 ) -> Result<()> {
    ctx.accounts.withdraw(amount, min_x, min_y)
    }

}
