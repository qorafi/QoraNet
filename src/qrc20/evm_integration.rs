use evm::{
    executor::stack::{MemoryStackState, StackSubstateMetadata, StackState},
    Config, Context, CreateScheme, ExitReason, Handler, Runtime,
};
use primitive_types::{H160, H256, U256};
use std::collections::BTreeMap;
use serde::{Deserialize, Serialize};

/// QoraNet EVM compatibility layer for QRC-20 tokens
pub struct QoraNetEVM {
    /// EVM configuration
    config: Config,
    /// Account states
    accounts: BTreeMap<H160, Account>,
    /// Contract storage
    storage: BTreeMap<(H160, H256), H256>,
    /// Block context
    block_context: BlockContext,
}

#[derive(Debug, Clone)]
pub struct Account {
    pub balance: U256,
    pub nonce: U256,
    pub code: Vec<u8>,
}

#[derive(Debug, Clone)]
pub struct BlockContext {
    pub number: U256,
    pub timestamp: U256,
    pub difficulty: U256,
    pub gas_limit: U256,
    pub coinbase: H160, // QOR rewards recipient
    pub chain_id: U256,
}

impl QoraNetEVM {
    pub fn new() -> Self {
        Self {
            config: Config::istanbul(), // Use Istanbul hard fork rules
            accounts: BTreeMap::new(),
            storage: BTreeMap::new(),
            block_context: BlockContext {
                number: U256::zero(),
                timestamp: U256::from(std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()),
                difficulty: U256::zero(), // PoL doesn't use difficulty
                gas_limit: U256::from(30_000_000u64), // 30M gas limit
                coinbase: H160::zero(), // Set to QOR treasury
                chain_id: U256::from(2024), // QoraNet chain ID
            },
        }
    }

    /// Create EVM with custom configuration
    pub fn with_config(chain_id: u64, gas_limit: u64, coinbase: H160) -> Self {
        let mut evm = Self::new();
        evm.block_context.chain_id = U256::from(chain_id);
        evm.block_context.gas_limit = U256::from(gas_limit);
        evm.block_context.coinbase = coinbase;
        evm
    }

    /// Deploy ERC-20 contract
    pub fn deploy_erc20(
        &mut self,
        deployer: H160,
        name: String,
        symbol: String,
        decimals: u8,
        total_supply: U256,
    ) -> Result<H160, String> {
        // Generate ERC-20 bytecode
        let erc20_bytecode = self.generate_erc20_bytecode(&name, &symbol, decimals, total_supply);
        
        let create_address = self.create_address(&deployer, self.get_nonce(&deployer));
        
        // Execute contract creation
        let result = self.create_contract(deployer, erc20_bytecode, U256::zero())?;
        
        match result {
            ExitReason::Succeed(_) => {
                tracing::info!(
                    "Deployed ERC-20 contract {} ({}) at address {:?}",
                    name, symbol, create_address
                );
                Ok(create_address)
            },
            ExitReason::Revert(_) => Err("Contract deployment reverted".to_string()),
            ExitReason::Error(err) => Err(format!("Contract deployment error: {:?}", err)),
            ExitReason::Fatal(err) => Err(format!("Fatal error during deployment: {:?}", err)),
        }
    }

    /// Execute ERC-20 transfer
    pub fn erc20_transfer(
        &mut self,
        contract: H160,
        from: H160,
        to: H160,
        amount: U256,
    ) -> Result<bool, String> {
        // ERC-20 transfer function selector: 0xa9059cbb
        let mut input = vec![0xa9, 0x05, 0x9c, 0xbb];
        
        // Encode 'to' address (32 bytes)
        input.extend_from_slice(&[0u8; 12]); // Padding
        input.extend_from_slice(to.as_bytes());
        
        // Encode amount (32 bytes)
        let mut amount_bytes = [0u8; 32];
        amount.to_big_endian(&mut amount_bytes);
        input.extend_from_slice(&amount_bytes);

        let result = self.call_contract(from, contract, input, U256::zero())?;
        
        // Check if transfer succeeded (returns true)
        Ok(result.len() == 32 && result[31] == 1)
    }

