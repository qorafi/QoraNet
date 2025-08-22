// examples/erc20_demo.rs - How to use ERC-20 compatibility

use qoranet::{
    QoraNet, 
    TransactionType, 
    Transaction,
    qrc20::QRC20Transaction,
    evm::EVMTransaction,
    BridgeTransaction,
    wallet,
};
use primitive_types::{H160, H256, U256};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🌊 QoraNet ERC-20 Compatibility Demo");
    println!("=====================================\n");

    // Initialize QoraNet blockchain
    let mut qoranet = QoraNet::new();
    
    // Create some demo accounts with initial QOR
    let alice = qoranet.create_account(U256::from(1000) * U256::from(10).pow(9.into())); // 1000 QOR
    let bob = qoranet.create_account(U256::from(500) * U256::from(10).pow(9.into()));   // 500 QOR
    let charlie = qoranet.create_account(U256::from(200) * U256::from(10).pow(9.into())); // 200 QOR

    println!("👥 Created accounts:");
    println!("   Alice: 0x{:x} (1000 QOR)", alice);
    println!("   Bob:   0x{:x} (500 QOR)", bob);
    println!("   Charlie: 0x{:x} (200 QOR)", charlie);
    println!();

    // Demo 1: Deploy QRC-20 Token (Native QoraNet Standard)
    println!("🚀 Demo 1: Deploying QRC-20 Token");
    println!("----------------------------------");
    
    let usdc_contract = qoranet.qrc20_registry.deploy_token(
        alice,
        "QoraNet USD Coin".to_string(),
        "qUSDC".to_string(),
        6, // 6 decimals like real USDC
        U256::from(1_000_000) * U256::from(10).pow(6.into()), // 1M USDC
    )?;
    
    println!("✅ Deployed qUSDC token at: 0x{:x}", usdc_contract);
    println!("   Total Supply: {} qUSDC", wallet::format_token_balance(
        U256::from(1_000_000) * U256::from(10).pow(6.into()), 6
    ));
    println!();

    // Demo 2: QRC-20 Token Transfer
    println!("💸 Demo 2: QRC-20 Token Transfer");
    println!("---------------------------------");
    
    let transfer_amount = wallet::parse_token_amount("1000.50", 6)?; // 1000.50 USDC
    let transfer_tx = QRC20Transaction::Transfer {
        contract: usdc_contract,
        to: bob,
        amount: transfer_amount,
    };
    
    qoranet.process_qrc20_transaction(alice, transfer_tx, 50000)?;
    
    let alice_usdc = qoranet.get_token_balance(alice, Some(usdc_contract));
    let bob_usdc = qoranet.get_token_balance(bob, Some(usdc_contract));
    
    println!("✅ Transferred 1000.50 qUSDC from Alice to Bob");
    println!("   Alice qUSDC: {}", wallet::format_token_balance(alice_usdc, 6));
    println!("   Bob qUSDC:   {}", wallet::format_token_balance(bob_usdc, 6));
    println!();

    // Demo 3: ERC-20 via EVM (Full Ethereum Compatibility)
    println!("🔥 Demo 3: ERC-20 via EVM");
    println!("-------------------------");
    
    // This would deploy a real ERC-20 contract using Solidity bytecode
    let erc20_bytecode = generate_erc20_bytecode(
        "Ethereum USDT",
        "eUSDT", 
        6,
        U256::from(500_000) * U256::from(10).pow(6.into())
    );
    
    let evm_tx = EVMTransaction {
        from: alice,
        to: None, // Contract deployment
        value: U256::zero(),
        gas_limit: U256::from(2_000_000),
        gas_price: qoranet.gas_price,
        data: erc20_bytecode,
        nonce: U256::zero(),
    };
    
    let receipt = qoranet.process_evm_transaction(alice, evm_tx, 2_000_000)?;
    let erc20_contract = receipt.contract_address.unwrap();
    
    println!("✅ Deployed ERC-20 contract at: 0x{:x}", erc20_contract);
    println!("   Gas Used: {}", receipt.gas_used);
    println!();

    // Demo 4: Cross-chain Bridge
    println!("🌉 Demo 4: Cross-chain Bridge");
    println!("-----------------------------");
    
    let bridge_tx = BridgeTransaction::FromEthereum {
        eth_token: H160::from_low_u64_be(0xa0b86a33e6ba), // Mock Ethereum USDC
        amount: U256::from(5000) * U256::from(10).pow(6.into()), // 5000 USDC
        token_name: "USD Coin".to_string(),
        token_symbol: "USDC".to_string(),
        decimals: 6,
    };
    
    let bridge_receipt = qoranet.process_bridge_transaction(charlie, bridge_tx, 150000)?;
    let bridged_usdc = bridge_receipt.contract_address.unwrap();
    
    let charlie_bridged_balance = qoranet.get_token_balance(charlie, Some(bridged_usdc));
    
    println!("✅ Bridged 5000 USDC from Ethereum to QoraNet");
    println!("   Bridged contract: 0x{:x}", bridged_usdc);
    println!("   Charlie balance: {} bUSDC", wallet::format_token_balance(charlie_bridged_balance, 6));
    println!();

    // Demo 5: Multi-token Portfolio View
    println!("📊 Demo 5: Account Portfolio");
    println!("-----------------------------");
    
    let alice_info = qoranet.get_account_info(alice);
    println!("Alice's Portfolio:");
    println!("  QOR: {} QOR", wallet::format_token_balance(alice_info.qor_balance, 9));
    
    for (contract, token_balance) in alice_info.token_balances {
        println!("  {}: {} {}", 
            token_balance.symbol,
            wallet::format_token_balance(token_balance.balance, token_balance.decimals),
            token_balance.symbol
        );
    }
    println!();

    // Demo 6: Token Allowances and Approvals
    println!("🤝 Demo 6: Token Approvals");
    println!("--------------------------");
    
    let approval_amount = U256::from(100) * U256::from(10).pow(6.into()); // 100 USDC
    let approve_tx = QRC20Transaction::Approve {
        contract: usdc_contract,
        spender: charlie,
        amount: approval_amount,
    };
    
    qoranet.process_qrc20_transaction(bob, approve_tx, 30000)?;
    
    let allowance = qoranet.qrc20_registry
        .get_token(usdc_contract)
        .unwrap()
        .allowance(bob, charlie);
    
    println!("✅ Bob approved Charlie to spend 100 qUSDC");
    println!("   Allowance: {} qUSDC", wallet::format_token_balance(allowance, 6));
    
    // Charlie spends on Bob's behalf
    let transfer_from_tx = QRC20Transaction::TransferFrom {
        contract: usdc_contract,
        from: bob,
        to: charlie,
        amount: U256::from(50) * U256::from(10).pow(6.into()), // 50 USDC
    };
    
    qoranet.process_qrc20_transaction(charlie, transfer_from_tx, 50000)?;
    
    let charlie_usdc = qoranet.get_token_balance(charlie, Some(usdc_contract));
    println!("✅ Charlie spent 50 qUSDC on Bob's behalf");
    println!("   Charlie qUSDC: {}", wallet::format_token_balance(charlie_usdc, 6));
    println!();

    // Demo 7: Gas Cost Analysis
    println!("⛽ Demo 7: Gas Costs in QOR");
    println!("---------------------------");
    
    let gas_price = qoranet.get_gas_price_in_qor();
    println!("Current gas price: {} QOR per gas unit", 
        wallet::format_token_balance(gas_price, 9));
    
    println!("Typical transaction costs:");
    println!("  QOR Transfer:       {} QOR", 
        wallet::format_token_balance(gas_price * 21000, 9));
    println!("  QRC-20 Transfer:    {} QOR", 
        wallet::format_token_balance(gas_price * 50000, 9));
    println!("  ERC-20 Deployment:  {} QOR", 
        wallet::format_token_balance(gas_price * 2000000, 9));
    println!("  Bridge Transaction: {} QOR", 
        wallet::format_token_balance(gas_price * 150000, 9));
    println!();

    // Demo 8: Network Statistics
    println!("📈 Demo 8: Network Statistics");
    println!("-----------------------------");
    
    let total_tokens = qoranet.qrc20_registry.list_tokens().len();
    let total_qor_supply = U256::from(21_000_000) * U256::from(10).pow(9.into()); // 21M QOR
    
    println!("QoraNet Network Stats:");
    println!("  Block Number: {}", qoranet.current_block);
    println!("  Total QOR Supply: {} QOR", wallet::format_token_balance(total_qor_supply, 9));
    println!("  QRC-20 Tokens: {}", total_tokens);
    println!("  EVM Compatible: ✅");
    println!("  Bridge Support: ✅");
    
    println!("\n🎉 Demo completed successfully!");
    println!("QoraNet now supports full ERC-20 compatibility through:");
    println!("  • Native QRC-20 standard (gas efficient)");
    println!("  • Full EVM integration (Solidity contracts)");
    println!("  • Cross-chain bridges (Ethereum ↔ QoraNet)");
    println!("  • Proof of Liquidity rewards for all token holders");

    Ok(())
}

