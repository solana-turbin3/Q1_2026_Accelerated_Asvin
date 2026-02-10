use anchor_lang::{
    prelude::*, 
};

use crate::errors::WhitelistTransferHookError;
use crate::states::{whitelist::Whitelist, VaultState};

#[derive(Accounts)]
pub struct WhitelistOperations<'info> {
    #[account(
        mut,
        address = vault_state.authority @ WhitelistTransferHookError::Unauthorized
    )]
    pub admin: Signer<'info>,
    #[account(
        mut,
        seeds = [b"whitelist", user.key().as_ref()],
        bump,
    )]
    pub whitelist: Account<'info, Whitelist>,
    #[account(
        seeds = [b"vault_state"],
        bump = vault_state.state_bump,
    )]
    pub vault_state: Account<'info, VaultState>,
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