    /// Execute ERC-20 transferFrom
    pub fn erc20_transfer_from(
        &mut self,
        contract: H160,
        spender: H160,
        from: H160,
        to: H160,
        amount: U256,
    ) -> Result<bool, String> {
        // ERC-20 transferFrom function selector: 0x23b872dd
        let mut input = vec![0x23, 0xb8, 0x72, 0xdd];
        
        // Encode 'from' address (32 bytes)
        input.extend_from_slice(&[0u8; 12]);
        input.extend_from_slice(from.as_bytes());
        
        // Encode 'to' address (32 bytes)
        input.extend_from_slice(&[0u8; 12]);
        input.extend_from_slice(to.as_bytes());
        
        // Encode amount (32 bytes)
        let mut amount_bytes = [0u8; 32];
        amount.to_big_endian(&mut amount_bytes);
        input.extend_from_slice(&amount_bytes);

        let result = self.call_contract(spender, contract, input, U256::zero())?;
        Ok(result.len() == 32 && result[31] == 1)
    }

    /// Execute ERC-20 approve
    pub fn erc20_approve(
        &mut self,
        contract: H160,
        owner: H160,
        spender: H160,
        amount: U256,
    ) -> Result<bool, String> {
        // ERC-20 approve function selector: 0x095ea7b3
        let mut input = vec![0x09, 0x5e, 0xa7, 0xb3];
        
        // Encode spender address (32 bytes)
        input.extend_from_slice(&[0u8; 12]);
        input.extend_from_slice(spender.as_bytes());
        
        // Encode amount (32 bytes)
        let mut amount_bytes = [0u8; 32];
        amount.to_big_endian(&mut amount_bytes);
        input.extend_from_slice(&amount_bytes);

        let result = self.call_contract(owner, contract, input, U256::zero())?;
        Ok(result.len() == 32 && result[31] == 1)
    }

    /// Get ERC-20 balance
    pub fn erc20_balance(&self, contract: H160, account: H160) -> Result<U256, String> {
        // ERC-20 balanceOf function selector: 0x70a08231
        let mut input = vec![0x70, 0xa0, 0x82, 0x31];
        
        // Encode account address (32 bytes)
        input.extend_from_slice(&[0u8; 12]); // Padding
        input.extend_from_slice(account.as_bytes());

        let result = self.static_call(contract, input)?;
        
        if result.len() == 32 {
            Ok(U256::from_big_endian(&result))
        } else {
            Err("Invalid balance response".to_string())
        }
    }

    /// Get ERC-20 allowance
    pub fn erc20_allowance(&self, contract: H160, owner: H160, spender: H160) -> Result<U256, String> {
        // ERC-20 allowance function selector: 0xdd62ed3e
        let mut input = vec![0xdd, 0x62, 0xed, 0x3e];
        
        // Encode owner address (32 bytes)
        input.extend_from_slice(&[0u8; 12]);
        input.extend_from_slice(owner.as_bytes());
        
        // Encode spender address (32 bytes)
        input.extend_from_slice(&[0u8; 12]);
        input.extend_from_slice(spender.as_bytes());

        let result = self.static_call(contract, input)?;
        
        if result.len() == 32 {
            Ok(U256::from_big_endian(&result))
        } else {
            Err("Invalid allowance response".to_string())
        }
    }

    /// Get ERC-20 token name
    pub fn erc20_name(&self, contract: H160) -> Result<String, String> {
        // ERC-20 name function selector: 0x06fdde03
        let input = vec![0x06, 0xfd, 0xde, 0x03];
        let result = self.static_call(contract, input)?;
        
        // Decode string from ABI encoding (simplified)
        if result.len() >= 64 {
            let length = U256::from_big_endian(&result[32..64]).as_usize();
            if result.len() >= 64 + length {
                let name_bytes = &result[64..64 + length];
                return Ok(String::from_utf8_lossy(name_bytes).to_string());
            }
        }
        
        Err("Invalid name response".to_string())
    }

    /// Get ERC-20 token symbol
    pub fn erc20_symbol(&self, contract: H160) -> Result<String, String> {
        // ERC-20 symbol function selector: 0x95d89b41
        let input = vec![0x95, 0xd8, 0x9b, 0x41];
        let result = self.static_call(contract, input)?;
        
        // Decode string from ABI encoding (simplified)
        if result.len() >= 64 {
            let length = U256::from_big_endian(&result[32..64]).as_usize();
            if result.len() >= 64 + length {
                let symbol_bytes = &result[64..64 + length];
                return Ok(String::from_utf8_lossy(symbol_bytes).to_string());
            }
        }
        
        Err("Invalid symbol response".to_string())
    }

