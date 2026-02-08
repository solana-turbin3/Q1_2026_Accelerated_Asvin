use anchor_lang::prelude::*;

declare_id!("G3Z36nvjRzn7F4bfn4mMovi1MJEUYhCXcv6xHotrAd9B");

#[program]
pub mod challenge_program {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
