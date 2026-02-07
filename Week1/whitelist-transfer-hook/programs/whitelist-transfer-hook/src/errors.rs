use anchor_lang::prelude::*;

#[error_code]
pub enum WhitelistTransferHookError {
    #[msg("User is not whitelisted")]
    NotWhitelisted,
    #[msg("User is already whitelisted")]
    AlreadyWhitelisted,
    #[msg("TransferHook: Not transferring")]
    NotTransferring,
}