    /// Get ERC-20 token decimals
    pub fn erc20_decimals(&self, contract: H160) -> Result<u8, String> {
        // ERC-20 decimals function selector: 0x313ce567
        let input = vec![0x31, 0x3c, 0xe5, 0x67];
        let result = self.static_call(contract, input)?;
        
        if result.len() == 32 {
            Ok(result[31])
        } else {
            Err("Invalid decimals response".to_string())
        }
    }

    /// Get ERC-20 total supply
    pub fn erc20_total_supply(&self, contract: H160) -> Result<U256, String> {
        // ERC-20 totalSupply function selector: 0x18160ddd
        let input = vec![0x18, 0x16, 0x0d, 0xdd];
        let result = self.static_call(contract, input)?;
        
        if result.len() == 32 {
            Ok(U256::from_big_endian(&result))
        } else {
            Err("Invalid total supply response".to_string())
        }
    }

    /// Create contract
    fn create_contract(
        &mut self,
        caller: H160,
        code: Vec<u8>,
        value: U256,
    ) -> Result<ExitReason, String> {
        let backend = self.create_backend();
        let metadata = StackSubstateMetadata::new(1_000_000, &self.config);
        let state = MemoryStackState::new(metadata, &backend);
        let precompiles = BTreeMap::new(); // No precompiles for now
        
        let mut executor = StackState::new(state, &self.config, &precompiles);

        let (exit_reason, _) = executor.transact_create(
            caller,
            value,
            code,
            1_000_000, // Gas limit
            Vec::new(), // Access list
        );

        // Commit changes back to storage (simplified)
        self.commit_backend(backend);
        
        // Increment nonce
        let nonce = self.get_nonce(&caller);
        self.set_nonce(caller, nonce + U256::one());
        
        Ok(exit_reason)
    }

    /// Call contract
    fn call_contract(
        &mut self,
        caller: H160,
        contract: H160,
        input: Vec<u8>,
        value: U256,
    ) -> Result<Vec<u8>, String> {
        let backend = self.create_backend();
        let metadata = StackSubstateMetadata::new(1_000_000, &self.config);
        let state = MemoryStackState::new(metadata, &backend);
        let precompiles = BTreeMap::new();
        
        let mut executor = StackState::new(state, &self.config, &precompiles);

        let (exit_reason, output) = executor.transact_call(
            caller,
            contract,
            value,
            input,
            1_000_000, // Gas limit
            Vec::new(), // Access list
        );

        self.commit_backend(backend);

        match exit_reason {
            ExitReason::Succeed(_) => Ok(output),
            ExitReason::Revert(_) => Err("Contract call reverted".to_string()),
            ExitReason::Error(err) => Err(format!("Contract call error: {:?}", err)),
            ExitReason::Fatal(err) => Err(format!("Fatal error during call: {:?}", err)),
        }
    }

    /// Static call (read-only)
    fn static_call(&self, contract: H160, input: Vec<u8>) -> Result<Vec<u8>, String> {
        let backend = self.create_backend();
        let metadata = StackSubstateMetadata::new(1_000_000, &self.config);
        let state = MemoryStackState::new(metadata, &backend);
        let precompiles = BTreeMap::new();
        
        let executor = StackState::new(state, &self.config, &precompiles);

        // For static calls, we would use a read-only version
        // This is simplified - in practice you'd use staticcall opcode
        Ok(vec![0u8; 32]) // Simplified placeholder
    }

    /// Generate ERC-20 bytecode (simplified)
    fn generate_erc20_bytecode(
        &self,
        name: &str,
        symbol: &str,
        decimals: u8,
        total_supply: U256,
    ) -> Vec<u8> {
        // This is a simplified ERC-20 bytecode template
        // In practice, you'd use a proper compiler like solc
        let mut bytecode = vec![
            // Constructor and basic contract setup
            0x60, 0x80, 0x60, 0x40, 0x52, 0x34, 0x80, 0x15,
            // Store name, symbol, decimals, totalSupply
        ];
        
        // Encode parameters into bytecode (simplified)
        bytecode.extend_from_slice(&[decimals]);
        
        let mut supply_bytes = [0u8; 32];
        total_supply.to_big_endian(&mut supply_bytes);
        bytecode.extend_from_slice(&supply_bytes);
        
        // Add name and symbol (simplified encoding)
        bytecode.extend_from_slice(name.as_bytes());
        bytecode.extend_from_slice(symbol.as_bytes());
        
        bytecode
    }

