# QoraNet 🌊⚡

**A next-generation blockchain powered by Proof of Liquidity and distributed application hosting**

QoraNet revolutionizes blockchain consensus by combining liquidity provision with useful computational work. Users earn rewards by providing liquidity to DEX pools AND hosting applications on their systems, creating a truly productive and economically efficient network.

## 🎯 Key Features

- **Proof of Liquidity (PoL)** - Consensus mechanism based on verified LP token holdings
- **Distributed App Hosting** - Users run applications to earn additional rewards
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
git clone https://github.com/yourusername/qoranet.git
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
│   ├── transaction/       # Transaction processing
│   ├── storage/          # Blockchain data storage
│   ├── rpc/              # RPC API server
│   ├── app_monitor/      # Application performance monitoring
│   ├── rewards/          # Reward calculation and distribution
│   └── lib.rs            # Main library entry point
├── programs/             # Smart contracts and programs
├── tools/               # CLI tools and utilities
├── tests/              # Integration tests
├── docs/               # Documentation
└── examples/           # Usage examples
```

## 🔧 Development Status

- [ ] Core blockchain infrastructure
- [ ] Proof of Liquidity consensus implementation
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

### 3. Reward Mechanism
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

### Phase 1: Foundation (Q1 2024)
- [ ] Core blockchain implementation
- [ ] Basic consensus mechanism
- [ ] LP token integration

### Phase 2: Proof of Liquidity (Q2 2024)  
- [ ] Full PoL consensus
- [ ] Application monitoring
- [ ] Reward distribution

### Phase 3: Ecosystem (Q3 2024)
- [ ] Solana program compatibility
- [ ] Developer tooling
- [ ] Mainnet launch

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
