use anchor_lang::prelude::*;

use crate::states::Whitelist;

#[derive(Accounts)]
#[instruction(user: Pubkey)]
pub struct InitializeWhitelist<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,
    #[account(
        init,
        payer = admin,
        space = Whitelist::INIT_SPACE + Whitelist::DISCRIMINATOR.len(), // 8 bytes for discriminator,32 bytes for address, 4 bytes for vector length, 1 byte for bump
        seeds = [b"whitelist", user.key().as_ref()],
        bump
    )]
    pub whitelist: Account<'info, Whitelist>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializeWhitelist<'info> {
    pub fn initialize_whitelist(
        &mut self, 
        bumps: InitializeWhitelistBumps,
        user: Pubkey,
        amount: u64
    ) -> Result<()> {
        // Initialize the whitelist with an empty address vector
        self.whitelist.address = user.key();
        self.whitelist.bump = bumps.whitelist;
        self.whitelist.is_whitelisted = false;
        self.whitelist.amount = amount;
        Ok(())
    }
}