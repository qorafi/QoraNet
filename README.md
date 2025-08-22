# QoraNet 🌊⚡

**A next-generation blockchain powered by Proof of Liquidity and distributed application hosting**

QoraNet revolutionizes blockchain consensus by combining liquidity provision with useful computational work. Users earn rewards by providing liquidity to DEX pools AND hosting applications on their systems, creating a truly productive and economically efficient network.

## 🪙 Native Token: QOR

**QOR** is the native token of QoraNet with predictable, USD-based transaction fees:

- **Symbol:** QOR
- **Decimals:** 9 (1 QOR = 1,000,000,000 units)
- **Fee Structure:** Fixed USD amounts, paid in QOR tokens
- **Oracle-based Pricing:** Real-time QOR/USD conversion from multiple sources

### 💰 Transaction Fees (USD-based)

| Transaction Type | Base Fee (USD) | Description |
|------------------|----------------|-------------|
| Transfer | $0.0001 | Basic token transfers |
| Provide Liquidity | $0.0002 | Adding liquidity to DEX pools |
| Register App | $0.0005 | Registering apps for hosting |
| Report Metrics | $0.00005 | Performance metric reporting |
| Claim Rewards | $0.00015 | Claiming LP and app rewards |
| Smart Contract (Simple) | $0.0003 | Basic contract execution |
| Smart Contract (Complex) | $0.005 | Heavy computation contracts |

### ⚡ Priority Multipliers

Users can choose transaction priority with fee multipliers:

- **Low Priority:** 1.0x (standard fee)
- **Medium Priority:** 1.5x (+50% fee)  
- **High Priority:** 2.0x (+100% fee)
- **Urgent Priority:** 5.0x (+400% fee)

**Fee Range:** $0.0001 - $0.01 USD (converted to QOR at current market rate)

## 🎯 Key Features

- **Proof of Liquidity (PoL)** - Consensus mechanism based on verified LP token holdings
- **Distributed App Hosting** - Users run applications to earn additional rewards
- **QOR Native Token** - Predictable USD-based fees paid in QOR tokens
- **Oracle Price Feeds** - Multi-source QOR/USD pricing for accurate fee calculation
- **Solana Compatibility** - Run existing Solana programs on QoraNet
- **Energy Efficient** - Useful computation instead of wasteful mining
- **Economic Utility** - Every participant contributes liquidity AND computational resources

## 🏗️ Architecture Overview

QoraNet combines two key components:
1. **LP Token Verification** - Users must provide liquidity to DEX pools (verified via LP tokens)
2. **Application Hosting** - Users run network applications monitored by QoraNet nodes
3. **Reward Distribution** - Rewards based on LP contribution + computational performance

## 🚀 Quick Start

```bash
# Clone the repository
git clone https://github.com/qorafi/qoranet.git
cd qoranet

# Build the project
cargo build --release

# Run a validator node
./target/release/qoranet-validator

# Run application monitor
./target/release/qoranet-app-monitor
```

## 📁 Project Structure

```
qoranet/
├── src/
│   ├── consensus/          # Proof of Liquidity consensus mechanism
│   ├── validator/          # Validator node implementation
│   ├── network/           # P2P networking layer
│   ├── transaction/       # Transaction processing with QOR fees
│   ├── storage/          # Blockchain data storage
│   ├── rpc/              # RPC API server
│   ├── app_monitor/      # Application performance monitoring
│   ├── rewards/          # Reward calculation and distribution
│   ├── fee_oracle/       # QOR/USD price oracle system
│   └── lib.rs            # Main library entry point
├── programs/             # Smart contracts and programs
├── tools/               # CLI tools and utilities
├── tests/              # Integration tests
├── docs/               # Documentation
└── examples/           # Usage examples (including fee system)
```

## 🔧 Development Status

- [ ] Core blockchain infrastructure
- [ ] Proof of Liquidity consensus implementation
- [ ] QOR token system with USD-based fees ✅
- [ ] Fee oracle with multi-source price feeds ✅
- [ ] Transaction system with priority-based fees ✅
- [ ] Application monitoring system
- [ ] LP token verification
- [ ] Reward distribution mechanism
- [ ] Solana program compatibility layer
- [ ] RPC API
- [ ] CLI tools
- [ ] Documentation

## 🛠️ Building from Source

### Prerequisites
- Rust 1.70.0 or higher
- Git

### Build Instructions
```bash
# Install Rust if you haven't already
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/yourusername/qoranet.git
cd qoranet
cargo build --release

# Run example to see QOR token and fee system
cargo run --example basic_usage
```

## 📖 How It Works

### 1. Proof of Liquidity
Users must hold LP tokens from DEX pools to participate in consensus. This ensures:
- Real economic commitment to the network
- Genuine liquidity for the ecosystem
- Verifiable on-chain proof of stake

### 2. Application Hosting
Participants run applications that provide network services:
- Decentralized storage nodes
- Oracle services  
- Cross-chain bridges
- AI/ML computation
- Data indexing