/// Generate ERC-20 bytecode (simplified for demo)
/// In reality, you'd compile Solidity code
fn generate_erc20_bytecode(
    name: &str,
    symbol: &str,
    decimals: u8,
    total_supply: U256,
) -> Vec<u8> {
    // This is a simplified placeholder
    // Real implementation would compile Solidity:
    /*
    pragma solidity ^0.8.0;

    contract ERC20 {
        string public name;
        string public symbol;
        uint8 public decimals;
        uint256 public totalSupply;
        mapping(address => uint256) public balanceOf;
        mapping(address => mapping(address => uint256)) public allowance;
        
        constructor(string memory _name, string memory _symbol, uint8 _decimals, uint256 _totalSupply) {
            name = _name;
            symbol = _symbol;
            decimals = _decimals;
            totalSupply = _totalSupply;
            balanceOf[msg.sender] = _totalSupply;
        }
        
        function transfer(address to, uint256 amount) public returns (bool) {
            require(balanceOf[msg.sender] >= amount, "Insufficient balance");
            balanceOf[msg.sender] -= amount;
            balanceOf[to] += amount;
            return true;
        }
        
        function approve(address spender, uint256 amount) public returns (bool) {
            allowance[msg.sender][spender] = amount;
            return true;
        }
        
        function transferFrom(address from, address to, uint256 amount) public returns (bool) {
            require(balanceOf[from] >= amount, "Insufficient balance");
            require(allowance[from][msg.sender] >= amount, "Insufficient allowance");
            balanceOf[from] -= amount;
            balanceOf[to] += amount;
            allowance[from][msg.sender] -= amount;
            return true;
        }
    }
    */
    
    // Placeholder bytecode - in reality this would be much longer
    let mut bytecode = vec![
        0x60, 0x80, 0x60, 0x40, 0x52, // Contract setup
        0x34, 0x80, 0x15, 0x61, 0x00, 0x10, 0x57, 0x60, 0x00, 0x80, 0xfd, 0x5b, // Constructor
    ];
    
    // Encode constructor parameters (simplified)
    bytecode.extend_from_slice(&[decimals]); // decimals
    let mut supply_bytes = [0u8; 32];
    total_supply.to_big_endian(&mut supply_bytes);
    bytecode.extend_from_slice(&supply_bytes); // total supply
    
    // Add name and symbol (simplified encoding)
    bytecode.extend_from_slice(name.as_bytes());
    bytecode.extend_from_slice(symbol.as_bytes());
    
    bytecode
}

