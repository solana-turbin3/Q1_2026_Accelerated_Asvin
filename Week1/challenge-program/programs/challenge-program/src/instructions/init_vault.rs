use anchor_lang::prelude::*;
use anchor_spl::{
    token_interface::{
        Mint,
        TokenAccount,
        mint_to,
        MintTo,
    },
    token_2022::Token2022,
};

use crate::states::VaultState;

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,
    
    /// The mint account for the token 
    #[account(
        init,
        payer = authority,
        mint::decimals = 9,
        mint::authority = vault_state,
        mint::token_program = token_program,
        seeds = [b"mint"],
        bump,
    )]
    pub mint: InterfaceAccount<'info, Mint>,
    
    /// The vault's token account that holds the tokens
    #[account(
        init,
        payer = authority,
        token::mint = mint,
        token::authority = vault_state,
        token::token_program = token_program,
        seeds = [b"vault", mint.key().as_ref()],
        bump,
    )]
    pub vault: InterfaceAccount<'info, TokenAccount>,
    
    /// State account storing vault metadata
    #[account(
        init,
        payer = authority,
        space = 8 + VaultState::INIT_SPACE,
        seeds = [b"vault_state"],
        bump,
    )]
    pub vault_state: Account<'info, VaultState>,
    
    pub token_program: Program<'info, Token2022>,
    pub system_program: Program<'info, System>,
}

impl<'info> InitializeVault<'info> {
    pub fn initialize_vault(
        &mut self,
        bumps: &InitializeVaultBumps,
        initial_supply: u64,
    ) -> Result<()> {
        // Initialize the vault state
        self.vault_state.mint = self.mint.key();
        self.vault_state.authority = self.authority.key();
        self.vault_state.total_deposited = initial_supply;
        self.vault_state.vault_bump = bumps.vault;
        self.vault_state.state_bump = bumps.vault_state;
        
        msg!("Vault initialized with mint: {}", self.mint.key());
        msg!("Vault token account: {}", self.vault.key());
        msg!("Initial supply: {}", initial_supply);
        
        // Mint initial supply to the vault if requested
        if initial_supply > 0 {
            self.mint_to_vault(bumps.vault_state, initial_supply)?;
        }
        
        Ok(())
    }
    
    /// Mint tokens to the vault
    fn mint_to_vault(&self, state_bump: u8, amount: u64) -> Result<()> {
        let seeds = &[
            b"vault_state".as_ref(),
            &[state_bump],
        ];
        let signer_seeds = &[&seeds[..]];
        
        let cpi_accounts = MintTo {
            mint: self.mint.to_account_info(),
            to: self.vault.to_account_info(),
            authority: self.vault_state.to_account_info(),
        };
        
        let cpi_context = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            cpi_accounts,
            signer_seeds,
        );
        
        mint_to(cpi_context, amount)?;
        
        msg!("Minted {} tokens to vault", amount);
        
        Ok(())
    }
}