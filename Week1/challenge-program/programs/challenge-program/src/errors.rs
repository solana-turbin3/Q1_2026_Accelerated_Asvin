use anchor_lang::prelude::*;

#[error_code]
pub enum WhitelistTransferHookError {
    #[msg("Address is already whitelisted")]
    AlreadyWhitelisted,
    
    #[msg("Address is not whitelisted")]
    NotWhitelisted,
    
    #[msg("Transfer amount exceeds limit")]
    ExceedsLimit,
    
    #[msg("Insufficient funds in vault")]  
    InsufficientFunds,
    
    #[msg("Arithmetic overflow")]  
    ArithmeticOverflow,

    #[msg("Unauthorized: Only admin can perform this action")] 
    Unauthorized,
}