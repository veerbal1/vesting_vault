use anchor_lang::prelude::*;

declare_id!("GvzD2zDi4AvLjRA5893csKVsHYezfv6D2B3SpAenSxoi");

#[program]
pub mod vesting_vault {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
