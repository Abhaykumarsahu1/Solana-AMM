use {
    amm_program::{self, AmmConfig},
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{
            instruction::Instruction,
            program_pack::Pack,
            system_instruction,
            system_program,
        },
        AccountDeserialize,
        InstructionData,
        ToAccountMetas,
    },
    anchor_spl::token::ID as TOKEN_PROGRAM_ID,
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
    spl_associated_token_account::get_associated_token_address,
    spl_token::{
        instruction as token_instruction,
        state::{Account as TokenAccount, Mint},
    },
};

fn send_tx(
    svm: &mut LiteSVM,
    instructions: Vec<Instruction>,
    signers: Vec<&Keypair>,
) {
    let payer = signers[0];

    let blockhash = svm.latest_blockhash();

    let message = Message::new_with_blockhash(
        &instructions,
        Some(&payer.pubkey()),
        &blockhash,
    );

    let versioned_message = VersionedMessage::Legacy(message);

    let tx = VersionedTransaction::try_new(
        versioned_message,
        &signers,
    )
    .unwrap();

    svm.send_transaction(tx).unwrap();
}

fn create_mint(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Keypair,
) {
    let rent = svm.minimum_balance_for_rent_exemption(Mint::LEN);

    let create = system_instruction::create_account(
        &payer.pubkey(),
        &mint.pubkey(),
        rent,
        Mint::LEN as u64,
        &TOKEN_PROGRAM_ID,
    );

    let init = token_instruction::initialize_mint(
        &TOKEN_PROGRAM_ID,
        &mint.pubkey(),
        &payer.pubkey(),
        None,
        6,
    )
    .unwrap();

    send_tx(
        svm,
        vec![create, init],
        vec![payer, mint],
    );
}

fn create_ata(
    svm: &mut LiteSVM,
    payer: &Keypair,
    owner: &Pubkey,
    mint: &Pubkey,
) -> Pubkey {
    let ata = get_associated_token_address(owner, mint);

    let ix =
        spl_associated_token_account::instruction::create_associated_token_account(
            &payer.pubkey(),
            owner,
            mint,
            &TOKEN_PROGRAM_ID,
        );

    send_tx(svm, vec![ix], vec![payer]);

    ata
}

fn mint_tokens(
    svm: &mut LiteSVM,
    payer: &Keypair,
    mint: &Pubkey,
    destination: &Pubkey,
    amount: u64,
) {
    let ix = token_instruction::mint_to(
        &TOKEN_PROGRAM_ID,
        mint,
        destination,
        &payer.pubkey(),
        &[],
        amount,
    )
    .unwrap();

    send_tx(svm, vec![ix], vec![payer]);
}

fn token_balance(
    svm: &LiteSVM,
    account: &Pubkey,
) -> u64 {
    let account = svm.get_account(account).unwrap();

    TokenAccount::unpack(&account.data)
        .unwrap()
        .amount
}

