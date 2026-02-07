use anchor_lang::{
    prelude::*, 
    system_program
};

use crate::state::whitelist::Whitelist;
use crate::errors::WhitelistTransferHookError;


#[derive(Accounts)]
pub struct WhitelistOperations<'info> {
    #[account(
        mut,
        //address = 
    )]
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [b"whitelist", user.key().as_ref()],
        bump,
    )]
    pub whitelist: Account<'info, Whitelist>,
    #[account(mut)]
    pub user: SystemAccount<'info>,
    pub system_program: Program<'info, System>,
}

impl<'info> WhitelistOperations<'info> {
    pub fn add_to_whitelist(&mut self, user: Pubkey) -> Result<()> {
        if self.whitelist.is_whitelisted {
            Err(WhitelistTransferHookError::AlreadyWhitelisted)?
        }
        self.whitelist.is_whitelisted = true;
        Ok(())
    }

    pub fn remove_from_whitelist(&mut self, address: Pubkey) -> Result<()> {
        if !self.whitelist.is_whitelisted {
            Err(WhitelistTransferHookError::NotWhitelisted)?
        }
        self.whitelist.is_whitelisted = false;
        Ok(())
    }

}