    /// Create EVM backend
    fn create_backend(&self) -> EVMBackend {
        EVMBackend::new(&self.accounts, &self.storage, &self.block_context)
    }

    /// Commit backend changes (simplified)
    fn commit_backend(&mut self, backend: EVMBackend) {
        // Apply state changes back to QoraNet storage
        self.accounts = backend.accounts;
        self.storage = backend.storage;
    }

    /// Generate contract address using CREATE opcode rules
    fn create_address(&self, deployer: &H160, nonce: U256) -> H160 {
        use sha3::{Digest, Keccak256};
        use rlp::RlpStream;
        
        let mut stream = RlpStream::new_list(2);
        stream.append(deployer);
        stream.append(&nonce);
        
        let hash = Keccak256::digest(&stream.out());
        H160::from_slice(&hash[12..])
    }

    /// Generate contract address using CREATE2 opcode rules
    fn create2_address(&self, deployer: &H160, salt: H256, code_hash: H256) -> H160 {
        use sha3::{Digest, Keccak256};
        
        let mut data = Vec::new();
        data.push(0xff);
        data.extend_from_slice(deployer.as_bytes());
        data.extend_from_slice(salt.as_bytes());
        data.extend_from_slice(code_hash.as_bytes());
        
        let hash = Keccak256::digest(&data);
        H160::from_slice(&hash[12..])
    }

    /// Get account nonce
    fn get_nonce(&self, address: &H160) -> U256 {
        self.accounts
            .get(address)
            .map(|account| account.nonce)
            .unwrap_or(U256::zero())
    }

    /// Set account nonce
    fn set_nonce(&mut self, address: H160, nonce: U256) {
        let account = self.accounts.entry(address).or_insert_with(|| Account {
            balance: U256::zero(),
            nonce: U256::zero(),
            code: Vec::new(),
        });
        account.nonce = nonce;
    }

    /// Get account balance
    pub fn get_balance(&self, address: H160) -> U256 {
        self.accounts
            .get(&address)
            .map(|account| account.balance)
            .unwrap_or(U256::zero())
    }

    /// Set account balance
    pub fn set_balance(&mut self, address: H160, balance: U256) {
        let account = self.accounts.entry(address).or_insert_with(|| Account {
            balance: U256::zero(),
            nonce: U256::zero(),
            code: Vec::new(),
        });
        account.balance = balance;
    }

    /// Update block context
    pub fn update_block_context(&mut self, number: U256, timestamp: U256) {
        self.block_context.number = number;
        self.block_context.timestamp = timestamp;
    }

    /// Get block number
    pub fn block_number(&self) -> U256 {
        self.block_context.number
    }

    /// Get chain ID
    pub fn chain_id(&self) -> U256 {
        self.block_context.chain_id
    }

    /// Estimate gas for ERC-20 operations
    pub fn estimate_gas(&self, operation: EVMOperation) -> u64 {
        match operation {
            EVMOperation::Deploy => 500_000,
            EVMOperation::Transfer => 50_000,
            EVMOperation::Approve => 45_000,
            EVMOperation::TransferFrom => 55_000,
            EVMOperation::BalanceOf => 25_000,
            EVMOperation::Allowance => 25_000,
        }
    }
}

/// EVM Backend for QoraNet integration
pub struct EVMBackend {
    accounts: BTreeMap<H160, Account>,
    storage: BTreeMap<(H160, H256), H256>,
    block_context: BlockContext,
}

impl EVMBackend {
    pub fn new(
        accounts: &BTreeMap<H160, Account>,
        storage: &BTreeMap<(H160, H256), H256>,
        block_context: &BlockContext,
    ) -> Self {
        Self {
            accounts: accounts.clone(),
            storage: storage.clone(),
            block_context: block_context.clone(),
        }
    }
}

// Implement EVM Handler traits for backend
impl evm::backend::Backend for EVMBackend {
    fn gas_price(&self) -> U256 {
        // Convert QOR gas price to wei equivalent
        U256::from(20_000_000_000u64) // 20 gwei equivalent
    }

    fn origin(&self) -> H160 {
        H160::zero() // Transaction origin
    }

    fn block_hash(&self, _number: U256) -> H256 {
        H256::zero() // Get from QoraNet block storage
    }

    fn block_number(&self) -> U256 {
        self.block_context.number
    }

