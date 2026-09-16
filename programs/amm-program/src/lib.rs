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

    pub fn initialize(ctx: Context<Initialize>,seed: u64, fee:u64) -> Result<()> {
        crate::instructions::initialize::handle_initialize(ctx, seed, fee)
    }
    
    pub fn deposit(ctx: Context<Deposit>,amount: u64,max_x: u64,max_y: u64,) -> Result<()> {
    crate::instructions::deposit::handle_deposit(ctx,amount,max_x,max_y)
    }
    
}
