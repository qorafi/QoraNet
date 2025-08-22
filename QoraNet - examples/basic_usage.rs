use qoranet::{
    transaction::{Transaction, TransactionData, TransactionPool},
    fee_oracle::{GlobalFeeOracle, FeePriority, TransactionType},
    Address, Balance, LPToken,
};
use ed25519_dalek::Keypair;
use rand::rngs::OsRng;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌊 QoraNet Basic Usage Example");
    println!("================================");
    
    // Initialize fee oracle
    let fee_oracle = GlobalFeeOracle::new();
    
    // Update price (in real implementation, this would fetch from external sources)
    fee_oracle.update_price().await?;
    let qor_price = fee_oracle.get_qor_price().await;
    println!("💰 Current QOR price: ${:.6}", qor_price);
    
    // Generate keypairs for demo
    let mut csprng = OsRng;
    let alice_keypair = Keypair::generate(&mut csprng);
    let bob_keypair = Keypair::generate(&mut csprng);
    
    let alice_address = Address::from_pubkey(&alice_keypair.public);
    let bob_address = Address::from_pubkey(&bob_keypair.public);
    
    println!("👤 Alice: {}", alice_address);
    println!("👤 Bob: {}", bob_address);
    
    // Create balances
    let mut alice_balance = Balance::from_qor(1000.0); // 1000 QOR
    let bob_balance = Balance::from_qor(500.0);        // 500 QOR
    
    println!("💳 Alice balance: {}", alice_balance);
    println!("💳 Bob balance: {}", bob_balance);
    
    // Example 1: Get fee estimates for different transaction types
    println!("\n📊 Fee Estimates:");
    println!("------------------");
    
    let transfer_estimate = fee_oracle.get_fee_estimate(&TransactionType::Transfer).await;
    println!("Transfer fees:");
    println!("  Low: {} QOR (${:.6})", 
        Balance::new(transfer_estimate.low), 
        transfer_estimate.get_usd_fee(FeePriority::Low)
    );
    println!("  Medium: {} QOR (${:.6})", 
        Balance::new(transfer_estimate.medium), 
        transfer_estimate.get_usd_fee(FeePriority::Medium)
    );
    println!("  High: {} QOR (${:.6})", 
        Balance::new(transfer_estimate.high), 
        transfer_estimate.get_usd_fee(FeePriority::High)
    );
    
    let lp_estimate = fee_oracle.get_fee_estimate(&TransactionType::ProvideLiquidity).await;
    println!("Provide Liquidity fees:");
    println!("  Medium: {} QOR (${:.6})", 
        Balance::new(lp_estimate.medium), 
        lp_estimate.get_usd_fee(FeePriority::Medium)
    );
    
    // Example 2: Create a simple transfer transaction
    println!("\n💸 Creating Transfer Transaction:");
    println!("----------------------------------");
    
    let transfer_amount = Balance::from_qor(50.0).amount; // 50 QOR
    let transfer_data = TransactionData::Transfer {
        from: alice_address.clone(),
        to: bob_address.clone(),
        amount: transfer_amount,
    };
    
    let transfer_tx = Transaction::new(
        transfer_data,
        1, // nonce
        FeePriority::Medium,
        &alice_keypair,
        &fee_oracle
    ).await?;
    
    println!("✅ Transfer transaction created:");
    println!("  Amount: {} QOR", Balance::new(transfer_amount));
    println!("  Fee: {} QOR (${:.6})", Balance::new(transfer_tx.fee_qor), transfer_tx.fee_usd);
    println!("  Hash: {}", transfer_tx.hash());
    
    // Update Alice's balance (subtract transfer + fee)
    alice_balance.subtract(transfer_amount)?;
    alice_balance.subtract(transfer_tx.fee_qor)?;
    println!("  Alice balance after: {}", alice_balance);
    
    // Example 3: Create LP provision transaction
    println!("\n🏊 Creating Liquidity Provision Transaction:");
    println!("---------------------------------------------");
    
    let lp_tokens = vec![
        LPToken {
            pool_address: Address::from_pubkey(&bob_keypair.public), // Mock pool address
            amount: Balance::from_qor(100.0).amount,
            token_a: alice_address.clone(),
            token_b: bob_address.clone(),
        }
    ];
    
    let lp_data = TransactionData::ProvideLiquidity {
        provider: alice_address.clone(),
        lp_tokens,
    };
    
    let lp_tx = Transaction::new(
        lp_data,
        2, // nonce
        FeePriority::High, // Higher priority for LP transactions
        &alice_keypair,
        &fee_oracle
    ).await?;
    
    println!("✅ LP provision transaction created:");
    println!("  LP Amount: {} QOR", Balance::from_qor(100.0));
    println!("  Fee: {} QOR (${:.6})", Balance::new(lp_tx.fee_qor), lp_tx.fee_usd);
    println!("  Hash: {}", lp_tx.hash());
    
    // Example 4: Transaction pool usage
    println!("\n📦 Transaction Pool:");
    println!("--------------------");
    
    let mut tx_pool = TransactionPool::new();
    
    // Add transactions to pool
    tx_pool.add_transaction(transfer_tx, &fee_oracle).await?;
    tx_pool.add_transaction(lp_tx, &fee_oracle).await?;
    
    println!("✅ Added transactions to pool");
    println!("  Pending transactions: {}", tx_pool.pending_count());
    
    // Get transactions for block (sorted by priority and fee)
    let block_txs = tx_pool.get_transactions_for_block(10);
    println!("  Transactions for next block: {}", block_txs.len());
    
    for (i, tx) in block_txs.iter().enumerate() {
        println!("    {}. Priority: {:?}, Fee: {} QOR", 
            i + 1, 
            tx.priority, 
            Balance::new(tx.fee_qor)
        );
    }
    
    // Example 5: Fee validation
    println!("\n🔍 Fee Validation:");
    println!("------------------");
    
    // Try to validate a low fee
    let low_fee = qoranet::usd_to_qor(0.00005, qor_price); // Below minimum
    match fee_oracle.validate_fee(low_fee, &TransactionType::Transfer).await {
        Ok(_) => println!("✅ Low fee is valid"),
        Err(e) => println!("❌ Low fee rejected: {}", e),
    }
    
    // Validate a proper fee
    let proper_fee = qoranet::usd_to_qor(0.0002, qor_price); // Above minimum
    match fee_oracle.validate_fee(proper_fee, &TransactionType::Transfer).await {
        Ok(_) => println!("✅ Proper fee is valid"),
        Err(e) => println!("❌ Proper fee rejected: {}", e),
    }
    
    println!("\n🎉 QoraNet example completed successfully!");
    
    Ok(())
}