    fn block_coinbase(&self) -> H160 {
        self.block_context.coinbase
    }

    fn block_timestamp(&self) -> U256 {
        self.block_context.timestamp
    }

    fn block_difficulty(&self) -> U256 {
        self.block_context.difficulty
    }

    fn block_gas_limit(&self) -> U256 {
        self.block_context.gas_limit
    }

    fn chain_id(&self) -> U256 {
        self.block_context.chain_id
    }

    fn exists(&self, address: H160) -> bool {
        self.accounts.contains_key(&address)
    }

    fn basic(&self, address: H160) -> evm::backend::Basic {
        if let Some(account) = self.accounts.get(&address) {
            evm::backend::Basic {
                balance: account.balance,
                nonce: account.nonce,
            }
        } else {
            evm::backend::Basic::default()
        }
    }

    fn code(&self, address: H160) -> Vec<u8> {
        self.accounts
            .get(&address)
            .map(|account| account.code.clone())
            .unwrap_or_default()
    }

    fn storage(&self, address: H160, index: H256) -> H256 {
        self.storage
            .get(&(address, index))
            .copied()
            .unwrap_or_default()
    }

    fn original_storage(&self, address: H160, index: H256) -> Option<H256> {
        self.storage.get(&(address, index)).copied()
    }
}

