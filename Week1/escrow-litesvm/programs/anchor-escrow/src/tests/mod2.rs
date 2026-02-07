#[cfg(test)]
mod time_lock_tests {

    use {
        anchor_lang::{
            prelude::msg, 
            solana_program::{program_pack::Pack, clock::Clock}, 
            AccountDeserialize, 
            InstructionData, 
            ToAccountMetas
        }, anchor_spl::{
            associated_token::{
                self, 
                spl_associated_token_account
            }, 
            token::spl_token
        }, 
        litesvm::LiteSVM, 
        litesvm_token::{
            spl_token::ID as TOKEN_PROGRAM_ID, 
            CreateAssociatedTokenAccount, 
            CreateMint, MintTo
        }, 
        solana_rpc_client::rpc_client::RpcClient,
        solana_account::Account,
        solana_instruction::Instruction, 
        solana_keypair::Keypair, 
        solana_message::Message, 
        solana_native_token::LAMPORTS_PER_SOL, 
        solana_pubkey::Pubkey, 
        solana_sdk_ids::system_program::ID as SYSTEM_PROGRAM_ID, 
        solana_signer::Signer, 
        solana_transaction::Transaction, 
        solana_address::Address, 
        std::{
            path::PathBuf, 
            str::FromStr
        }
    };

    static PROGRAM_ID: Pubkey = crate::ID;
    const FIVE_DAYS_IN_SECONDS: i64 = 5 * 24 * 60 * 60; // 432,000 seconds

    // Setup function to initialize LiteSVM and create a payer keypair
    fn setup() -> (LiteSVM, Keypair) {
        let mut program = LiteSVM::new();
        let payer = Keypair::new();
    
        program
            .airdrop(&payer.pubkey(), 10 * LAMPORTS_PER_SOL)
            .expect("Failed to airdrop SOL to payer");
    
        let so_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/deploy/anchor_escrow.so");
    
        let program_data = std::fs::read(so_path).expect("Failed to read program SO file");
        program.add_program(PROGRAM_ID, &program_data);

        let rpc_client = RpcClient::new("https://api.devnet.solana.com");
        let account_address = Address::from_str("DRYvf71cbF2s5wgaJQvAGkghMkRcp5arvsK2w97vXhi2").unwrap();
        let fetched_account = rpc_client
            .get_account(&account_address)
            .expect("Failed to fetch account from devnet");

        program.set_account(payer.pubkey(), Account { 
            lamports: fetched_account.lamports, 
            data: fetched_account.data, 
            owner: Pubkey::from(fetched_account.owner.to_bytes()), 
            executable: fetched_account.executable, 
            rent_epoch: fetched_account.rent_epoch 
        }).unwrap();

        msg!("Lamports of fetched account: {}", fetched_account.lamports);
        
        let initial_clock = Clock {
            slot: 1000,
            epoch_start_timestamp: 1_000_000_000,
            epoch: 0,
            leader_schedule_epoch: 0,
            unix_timestamp: 1_700_000_000,
        };
        program.set_sysvar(&initial_clock);
        
        (program, payer)
    }