#[test]
fn test_full_amm_flow() {
    let mut svm = LiteSVM::new();

    // Load our compiled Anchor program.
    svm.add_program(
        amm_program::ID,
        include_bytes!("../../../target/deploy/amm_program.so"),
    );

    // -------------------------
    // Accounts
    // -------------------------

    let payer = Keypair::new();
    let treasury = Keypair::new();

    svm.airdrop(&payer.pubkey(), 10_000_000_000)
        .unwrap();

    svm.airdrop(&treasury.pubkey(), 1_000_000)
        .unwrap();

    // -------------------------
    // Create X and Y mints
    // -------------------------

    let mint_x = Keypair::new();
    let mint_y = Keypair::new();

    create_mint(&mut svm, &payer, &mint_x);
    create_mint(&mut svm, &payer, &mint_y);

    // -------------------------
    // User token accounts
    // -------------------------

    let user_x = create_ata(
        &mut svm,
        &payer,
        &payer.pubkey(),
        &mint_x.pubkey(),
    );

    let user_y = create_ata(
        &mut svm,
        &payer,
        &payer.pubkey(),
        &mint_y.pubkey(),
    );

    // Give user tokens.
    mint_tokens(
        &mut svm,
        &payer,
        &mint_x.pubkey(),
        &user_x,
        1_000_000_000,
    );

    mint_tokens(
        &mut svm,
        &payer,
        &mint_y.pubkey(),
        &user_y,
        1_000_000_000,
    );

    // -------------------------
    // Derive AMM PDAs
    // -------------------------

    let seed: u64 = 1;

    let seed_bytes = seed.to_le_bytes();

    let (config, _) = Pubkey::find_program_address(
        &[b"amm_config", seed_bytes.as_ref()],
        &amm_program::ID,
    );

    let (lp_mint, _) = Pubkey::find_program_address(
        &[b"lp", config.as_ref()],
        &amm_program::ID,
    );

    let vault_x =
        get_associated_token_address(&config, &mint_x.pubkey());

    let vault_y =
        get_associated_token_address(&config, &mint_y.pubkey());

    // -------------------------
    // Initialize
    // -------------------------

    let initialize_ix = Instruction {
        program_id: amm_program::ID,
        accounts: amm_program::accounts::Initialize {
            payer: payer.pubkey(),
            mint_x: mint_x.pubkey(),
            mint_y: mint_y.pubkey(),
            treasury: treasury.pubkey(),
            config,
            vault_x,
            vault_y,
            lp_mint,
            associated_token_program:
                spl_associated_token_account::ID,
            token_program: TOKEN_PROGRAM_ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: amm_program::instruction::Initialize {
            seed,
            fee: 30,
            protocol_fee: 10,
        }
        .data(),
    };

    send_tx(
        &mut svm,
        vec![initialize_ix],
        vec![&payer],
    );

    // Check config was created.
    let config_account = svm.get_account(&config).unwrap();

    let mut data: &[u8] = &config_account.data;

    let config_state =
        AmmConfig::try_deserialize(&mut data).unwrap();

    assert_eq!(config_state.seed, seed);
    assert_eq!(config_state.mint_x, mint_x.pubkey());
    assert_eq!(config_state.mint_y, mint_y.pubkey());
    assert_eq!(config_state.lp_mint, lp_mint);
    assert_eq!(config_state.treasury, treasury.pubkey());
    assert_eq!(config_state.fee, 30);
    assert_eq!(config_state.protocol_fee, 10);

    println!("Initialize");

    // -------------------------
    // Deposit
    // -------------------------

    let user_lp =
        get_associated_token_address(&payer.pubkey(), &lp_mint);

    let deposit_ix = Instruction {
        program_id: amm_program::ID,
        accounts: amm_program::accounts::Deposit {
            user: payer.pubkey(),
            mint_x: mint_x.pubkey(),
            mint_y: mint_y.pubkey(),
            config,
            lp_mint,
            vault_x,
            vault_y,
            user_x,
            user_y,
            user_lp,
            associated_token_program:
                spl_associated_token_account::ID,
            token_program: TOKEN_PROGRAM_ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: amm_program::instruction::Deposit {
            amount: 100_000_000,
            max_x: 200_000_000,
            max_y: 200_000_000,
        }
        .data(),
    };

    send_tx(
        &mut svm,
        vec![deposit_ix],
        vec![&payer],
    );

    assert_eq!(
        token_balance(&svm, &vault_x),
        200_000_000
    );

    assert_eq!(
        token_balance(&svm, &vault_y),
        200_000_000
    );

    assert_eq!(
        token_balance(&svm, &user_lp),
        100_000_000
    );

    println!("Deposit");

    // -------------------------
    // Swap X -> Y
    // -------------------------

    let treasury_x =
        get_associated_token_address(
            &treasury.pubkey(),
            &mint_x.pubkey(),
        );

    let treasury_y =
        get_associated_token_address(
            &treasury.pubkey(),
            &mint_y.pubkey(),
        );

    let swap_amount = 10_000_000u64;

    let protocol_cut =
        swap_amount * 10 / 10_000;

    let swap_ix = Instruction {
        program_id: amm_program::ID,
        accounts: amm_program::accounts::Swap {
            user: payer.pubkey(),
            mint_x: mint_x.pubkey(),
            mint_y: mint_y.pubkey(),
            config,
            lp_mint,
            vault_x,
            vault_y,
            user_x,
            user_y,
            treasury: treasury.pubkey(),
            treasury_x,
            treasury_y,
            associated_token_program:
                spl_associated_token_account::ID,
            token_program: TOKEN_PROGRAM_ID,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
        data: amm_program::instruction::Swap {
            is_x: true,
            amount: swap_amount,
            min: 1,
        }
        .data(),
    };

    send_tx(
        &mut svm,
        vec![swap_ix],
        vec![&payer],
    );

    assert_eq!(
        token_balance(&svm, &treasury_x),
        protocol_cut
    );

    println!("Swap + protocol fee");

    // -------------------------
    // Withdraw
    // -------------------------

    let withdraw_ix = Instruction {
        program_id: amm_program::ID,
        accounts: amm_program::accounts::Withdraw {
            user: payer.pubkey(),
            mint_x: mint_x.pubkey(),
            mint_y: mint_y.pubkey(),
            config,
            lp_mint,
            vault_x,
            vault_y,
            user_x,
            user_y,
            user_lp,
            token_program: TOKEN_PROGRAM_ID,
            system_program: system_program::ID,
            associated_token_program:
                spl_associated_token_account::ID,
        }
        .to_account_metas(None),
        data: amm_program::instruction::Withdraw {
            amount: 100_000_000,
            min_x: 1,
            min_y: 1,
        }
        .data(),
    };

    send_tx(
        &mut svm,
        vec![withdraw_ix],
        vec![&payer],
    );

    assert_eq!(
        token_balance(&svm, &user_lp),
        0
    );

    println!("Withdraw");

    println!("FULL AMM TEST PASSED");
}