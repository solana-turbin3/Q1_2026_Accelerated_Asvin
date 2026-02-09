use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct VaultState {
    pub mint: Pubkey,           // The token mint
    pub authority: Pubkey,       // Who can manage the vault
    pub total_deposited: u64,    // Total amount in vault
    pub vault_bump: u8,          // Bump for the vault account
    pub state_bump: u8,          // Bump for the state account
}