    #[test]
    fn test_take_before_5_days_fails() {
        msg!(" TEST: Take BEFORE 5-day lock (should FAIL)");

        let (mut program, payer) = setup();
        let maker = payer.pubkey();
        let taker = Keypair::new();
        
        program.airdrop(&taker.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        
        let mint_a = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();

        let mint_b = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();

        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker).send().unwrap();

        let taker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &taker, &mint_a)
            .owner(&taker.pubkey()).send().unwrap();

        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &taker, &mint_b)
            .owner(&taker.pubkey()).send().unwrap();

        let maker_ata_b = associated_token::get_associated_token_address(&maker, &mint_b);

        let escrow = Pubkey::find_program_address(
            &[b"escrow", maker.as_ref(), &123u64.to_le_bytes()],
            &PROGRAM_ID
        ).0;

        let vault = associated_token::get_associated_token_address(&escrow, &mint_a);

        let asspciated_token_program = spl_associated_token_account::ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1000000000).send().unwrap();
        MintTo::new(&mut program, &payer, &mint_b, &taker_ata_b, 1000000000).send().unwrap();

        // Execute Make instruction
        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Make {
                maker,
                mint_a,
                mint_b,
                maker_ata_a,
                escrow,
                vault,
                associated_token_program: asspciated_token_program,
                token_program,
                system_program,
            }.to_account_metas(None),
            data: crate::instruction::Make {deposit: 10, seed: 123u64, receive: 10 }.data(),
        };

        let message = Message::new(&[make_ix], Some(&payer.pubkey()));
        let transaction = Transaction::new(&[&payer], message, program.latest_blockhash());
        program.send_transaction(transaction).unwrap();

        msg!(" Make transaction successful");

        // Verify escrow timestamp
        let escrow_account = program.get_account(&escrow).unwrap();
        let escrow_data = crate::state::Escrow::try_deserialize(&mut escrow_account.data.as_ref()).unwrap();
        msg!(" Escrow start_time: {}", escrow_data.start_time);
        assert!(escrow_data.start_time > 0);

        // Try Take IMMEDIATELY (should fail)
        msg!("\n→ Attempting Take immediately (before 5 days)...");

        let take_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Take {
                taker: taker.pubkey(),
                maker,
                mint_a,
                mint_b,
                taker_ata_a,
                taker_ata_b,
                maker_ata_b,
                escrow,
                vault,
                associated_token_program: asspciated_token_program,
                token_program,
                system_program,
            }.to_account_metas(None),
            data: crate::instruction::Take {}.data(),
        };

        let message = Message::new(&[take_ix], Some(&taker.pubkey()));
        let transaction = Transaction::new(&[&taker], message, program.latest_blockhash());
        let result = program.send_transaction(transaction);

        assert!(result.is_err(), "Take should fail before 5-day time lock");
        msg!(" Take correctly FAILED (time lock active)");
        msg!(" Error: {:?}\n", result.err());
        msg!(" TEST PASSED: Time lock enforced correctly!\n");
    }

    #[test]
    fn test_take_after_5_days_succeeds() {
        msg!(" TEST: Take AFTER 5-day lock (should SUCCEED)");

        let (mut program, payer) = setup();
        let maker = payer.pubkey();
        let taker = Keypair::new();
        
        program.airdrop(&taker.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        
        let mint_a = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();

        let mint_b = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();

        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker).send().unwrap();

        let taker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &taker, &mint_a)
            .owner(&taker.pubkey()).send().unwrap();

        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &taker, &mint_b)
            .owner(&taker.pubkey()).send().unwrap();

        let maker_ata_b = associated_token::get_associated_token_address(&maker, &mint_b);

        let escrow = Pubkey::find_program_address(
            &[b"escrow", maker.as_ref(), &456u64.to_le_bytes()],
            &PROGRAM_ID
        ).0;

        let vault = associated_token::get_associated_token_address(&escrow, &mint_a);

        let asspciated_token_program = spl_associated_token_account::ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1000000000).send().unwrap();
        MintTo::new(&mut program, &payer, &mint_b, &taker_ata_b, 1000000000).send().unwrap();

        // Execute Make
        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Make {
                maker,
                mint_a,
                mint_b,
                maker_ata_a,
                escrow,
                vault,
                associated_token_program: asspciated_token_program,
                token_program,
                system_program,
            }.to_account_metas(None),
            data: crate::instruction::Make {deposit: 10, seed: 456u64, receive: 10 }.data(),
        };

        let message = Message::new(&[make_ix], Some(&payer.pubkey()));
        let transaction = Transaction::new(&[&payer], message, program.latest_blockhash());
        program.send_transaction(transaction).unwrap();

        msg!(" Make transaction successful");

        // Get escrow start time
        let escrow_account = program.get_account(&escrow).unwrap();
        let escrow_data = crate::state::Escrow::try_deserialize(&mut escrow_account.data.as_ref()).unwrap();
        let start_time = escrow_data.start_time;
        msg!(" Escrow start_time: {}", start_time);

    
        msg!("\n→ Advancing time by 5 days ({} seconds)...", FIVE_DAYS_IN_SECONDS);
        let new_time = start_time + FIVE_DAYS_IN_SECONDS;
        
        let new_clock = Clock {
            slot: 100000,
            epoch_start_timestamp: new_time - 100000,
            epoch: 0,
            leader_schedule_epoch: 0,
            unix_timestamp: new_time,
        };
        
        program.set_sysvar(&new_clock);
        msg!(" Clock advanced to: {}", new_time);
        msg!("Time elapsed: {} seconds ({}+ days)\n", new_time - start_time, FIVE_DAYS_IN_SECONDS / 86400);

        // Execute Take (should succeed now)
        msg!("→ Executing Take instruction after time lock...");
        
        let take_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Take {
                taker: taker.pubkey(),
                maker,
                mint_a,
                mint_b,
                taker_ata_a,
                taker_ata_b,
                maker_ata_b,
                escrow,
                vault,
                associated_token_program: asspciated_token_program,
                token_program,
                system_program,
            }.to_account_metas(None),
            data: crate::instruction::Take {}.data(),
        };

        let message = Message::new(&[take_ix], Some(&taker.pubkey()));
        let transaction = Transaction::new(&[&taker], message, program.latest_blockhash());
        let result = program.send_transaction(transaction);

        assert!(result.is_ok(), "Take should succeed after 5 days: {:?}", result.err());
        msg!(" Take transaction SUCCESSFUL!");

        // Verify final state
        msg!("\n→ Verifying final state...");

        // Vault closed
        let vault_account = program.get_account(&vault);
        if let Some(vault_acc) = vault_account {
            assert_eq!(vault_acc.lamports, 0);
        }
        msg!(" Vault closed");

        // Escrow closed
        let escrow_account = program.get_account(&escrow);
        if let Some(escrow_acc) = escrow_account {
            assert_eq!(escrow_acc.lamports, 0);
        }
        msg!(" Escrow closed");

        // Taker received tokens
        let taker_ata_a_account = program.get_account(&taker_ata_a).unwrap();
        let taker_ata_a_data = spl_token::state::Account::unpack(&taker_ata_a_account.data).unwrap();
        assert_eq!(taker_ata_a_data.amount, 10);
        msg!(" Taker received 10 tokens of Mint A");

        // Maker received tokens
        let maker_ata_b_account = program.get_account(&maker_ata_b).unwrap();
        let maker_ata_b_data = spl_token::state::Account::unpack(&maker_ata_b_account.data).unwrap();
        assert_eq!(maker_ata_b_data.amount, 10);
        msg!(" Maker received 10 tokens of Mint B");

        msg!("\n TEST PASSED: Take succeeded after 5-day time lock!\n");
    }

    #[test]
    fn test_take_exactly_at_5_days() {
        msg!(" TEST: Take EXACTLY at 5 days (edge case)");

        let (mut program, payer) = setup();
        let maker = payer.pubkey();
        let taker = Keypair::new();
        
        program.airdrop(&taker.pubkey(), 10 * LAMPORTS_PER_SOL).unwrap();
        
        let mint_a = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();

        let mint_b = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();

        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker).send().unwrap();

        let taker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &taker, &mint_a)
            .owner(&taker.pubkey()).send().unwrap();

        let taker_ata_b = CreateAssociatedTokenAccount::new(&mut program, &taker, &mint_b)
            .owner(&taker.pubkey()).send().unwrap();

        let maker_ata_b = associated_token::get_associated_token_address(&maker, &mint_b);

        let escrow = Pubkey::find_program_address(
            &[b"escrow", maker.as_ref(), &789u64.to_le_bytes()],
            &PROGRAM_ID
        ).0;

        let vault = associated_token::get_associated_token_address(&escrow, &mint_a);

        let asspciated_token_program = spl_associated_token_account::ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1000000000).send().unwrap();
        MintTo::new(&mut program, &payer, &mint_b, &taker_ata_b, 1000000000).send().unwrap();

        // Execute Make
        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Make {
                maker,
                mint_a,
                mint_b,
                maker_ata_a,
                escrow,
                vault,
                associated_token_program: asspciated_token_program,
                token_program,
                system_program,
            }.to_account_metas(None),
            data: crate::instruction::Make {deposit: 25, seed: 789u64, receive: 25 }.data(),
        };

        let message = Message::new(&[make_ix], Some(&payer.pubkey()));
        let transaction = Transaction::new(&[&payer], message, program.latest_blockhash());
        program.send_transaction(transaction).unwrap();

        // Get start time
        let escrow_account = program.get_account(&escrow).unwrap();
        let escrow_data = crate::state::Escrow::try_deserialize(&mut escrow_account.data.as_ref()).unwrap();
        let start_time = escrow_data.start_time;
        msg!(" Escrow start_time: {}", start_time);

        // Set time to EXACTLY 5 days (432000 seconds)
        msg!("\n→ Setting time to EXACTLY 5 days (edge case test)...");
        let exact_time = start_time + FIVE_DAYS_IN_SECONDS;
        
        let new_clock = Clock {
            slot: 100000,
            epoch_start_timestamp: exact_time - 100000,
            epoch: 0,
            leader_schedule_epoch: 0,
            unix_timestamp: exact_time,
        };
        
        program.set_sysvar(&new_clock);
        msg!(" Clock set to exactly: {} (elapsed: {} seconds)", exact_time, FIVE_DAYS_IN_SECONDS);

        // Execute Take
        let take_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Take {
                taker: taker.pubkey(),
                maker,
                mint_a,
                mint_b,
                taker_ata_a,
                taker_ata_b,
                maker_ata_b,
                escrow,
                vault,
                associated_token_program: asspciated_token_program,
                token_program,
                system_program,
            }.to_account_metas(None),
            data: crate::instruction::Take {}.data(),
        };

        let message = Message::new(&[take_ix], Some(&taker.pubkey()));
        let transaction = Transaction::new(&[&taker], message, program.latest_blockhash());
        let result = program.send_transaction(transaction);

        assert!(result.is_ok(), "Take should succeed at exactly 5 days");
        msg!(" Take succeeded at EXACTLY 5 days");
        msg!("\n TEST PASSED: Edge case handled correctly!\n");
    }

    #[test]
    fn test_refund_not_affected_by_timelock() {
        msg!(" TEST: Refund NOT affected by time lock");

        let (mut program, payer) = setup();
        let maker = payer.pubkey();
        
        let mint_a = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();

        let mint_b = CreateMint::new(&mut program, &payer)
            .decimals(6)
            .authority(&maker)
            .send()
            .unwrap();

        let maker_ata_a = CreateAssociatedTokenAccount::new(&mut program, &payer, &mint_a)
            .owner(&maker).send().unwrap();

        let escrow = Pubkey::find_program_address(
            &[b"escrow", maker.as_ref(), &999u64.to_le_bytes()],
            &PROGRAM_ID
        ).0;

        let vault = associated_token::get_associated_token_address(&escrow, &mint_a);

        let asspciated_token_program = spl_associated_token_account::ID;
        let token_program = TOKEN_PROGRAM_ID;
        let system_program = SYSTEM_PROGRAM_ID;

        MintTo::new(&mut program, &payer, &mint_a, &maker_ata_a, 1000000000).send().unwrap();

        // Execute Make
        let make_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Make {
                maker,
                mint_a,
                mint_b,
                maker_ata_a,
                escrow,
                vault,
                associated_token_program: asspciated_token_program,
                token_program,
                system_program,
            }.to_account_metas(None),
            data: crate::instruction::Make {deposit: 100, seed: 999u64, receive: 50 }.data(),
        };

        let message = Message::new(&[make_ix], Some(&payer.pubkey()));
        let transaction = Transaction::new(&[&payer], message, program.latest_blockhash());
        program.send_transaction(transaction).unwrap();
        msg!(" Make transaction successful");

        // Execute Refund IMMEDIATELY (should work)
        msg!("\n→ Executing Refund immediately (before 5 days)...");

        let refund_ix = Instruction {
            program_id: PROGRAM_ID,
            accounts: crate::accounts::Refund {
                maker,
                mint_a,
                maker_ata_a,
                escrow,
                vault,
                token_program,
                system_program,
            }.to_account_metas(None),
            data: crate::instruction::Refund {}.data(),
        };

        let message = Message::new(&[refund_ix], Some(&payer.pubkey()));
        let transaction = Transaction::new(&[&payer], message, program.latest_blockhash());
        let result = program.send_transaction(transaction);

        assert!(result.is_ok(), "Refund should work immediately");
        msg!(" Refund succeeded IMMEDIATELY");

        // Verify maker got tokens back
        let maker_ata_a_account = program.get_account(&maker_ata_a).unwrap();
        let maker_ata_a_data = spl_token::state::Account::unpack(&maker_ata_a_account.data).unwrap();
        assert_eq!(maker_ata_a_data.amount, 1000000000);
        msg!(" Maker received all 1000000000 tokens back");

        msg!("\n TEST PASSED: Refund works anytime (no time lock)!\n");
    }
}
