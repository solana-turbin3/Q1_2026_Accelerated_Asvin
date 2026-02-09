#![allow(unexpected_cfgs)]
#![allow(deprecated)]

mod instructions;
mod states;

use instructions::*;
use states::*;
use spl_discriminator::SplDiscriminate;
use spl_transfer_hook_interface::{
    instruction::{
        ExecuteInstruction, 
        InitializeExtraAccountMetaListInstruction
    },
};
use spl_tlv_account_resolution::state::ExtraAccountMetaList;
use anchor_lang::prelude::*;

declare_id!("G3Z36nvjRzn7F4bfn4mMovi1MJEUYhCXcv6xHotrAd9B");

#[program]
pub mod challenge_program {
    use super::*;

    pub fn initialize_whitelist(
        ctx: Context<InitializeWhitelist>,
        user: Pubkey
    ) -> Result<()> {
        ctx.accounts.initialize_whitelist(ctx.bumps, user)
    }

    pub fn add_to_whitelist(
        ctx: Context<WhitelistOperations>, 
        user: Pubkey
    ) -> Result<()> {
        ctx.accounts.add_to_whitelist(user)
    }

    pub fn remove_from_whitelist(
        ctx: Context<WhitelistOperations>, 
        user: Pubkey
    ) -> Result<()> {
        ctx.accounts.remove_from_whitelist(user)
    }


    pub fn initialize_transfer_hook(
        ctx: Context<InitializeExtraAccountMetaList>
    ) -> Result<()> {

        msg!("Initializing Transfer Hook...");

        // Get the extra account metas for the transfer hook
        let extra_account_metas = InitializeExtraAccountMetaList::extra_account_metas()?;

        msg!("Extra Account Metas: {:?}", extra_account_metas);
        msg!("Extra Account Metas Length: {}", extra_account_metas.len());

        // initialize ExtraAccountMetaList account with extra accounts
        ExtraAccountMetaList::init::<ExecuteInstruction>(
            &mut ctx.accounts.extra_account_meta_list.try_borrow_mut_data()?,
            &extra_account_metas
        ).unwrap();

        Ok(())
    }

    #[instruction(discriminator = ExecuteInstruction::SPL_DISCRIMINATOR_SLICE)]
    pub fn transfer_hook(ctx: Context<TransferHook>, amount: u64) -> Result<()> {
        // Call the transfer hook logic
        ctx.accounts.transfer_hook(amount)
    }
}