/// QoraNet transaction with EVM compatibility
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EVMTransaction {
    pub from: H160,
    pub to: Option<H160>, // None for contract creation
    pub value: U256,
    pub gas_limit: U256,
    pub gas_price: U256,
    pub data: Vec<u8>,
    pub nonce: U256,
    pub transaction_type: EVMTransactionType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EVMTransactionType {
    Legacy,
    EIP2930, // Access list transaction
    EIP1559, // Fee market transaction
}

#[derive(Debug, Clone, Copy)]
pub enum EVMOperation {
    Deploy,
    Transfer,
    Approve,
    TransferFrom,
    BalanceOf,
    Allowance,
}

impl EVMTransaction {
    /// Create ERC-20 deployment transaction
    pub fn deploy_erc20(
        from: H160,
        name: String,
        symbol: String,
        decimals: u8,
        total_supply: U256,
        gas_limit: U256,
        gas_price: U256,
        nonce: U256,
    ) -> Self {
        let data = Self::encode_erc20_constructor(name, symbol, decimals, total_supply);
        
        Self {
            from,
            to: None, // Contract creation
            value: U256::zero(),
            gas_limit,
            gas_price,
            data,
            nonce,
            transaction_type: EVMTransactionType::Legacy,
        }
    }

    /// Create ERC-20 transfer transaction
    pub fn erc20_transfer(
        from: H160,
        contract: H160,
        to: H160,
        amount: U256,
        gas_limit: U256,
        gas_price: U256,
        nonce: U256,
    ) -> Self {
        let mut data = vec![0xa9, 0x05, 0x9c, 0xbb]; // transfer(address,uint256)
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(to.as_bytes());
        
        let mut amount_bytes = [0u8; 32];
        amount.to_big_endian(&mut amount_bytes);
        data.extend_from_slice(&amount_bytes);

        Self {
            from,
            to: Some(contract),
            value: U256::zero(),
            gas_limit,
            gas_price,
            data,
            nonce,
            transaction_type: EVMTransactionType::Legacy,
        }
    }

    /// Create ERC-20 approve transaction
    pub fn erc20_approve(
        from: H160,
        contract: H160,
        spender: H160,
        amount: U256,
        gas_limit: U256,
        gas_price: U256,
        nonce: U256,
    ) -> Self {
        let mut data = vec![0x09, 0x5e, 0xa7, 0xb3]; // approve(address,uint256)
        data.extend_from_slice(&[0u8; 12]);
        data.extend_from_slice(spender.as_bytes());
        
        let mut amount_bytes = [0u8; 32];
        amount.to_big_endian(&mut amount_bytes);
        data.extend_from_slice(&amount_bytes);

        Self {
            from,
            to: Some(contract),
            value: U256::zero(),
            gas_limit,
            gas_price,
            data,
            nonce,
            transaction_type: EVMTransactionType::Legacy,
        }
    }

    fn encode_erc20_constructor(
        name: String,
        symbol: String,
        decimals: u8,
        total_supply: U256,
    ) -> Vec<u8> {
        // This would encode the constructor parameters for ERC-20
        // Simplified implementation
        let mut data = Vec::new();
        
        // Add constructor parameters encoding
        data.push(decimals);
        
        let mut supply_bytes = [0u8; 32];
        total_supply.to_big_endian(&mut supply_bytes);
        data.extend_from_slice(&supply_bytes);
        
        // Encode name and symbol (simplified)
        data.extend_from_slice(name.as_bytes());
        data.extend_from_slice(symbol.as_bytes());
        
        data
    }

    /// Get transaction hash
    pub fn hash(&self) -> H256 {
        use sha3::{Digest, Keccak256};
        use rlp::RlpStream;
        
        let mut stream = RlpStream::new_list(9);
        stream.append(&self.nonce);
        stream.append(&self.gas_price);
        stream.append(&self.gas_limit);
        
        if let Some(to) = self.to {
            stream.append(&to);
        } else {
            stream.append(&"");
        }
        
        stream.append(&self.value);
        stream.append(&self.data);
        stream.append(&self.from); // Simplified - normally would use v,r,s signature
        
        let hash = Keccak256::digest(&stream.out());
        H256::from_slice(&hash)
    }
}

impl Default for QoraNetEVM {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evm_creation() {
        let evm = QoraNetEVM::new();
        assert_eq!(evm.chain_id(), U256::from(2024));
        assert_eq!(evm.block_number(), U256::zero());
    }

    #[test]
    fn test_contract_address_generation() {
        let evm = QoraNetEVM::new();
        let deployer = H160::from_low_u64_be(1);
        let nonce = U256::zero();
        
        let address1 = evm.create_address(&deployer, nonce);
        let address2 = evm.create_address(&deployer, nonce + U256::one());
        
        // Different nonces should generate different addresses
        assert_ne!(address1, address2);
    }

    #[test]
    fn test_create2_address_generation() {
        let evm = QoraNetEVM::new();
        let deployer = H160::from_low_u64_be(1);
        let salt = H256::random();
        let code_hash = H256::random();
        
        let address1 = evm.create2_address(&deployer, salt, code_hash);
        let address2 = evm.create2_address(&deployer, H256::random(), code_hash);
        
        // Different salts should generate different addresses
        assert_ne!(address1, address2);
    }

    #[test]
    fn test_balance_operations() {
        let mut evm = QoraNetEVM::new();
        let address = H160::from_low_u64_be(1);
        let balance = U256::from(1000);
        
        assert_eq!(evm.get_balance(address), U256::zero());
        
        evm.set_balance(address, balance);
        assert_eq!(evm.get_balance(address), balance);
    }

    #[test]
    fn test_nonce_operations() {
        let mut evm = QoraNetEVM::new();
        let address = H160::from_low_u64_be(1);
        
        assert_eq!(evm.get_nonce(&address), U256::zero());
        
        evm.set_nonce(address, U256::from(5));
        assert_eq!(evm.get_nonce(&address), U256::from(5));
    }

    #[test]
    fn test_gas_estimation() {
        let evm = QoraNetEVM::new();
        
        assert_eq!(evm.estimate_gas(EVMOperation::Deploy), 500_000);
        assert_eq!(evm.estimate_gas(EVMOperation::Transfer), 50_000);
        assert_eq!(evm.estimate_gas(EVMOperation::Approve), 45_000);
    }

    #[test]
    fn test_evm_transaction_creation() {
        let from = H160::from_low_u64_be(1);
        let to = H160::from_low_u64_be(2);
        
        let tx = EVMTransaction::erc20_transfer(
            from,
            H160::from_low_u64_be(100), // contract
            to,
            U256::from(1000),
            U256::from(50000),
            U256::from(20_000_000_000u64),
            U256::zero(),
        );
        
        assert_eq!(tx.from, from);
        assert_eq!(tx.to, Some(H160::from_low_u64_be(100)));
        assert_eq!(tx.value, U256::zero());
        
        // Check function selector (first 4 bytes)
        assert_eq!(&tx.data[0..4], &[0xa9, 0x05, 0x9c, 0xbb]);
    }

    #[test]
    fn test_block_context_updates() {
        let mut evm = QoraNetEVM::new();
        let new_block = U256::from(100);
        let new_timestamp = U256::from(1640000000);
        
        evm.update_block_context(new_block, new_timestamp);
        
        assert_eq!(evm.block_number(), new_block);
        assert_eq!(evm.block_context.timestamp, new_timestamp);
    }
}