/// Integration with existing QoraNet CLI
#[cfg(feature = "cli")]
pub mod cli {
    use super::*;
    use clap::{App, Arg, SubCommand};

    pub fn run_cli() -> Result<(), Box<dyn std::error::Error>> {
        let matches = App::new("qoranet-tokens")
            .version("1.0")
            .about("QoraNet ERC-20 Compatible Token Manager")
            .subcommand(
                SubCommand::with_name("deploy")
                    .about("Deploy new QRC-20 token")
                    .arg(Arg::with_name("name").required(true))
                    .arg(Arg::with_name("symbol").required(true))
                    .arg(Arg::with_name("decimals").required(true))
                    .arg(Arg::with_name("supply").required(true))
            )
            .subcommand(
                SubCommand::with_name("transfer")
                    .about("Transfer tokens")
                    .arg(Arg::with_name("token").required(true))
                    .arg(Arg::with_name("to").required(true))
                    .arg(Arg::with_name("amount").required(true))
            )
            .subcommand(
                SubCommand::with_name("balance")
                    .about("Check token balance")
                    .arg(Arg::with_name("account").required(true))
                    .arg(Arg::with_name("token"))
            )
            .get_matches();

        let mut qoranet = QoraNet::new();

        match matches.subcommand() {
            ("deploy", Some(deploy_matches)) => {
                let name = deploy_matches.value_of("name").unwrap();
                let symbol = deploy_matches.value_of("symbol").unwrap();
                let decimals: u8 = deploy_matches.value_of("decimals").unwrap().parse()?;
                let supply = wallet::parse_token_amount(
                    deploy_matches.value_of("supply").unwrap(), 
                    decimals
                )?;
                
                let deployer = qoranet.create_account(U256::from(10) * U256::from(10).pow(9.into()));
                let contract = qoranet.qrc20_registry.deploy_token(
                    deployer, name.to_string(), symbol.to_string(), decimals, supply
                )?;
                
                println!("Token deployed at: 0x{:x}", contract);
            }
            ("balance", Some(balance_matches)) => {
                let account_str = balance_matches.value_of("account").unwrap();
                let account = H160::from_slice(&hex::decode(&account_str[2..])?);
                
                let token = balance_matches.value_of("token").map(|s| {
                    H160::from_slice(&hex::decode(&s[2..]).unwrap())
                });
                
                let balance = qoranet.get_token_balance(account, token);
                let decimals = if let Some(contract) = token {
                    qoranet.qrc20_registry.get_token(contract)
                        .map(|t| t.decimals)
                        .unwrap_or(9)
                } else {
                    9 // QOR decimals
                };
                
                println!("Balance: {}", wallet::format_token_balance(balance, decimals));
            }
            _ => {
                println!("Use --help for available commands");
            }
        }

        Ok(())
    }
}