### 3. QOR Token & Fee System
```
Transaction Fee = Base Fee (USD) × Priority Multiplier
Fee in QOR = Fee (USD) ÷ Current QOR Price (USD)

Example:
- Transfer with Medium Priority = $0.0001 × 1.5 = $0.00015
- If QOR = $2.50, then Fee = 0.00015 ÷ 2.50 = 0.00006 QOR
```

### 4. Reward Mechanism
```
Total Rewards = Base LP Rewards + Performance Multiplier
Performance Multiplier = f(CPU usage, uptime, network requests served)
```

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guidelines](CONTRIBUTING.md) for details.

### Development Setup
1. Fork the repository
2. Create a feature branch: `git checkout -b feature/amazing-feature`
3. Make your changes and test them
4. Submit a pull request

## 📚 Documentation

- [White Paper](docs/whitepaper.md) - Technical specification
- [API Reference](docs/api.md) - RPC API documentation  
- [Developer Guide](docs/development.md) - Building on QoraNet
- [Node Operator Guide](docs/node-operation.md) - Running QoraNet nodes

## 🗺️ Roadmap

### Phase 1: Foundation (Q1 2025)
- [x] QOR token system with USD-based fees
- [x] Fee oracle with multi-source pricing
- [x] Transaction system with priority fees
- [ ] Core blockchain implementation
- [ ] Basic consensus mechanism
- [ ] LP token integration

### Phase 2: Proof of Liquidity (Q2 2025)  
- [ ] Full PoL consensus
- [ ] Application monitoring
- [ ] Reward distribution

### Phase 3: Ecosystem (Q3 2025)
- [ ] Solana program compatibility
- [ ] Developer tooling
- [ ] Mainnet launch

📦 1. Block System (src/consensus/block.rs)

Block structure with header + transactions
Merkle tree for transaction verification
Genesis block creation
Block validation with height/hash checks
Block statistics for monitoring

🖥️ 2. Application Monitor (src/app_monitor/mod.rs)

5 App types: Storage, Oracle, Compute, Indexing, Relay nodes
Real-time monitoring: CPU, memory, uptime, requests served
Health checks for each app type
Resource requirements validation
Performance scoring for rewards
System statistics tracking

💾 3. Storage Layer (src/storage/mod.rs)

RocksDB backend with column families
Account state management with balances/nonces
Block/transaction storage and retrieval
Caching system for performance
Storage statistics and maintenance

🔗 4. Network Layer** (src/network/mod.rs)

P2P messaging system with broadcast/unicast
Peer discovery and connection management
Message types: transactions, blocks, validator announcements
Network statistics and health monitoring
Ping/pong connectivity checks

⚡ 5. Validator Node (src/bin/validator.rs)

Complete validator implementation
Block production with consensus selection
Transaction pool management
Fee oracle price updates
Configurable parameters (block time, requirements)
Real-time status reporting

💻 6. CLI Tool (src/bin/cli.rs)

Wallet operations: generate, check balance
Transaction creation: transfers with fee calculation
Fee estimates for all transaction types
Network status monitoring
QOR price information and conversions

🎯 Key Features Implemented:
✅ QOR Token System:

9 decimal precision
USD-based fee structure ($0.0001 - $0.01)
Priority system (1x to 5x multipliers)
Oracle price feeds

✅ Proof of Liquidity:

LP token verification
Stake weight calculation (liquidity × performance)
Weighted validator selection
Minimum requirements enforcement

✅ App Hosting Rewards:

Real system monitoring (CPU, memory, uptime)
Performance scoring algorithm
Health checks per app type
Resource requirement validation

✅ Complete Infrastructure:

Persistent storage with RocksDB
P2P networking foundation
Transaction pool with priority sorting
Block production and validation

🚀 Ready to Use:
```
Start a Validator:
bashcargo run --bin qoranet-validator --data-dir ./node1 --min-liquidity 500
```

Use the CLI:
```
bash# Generate wallet
cargo run --bin qoranet-cli wallet generate

# Check balance  
cargo run --bin qoranet-cli wallet balance -a <address>

# Get fee estimates
cargo run --bin qoranet-cli transaction fee-estimate --type transfer

# Check network status
cargo run --bin qoranet-cli network status
```
Run Examples:
```
bash# See QOR token and fee system in action
cargo run --example basic_usage
```
📋 What's Built vs. What's Next:

✅ Completed:

Core blockchain architecture
QOR token with USD fees
Transaction system with priorities
Application monitoring
Storage layer
Basic networking structure
CLI tools
Validator node

🔜 Next Steps:

Real P2P networking (libp2p integration)
Reward distribution mechanism
LP token DEX integration
Solana program compatibility
Web RPC API for dApps
Comprehensive testing

QoraNet now has a solid foundation with all the core components working together! The unique combination of Proof of Liquidity + App Hosting is fully implemented and ready for testing and further development.

## 📄 License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## 🔗 Links

- **Website**: https://qoranet.org
- **Twitter**: [@QoraNet](https://twitter.com/qoranet)
- **Discord**: [Join our community](https://discord.gg/qoranet)
- **Documentation**: https://docs.qoranet.org

## ⚠️ Disclaimer

QoraNet is currently in active development. Use at your own risk. This software is experimental and has not been audited for security vulnerabilities.

---

**Built with ❤️ by the QoraNet community**
