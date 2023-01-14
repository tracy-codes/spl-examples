use std::error::Error;

use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction,
    native_token::LAMPORTS_PER_SOL,
    program_pack::Pack,
    pubkey::Pubkey,
    signature::Signature,
    signer::{keypair::Keypair, Signer},
    transaction::Transaction,
};
use spl_token::state::Mint;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let signer_wallet = Keypair::new();
    let receiver_wallet = Keypair::new();
    let mint_account = Keypair::new();
    let client = RpcClient::new("http://localhost:8899");
    println!("Requesting airdrop to fund this testing...");
    request_airdrop(&client, &signer_wallet.pubkey())?;
    println!("Airdrop success.");

    let decimals = 9;

    let minimum_balance_for_rent_exemption =
        client.get_minimum_balance_for_rent_exemption(Mint::LEN)?;

    println!("Building initialize mint transaction...");

    let create_account_instruction: Instruction = solana_sdk::system_instruction::create_account(
        &signer_wallet.pubkey(),
        &mint_account.pubkey(),
        minimum_balance_for_rent_exemption,
        Mint::LEN as u64,
        &spl_token::ID,
    );

    let initialize_mint_instruction: Instruction = spl_token::instruction::initialize_mint(
        &spl_token::ID,
        &mint_account.pubkey(),
        &signer_wallet.pubkey(),
        None,
        decimals,
    )?;

    let recent_blockhash = client.get_latest_blockhash()?;

    let transaction: Transaction = Transaction::new_signed_with_payer(
        &[create_account_instruction, initialize_mint_instruction],
        Some(&signer_wallet.pubkey()),
        &[&mint_account, &signer_wallet],
        recent_blockhash,
    );

    client.send_and_confirm_transaction_with_spinner(&transaction)?;

    println!(
        "SPL Token mint account with {} decimals created successfully:\n{}",
        decimals,
        mint_account.pubkey().to_string()
    );

    let amount = 10_000 * 10_u64.pow(9);

    let signer_ata = spl_associated_token_account::get_associated_token_address(
        &signer_wallet.pubkey(),
        &mint_account.pubkey(),
    );
    #[allow(deprecated)]
    let signer_ata_init_ix = spl_associated_token_account::create_associated_token_account(
        &signer_wallet.pubkey(),
        &signer_wallet.pubkey(),
        &mint_account.pubkey(),
    );
    let mint_to_ix: Instruction = spl_token::instruction::mint_to(
        &spl_token::ID,
        &mint_account.pubkey(),
        &signer_ata,
        &signer_wallet.pubkey(),
        &[&signer_wallet.pubkey()],
        amount,
    )?;

    println!(
        "Preparing to mint {} tokens to account {} of wallet {}",
        amount,
        signer_ata.to_string(),
        signer_wallet.pubkey().to_string()
    );
    let recent_blockhash = client.get_latest_blockhash()?;
    let transaction: Transaction = Transaction::new_signed_with_payer(
        &[signer_ata_init_ix, mint_to_ix],
        Some(&signer_wallet.pubkey()),
        &[&signer_wallet],
        recent_blockhash,
    );
    client.send_and_confirm_transaction_with_spinner(&transaction)?;
    println!("SPL Tokens minted successfully.");
    println!("Amount: {}", amount);
    println!("Receiver pubkey: {}", signer_wallet.pubkey().to_string());
    println!("Associated token account: {}", signer_ata.to_string());

    let receiver_ata = spl_associated_token_account::get_associated_token_address(
        &receiver_wallet.pubkey(),
        &mint_account.pubkey(),
    );
    #[allow(deprecated)]
    let receiver_ata_init_ix = spl_associated_token_account::create_associated_token_account(
        &signer_wallet.pubkey(),
        &receiver_wallet.pubkey(),
        &mint_account.pubkey(),
    );
    let transfer_to_ix = spl_token::instruction::transfer(
        &spl_token::ID,
        &signer_ata,
        &receiver_ata,
        &signer_wallet.pubkey(),
        &[&signer_wallet.pubkey()],
        amount,
    )?;
    // If ATA doesn't exist we need to create it before sending tokens
    match client.get_token_account_balance(&receiver_ata) {
        Err(_) => {
            println!(
                "Receiver account {} doesn't already exist. We need to initialize it.",
                receiver_ata.to_string()
            );
            let recent_blockhash = client.get_latest_blockhash()?;
            let transaction = Transaction::new_signed_with_payer(
                &[receiver_ata_init_ix, transfer_to_ix],
                Some(&signer_wallet.pubkey()),
                &[&signer_wallet],
                recent_blockhash,
            );
            client.send_and_confirm_transaction_with_spinner(&transaction)?;
        }
        Ok(_) => {
            println!(
                "Receiver account {} already exists! Just need to send a transfer from {}.",
                receiver_ata.to_string(),
                signer_wallet.pubkey().to_string()
            );
            let recent_blockhash = client.get_latest_blockhash()?;
            let transaction = Transaction::new_signed_with_payer(
                &[transfer_to_ix],
                Some(&signer_wallet.pubkey()),
                &[&signer_wallet],
                recent_blockhash,
            );
            client.send_and_confirm_transaction_with_spinner(&transaction)?;
        }
    }

    let signer_bal = client.get_token_account_balance(&signer_ata)?;
    let receiver_bal = client.get_token_account_balance(&receiver_ata)?;

    println!("SPL Tokens Transferred Successfully.");
    println!("Signer Account: {:?}", signer_ata.to_string());
    println!("Signer Balance: {:?}", signer_bal);
    println!("Receiver Account: {:?}", receiver_ata.to_string());
    println!("Receiver Balance: {:?}", receiver_bal);
    Ok(())
}

fn request_airdrop(client: &RpcClient, pubkey: &Pubkey) -> Result<Signature, Box<dyn Error>> {
    let sig = client.request_airdrop(&pubkey, (10 * LAMPORTS_PER_SOL) as u64)?;
    println!("Awaiting confirmation for sig {:?}", sig.to_string());
    loop {
        let confirmed = client.confirm_transaction(&sig)?;
        if confirmed {
            break;
        }
    }
    Ok(sig)
}
