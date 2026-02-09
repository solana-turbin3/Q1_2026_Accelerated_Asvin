use anchor_lang::prelude::*;
use anchor_lang::solana_program::program::invoke;
use anchor_lang::system_program;
use anchor_spl::{
    token_2022::spl_token_2022,  // ADD THIS
    token_2022::spl_token_2022::extension::ExtensionType,
    token_2022::spl_token_2022::instruction::initialize_mint2,
    token_2022::Token2022,
    token_interface::{mint_to, MintTo, TokenAccount},
};

use crate::errors::WhitelistTransferHookError;
use crate::states::VaultState;

#[derive(Accounts)]
pub struct InitializeVault<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: Mint account is manually initialized with extensions in the instruction.
    /// We use UncheckedAccount because we create and initialize it manually
    /// to support multiple Token2022 extensions (TransferHook + MetadataPointer).
    #[account(
        mut,
        seeds = [b"mint"],
        bump,
    )]
    pub mint: UncheckedAccount<'info>,

    /// CHECK: Metadata account for the metadata pointer extension.
    /// This account will store token metadata and is validated by the metadata pointer extension.
    #[account(
        mut,
        seeds = [b"metadata"],
        bump,
    )]
    pub metadata: UncheckedAccount<'info>,

    // The vault's token account that holds the tokens
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

    // State account storing vault metadata
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
        // Calculate space needed for mint with both extensions
        let extension_types = vec![ExtensionType::TransferHook, ExtensionType::MetadataPointer];

        let space = ExtensionType::try_calculate_account_len::<spl_token_2022::state::Mint>(
            &extension_types,
        )
        .map_err(|_| error!(WhitelistTransferHookError::AccountNotEnoughKeys))?;

        // Create mint account
        let rent = Rent::get()?;
        let lamports = rent.minimum_balance(space);

        let mint_seeds = &[b"mint".as_ref(), &[bumps.mint]];
        let mint_signer = &[&mint_seeds[..]];

        system_program::create_account(
            CpiContext::new_with_signer(
                self.system_program.to_account_info(),
                system_program::CreateAccount {
                    from: self.authority.to_account_info(),
                    to: self.mint.to_account_info(),
                },
                mint_signer,
            ),
            lamports,
            space as u64,
            &self.token_program.key(),
        )?;

        // Initialize metadata pointer extension
        let metadata_pointer_init_ix =
            spl_token_2022::extension::metadata_pointer::instruction::initialize(
                &self.token_program.key(),
                &self.mint.key(),
                Some(self.vault_state.key()), // authority
                Some(self.metadata.key()),    // metadata address
            )
            .map_err(|_| error!(WhitelistTransferHookError::InvalidAccountData))?;

        invoke(&metadata_pointer_init_ix, &[self.mint.to_account_info()])?;

        // Initialize transfer hook extension
        let transfer_hook_init_ix =
            spl_token_2022::extension::transfer_hook::instruction::initialize(
                &self.token_program.key(),
                &self.mint.key(),
                Some(self.vault_state.key()), // authority
                Some(crate::ID),              // program_id that implements the hook
            )
            .map_err(|_| error!(WhitelistTransferHookError::InvalidAccountData))?;

        invoke(&transfer_hook_init_ix, &[self.mint.to_account_info()])?;

        // Initialize the mint
        let init_mint_ix = initialize_mint2(
            &self.token_program.key(),
            &self.mint.key(),
            &self.vault_state.key(), // mint authority
            None,                    // freeze authority
            9,                       // decimals
        )
        .map_err(|_| error!(WhitelistTransferHookError::InvalidAccountData))?;

        invoke(&init_mint_ix, &[self.mint.to_account_info()])?;

        // Initialize the vault state
        self.vault_state.mint = self.mint.key();
        self.vault_state.authority = self.authority.key();
        self.vault_state.total_deposited = initial_supply;
        self.vault_state.vault_bump = bumps.vault;
        self.vault_state.state_bump = bumps.vault_state;

        msg!("Vault initialized with mint: {}", self.mint.key());
        msg!("Metadata pointer: {}", self.metadata.key());
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
        let seeds = &[b"vault_state".as_ref(), &[state_bump]];
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
