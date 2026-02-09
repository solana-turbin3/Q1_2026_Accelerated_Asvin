use anchor_lang::prelude::*;
use anchor_spl::{
    token_interface::{
        Mint,
        TokenAccount,
        transfer_checked,
        TransferChecked,
    },
    token_2022::Token2022,
};

use crate::states::{VaultState, Whitelist};

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
        
        let cpi_accounts = TransferChecked {
            from: self.user_token_account.to_account_info(),
            mint: self.mint.to_account_info(),
            to: self.vault.to_account_info(),
            authority: self.user.to_account_info(),
        };
        
        let cpi_context = CpiContext::new(
            self.token_program.to_account_info(),
            cpi_accounts,
        );
        
        transfer_checked(cpi_context, amount, self.mint.decimals)?;
        
        self.vault_state.total_deposited = self.vault_state
            .total_deposited
            .checked_add(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        
        msg!("Deposit successful! New vault total: {}", self.vault_state.total_deposited);
        
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

impl<'info> Withdraw<'info> {
    pub fn withdraw(&mut self, amount: u64) -> Result<()> {
        msg!("User {} withdrawing {} tokens", self.user.key(), amount);
        
        let seeds = &[
            b"vault_state".as_ref(),
            &[self.vault_state.state_bump],
        ];
        let signer_seeds = &[&seeds[..]];

        let cpi_accounts = TransferChecked {
            from: self.vault.to_account_info(),
            mint: self.mint.to_account_info(),
            to: self.user_token_account.to_account_info(),
            authority: self.vault_state.to_account_info(),
        };
        
        let cpi_context = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        
        transfer_checked(cpi_context, amount, self.mint.decimals)?;
        
        self.vault_state.total_deposited = self.vault_state
            .total_deposited
            .checked_sub(amount)
            .ok_or(ProgramError::ArithmeticOverflow)?;
        
        msg!("Withdrawal successful! New vault total: {}", self.vault_state.total_deposited);
        
        Ok(())
    }
}