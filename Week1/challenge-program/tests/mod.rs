#[cfg(test)]
mod tests {
    use {
        anchor_lang::{
            prelude::*, 
            AccountDeserialize, 
            InstructionData, 
            ToAccountMetas
        },
        litesvm::LiteSVM,
        litesvm_token::{
            spl_token_2022::ID as TOKEN_2022_PROGRAM_ID,
            CreateMint, 
            CreateAssociatedTokenAccount,
            MintTo,
        },
        solana_account::Account,
        solana_instruction::Instruction,
        solana_keypair::Keypair,
        solana_message::Message,
        solana_native_token::LAMPORTS_PER_SOL,
        solana_pubkey::Pubkey,
        solana_sdk_ids::system_program::ID as SYSTEM_PROGRAM_ID,
        solana_signer::Signer,
        solana_transaction::Transaction,
        std::path::PathBuf,
    };

    static PROGRAM_ID: Pubkey = crate::ID;
   // Setup function to initialize LiteSVM and create a payer keypair
    fn setup() -> (LiteSVM, Keypair, Keypair, Keypair) {
        let mut svm = LiteSVM::new();
        
        // Create keypairs
        let authority = Keypair::new();
        let user1 = Keypair::new();
        let user2 = Keypair::new();
        
        // Airdrop SOL
        svm.airdrop(&authority.pubkey(), 100 * LAMPORTS_PER_SOL).unwrap();
        svm.airdrop(&user1.pubkey(), 100 * LAMPORTS_PER_SOL).unwrap();
        svm.airdrop(&user2.pubkey(), 100 * LAMPORTS_PER_SOL).unwrap();
        
        // Load program SO file
        let so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/deploy/challenge_program.so");
        
        let program_data = std::fs::read(so_path).expect("Failed to read program SO file");
        svm.add_program(PROGRAM_ID, &program_data);
        
        (svm, authority, user1, user2)
    }

    /// Helper to derive PDAs
    fn derive_pdas(
        program_id: &Pubkey,
        user1: &Pubkey,
        user2: &Pubkey,
    ) -> (Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey, Pubkey) {
        let (mint_pda, _) = Pubkey::find_program_address(&[b"mint"], program_id);
        
        let (metadata_pda, _) = Pubkey::find_program_address(&[b"metadata"], program_id);
        
        let (vault_state_pda, _) = Pubkey::find_program_address(&[b"vault_state"], program_id);
        
        let (vault_pda, _) = Pubkey::find_program_address(
            &[b"vault", mint_pda.as_ref()],
            program_id,
        );
        
        let (extra_metas_pda, _) = Pubkey::find_program_address(
            &[b"extra-account-metas", mint_pda.as_ref()],
            program_id,
        );
        
        let (user1_whitelist_pda, _) = Pubkey::find_program_address(
            &[b"whitelist", user1.as_ref()],
            program_id,
        );
        
        let (user2_whitelist_pda, _) = Pubkey::find_program_address(
            &[b"whitelist", user2.as_ref()],
            program_id,
        );
        
        (
            mint_pda,
            metadata_pda,
            vault_state_pda,
            vault_pda,
            extra_metas_pda,
            user1_whitelist_pda,
            user2_whitelist_pda,
        )
    }

    #[test]
    fn test_01_initialize_vault() {
        msg!("\n\nTest 01: Initialize Vault with Token2022 Extensions\n");
        
        let (mut svm, authority, user1, user2) = setup();
        let (mint_pda, metadata_pda, vault_state_pda, vault_pda, _, _, _) = 
            derive_pdas(&PROGRAM_ID, &user1.pubkey(), &user2.pubkey());
        
        let initial_supply = 1_000_000_000u64; // 1000 tokens with 9 decimals
        
        // Create initialize_vault instruction
        let ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::InitializeVault {
                authority: authority.pubkey(),
                mint: mint_pda,
                metadata: metadata_pda,
                vault: vault_pda,
                vault_state: vault_state_pda,
                token_program: TOKEN_2022_PROGRAM_ID,
                system_program: SYSTEM_PROGRAM_ID,
            }
            .to_account_metas(None),
            data: crate::instruction::InitializeVault { initial_supply }.data(),
        };
        
        // Send transaction
        let message = Message::new(&[ix], Some(&authority.pubkey()));
        let blockhash = svm.latest_blockhash();
        let tx = Transaction::new(&[&authority], message, blockhash);
        
        let result = svm.send_transaction(tx).unwrap();
        msg!("Vault initialized successfully!");
        msg!("   Signature: {}", result.signature);
        msg!("   CUs consumed: {}", result.compute_units_consumed);
        
        // Verify vault state
        let vault_state_account = svm.get_account(&vault_state_pda).unwrap();
        let vault_state = crate::state::VaultState::try_deserialize(
            &mut vault_state_account.data.as_ref()
        ).unwrap();
        
        assert_eq!(vault_state.mint, mint_pda);
        assert_eq!(vault_state.authority, authority.pubkey());
        assert_eq!(vault_state.total_deposited, initial_supply);
        
        msg!("Vault state verified successfully:");
        msg!("   Mint: {}", vault_state.mint);
        msg!("   Authority: {}", vault_state.authority);
        msg!("   Total Deposited: {}", vault_state.total_deposited);
        
        // Verify mint account has extensions
        let mint_account = svm.get_account(&mint_pda).unwrap();
        assert!(mint_account.data.len() > 82, "Mint should have extensions");
        msg!(" Mint has extensions (size: {} bytes)", mint_account.data.len());
    }


}