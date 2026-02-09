use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Whitelist {
    pub address: Pubkey,
    pub amount: u64, 
    pub is_whitelisted: bool,
    pub bump: u8,
}