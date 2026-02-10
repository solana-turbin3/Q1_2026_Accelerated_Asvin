use anchor_lang::prelude::*;
use anchor_spl::{
    token_interface::{
        Mint,
        TokenAccount,
        Approve,
        approve, 
    },
    token_2022::Token2022,
};

use crate::states::{VaultState, Whitelist};
use crate::errors::WhitelistTransferHookError;

//  DEPOSIT

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
   
    #[account(
        seeds = [b"mint"],
        bump,
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    
   
    #[account(
        mut,
        token::mint = mint,
        token::authority = user,
    )]
    pub user_token_account: InterfaceAccount<'info, TokenAccount>,
    
   
    #[account(
        mut,
        seeds = [b"vault", mint.key().as_ref()],
        bump = vault_state.vault_bump,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    
   
    #[account(
        mut,
        seeds = [b"vault_state"],
        bump = vault_state.state_bump,
    )]
    pub vault_state: Account<'info, VaultState>,
    
   
    #[account(
        seeds = [b"whitelist", user.key().as_ref()],
        bump = whitelist.bump,
    )]
    pub whitelist: Account<'info, Whitelist>,
    
    pub token_program: Program<'info, Token2022>,
}

impl<'info> Deposit<'info> {
    pub fn deposit(&mut self, amount: u64) -> Result<()> {
        msg!("User {} depositing {} tokens", self.user.key(), amount);
        
        // Only update the ledger balance
        // would CPI back into our transfer_hook, causing reentrancy
        
        self.whitelist.deposited_amount = self.whitelist
            .deposited_amount
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        
        msg!("Recorded deposit of {} tokens. New balance: {}", 
             amount, self.whitelist.deposited_amount);
        
        Ok(())
    }
}

//  WITHDRAW

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub user: Signer<'info>,
    
    #[account(
        seeds = [b"mint"],
        bump,
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    
    #[account(
        mut,
        token::mint = mint,
        token::authority = vault_state,  // Vault authority
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    
    #[account(
        mut,
        seeds = [b"vault_state"],
        bump = vault_state.state_bump,
    )]
    pub vault_state: Account<'info, VaultState>,
    
    #[account(
        mut,
        seeds = [b"whitelist", user.key().as_ref()],
        bump = whitelist.bump,
    )]
    pub whitelist: Account<'info, Whitelist>,
    
    pub token_program: Program<'info, Token2022>,
}

impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        msg!("User {} withdrawing {} tokens", self.user.key(), amount);
        
        // Check sufficient balance
        require!(
            self.whitelist.deposited_amount >= amount,
            WhitelistTransferHookError::InsufficientFunds
        );
        
        // Approve user as delegate on vault token account
        // Client MUST follow this with transfer_checked in same transaction
        // Since user is the delegate, transfer hook validates user's whitelist
        let seeds = &[
            b"vault_state".as_ref(),
            &[self.vault_state.state_bump],
        ];
        let signer_seeds = &[&seeds[..]];

        approve(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                Approve {
                    to: self.vault.to_account_info(),
                    delegate: self.user.to_account_info(),
                    authority: self.vault_state.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
        )?;
        
        // Update ledger balance
        self.whitelist.deposited_amount = self.whitelist
            .deposited_amount
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        
        msg!("Approved withdrawal of {} tokens. New balance: {}", 
             amount, self.whitelist.deposited_amount);
        
        Ok(())
    }
}