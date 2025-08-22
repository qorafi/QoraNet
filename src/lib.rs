pub mod consensus;
pub mod validator;
pub mod network;
pub mod transaction;
pub mod storage;
pub mod rpc;
pub mod app_monitor;
pub mod rewards;
pub mod fee_oracle;

use ed25519_dalek::{Keypair, PublicKey, Signature};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub use fee_oracle::*;

/// QoraNet version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Native token symbol
pub const NATIVE_TOKEN: &str = "QOR";

/// Fee constants (in USD)
pub const MIN_FEE_USD: f64 = 0.0001;  // $0.0001 minimum fee
pub const MAX_FEE_USD: f64 = 0.01;    // $0.01 maximum fee
pub const DEFAULT_FEE_USD: f64 = 0.0001; // Default fee for simple transactions

/// Convert USD to QOR tokens using current price
pub fn usd_to_qor(usd_amount: f64, qor_price_usd: f64) -> u64 {
    if qor_price_usd <= 0.0 {
        return 0;
    }
    
    let qor_amount = usd_amount / qor_price_usd;
    // Convert to smallest unit (assuming 9 decimals like SOL)
    (qor_amount * 1_000_000_000.0) as u64
}

/// Convert QOR tokens to USD using current price
pub fn qor_to_usd(qor_amount: u64, qor_price_usd: f64) -> f64 {
    let qor_float = qor_amount as f64 / 1_000_000_000.0;
    qor_float * qor_price_usd
}

/// Convert USD to any token using current price and decimals
pub fn usd_to_token(usd_amount: f64, token_price_usd: f64, decimals: u8) -> u64 {
    if token_price_usd <= 0.0 {
        return 0;
    }
    
    let token_amount = usd_amount / token_price_usd;
    let decimal_multiplier = 10_u64.pow(decimals as u32);
    (token_amount * decimal_multiplier as f64) as u64
}

/// Convert token amount to USD using current price and decimals
pub fn token_to_usd(token_amount: u64, token_price_usd: f64, decimals: u8) -> f64 {
    let decimal_multiplier = 10_u64.pow(decimals as u32);
    let token_float = token_amount as f64 / decimal_multiplier as f64;
    token_float * token_price_usd
}

/// QoraNet errors
#[derive(thiserror::Error, Debug)]
pub enum QoraNetError {
    #[error("Invalid transaction: {0}")]
    InvalidTransaction(String),
    
    #[error("Insufficient liquidity: required {required}, have {available}")]
    InsufficientLiquidity { required: u64, available: u64 },
    
    #[error("App monitoring error: {0}")]
    AppMonitorError(String),
    
    #[error("Network error: {0}")]
    NetworkError(String),
    
    #[error("Storage error: {0}")]
    StorageError(String),
    
    #[error("Consensus error: {0}")]
    ConsensusError(String),
    
    #[error("Token error: {0}")]
    TokenError(String),
    
    #[error("Bridge error: {0}")]
    BridgeError(String),
}

/// QoraNet result type
pub type Result<T> = std::result::Result<T, QoraNetError>;

/// Public key type
pub type QoraPublicKey = PublicKey;

/// Signature type  
pub type QoraSignature = Signature;

/// Hash type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    pub fn new(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        Hash(hasher.finalize().into())
    }
    
    pub fn zero() -> Self {
        Hash([0u8; 32])
    }
    
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl std::fmt::Display for Hash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

/// Account address
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address(pub [u8; 32]);

impl Address {
    pub fn from_pubkey(pubkey: &QoraPublicKey) -> Self {
        Address(pubkey.to_bytes())
    }
    
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    
    /// Native QOR token address (special case)
    pub fn native_qor() -> Self {
        Address([0u8; 32]) // QOR uses zero address
    }
    
    /// Check if this is the native QOR address
    pub fn is_native_qor(&self) -> bool {
        self.0 == [0u8; 32]
    }
    
    /// Create address from hex string (for ERC-20 tokens)
    pub fn from_hex(hex_str: &str) -> Result<Self> {
        let hex_clean = hex_str.strip_prefix("0x").unwrap_or(hex_str);
        if hex_clean.len() != 64 {
            return Err(QoraNetError::TokenError("Invalid address length".to_string()));
        }
        
        let bytes = hex::decode(hex_clean)
            .map_err(|_| QoraNetError::TokenError("Invalid hex address".to_string()))?;
        
        let mut addr = [0u8; 32];
        addr.copy_from_slice(&bytes);
        Ok(Address(addr))
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_native_qor() {
            write!(f, "QOR")
        } else {
            write!(f, "{}", hex::encode(self.0))
        }
    }
}

/// Token types supported on QoraNet
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TokenType {
    Native,                    // QOR token
    ERC20(ERC20TokenInfo),    // Bridged ERC-20 tokens
}

/// ERC-20 token information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ERC20TokenInfo {
    pub ethereum_address: String,    // Original Ethereum contract address
    pub qoranet_address: Address,    // Wrapped token address on QoraNet
    pub name: String,                // Token name (e.g., "Tether USD")
    pub symbol: String,              // Token symbol (e.g., "USDT")
    pub decimals: u8,               // Token decimals (e.g., 6 for USDT)
    pub total_supply: u64,          // Total wrapped supply on QoraNet
    pub is_fee_token: bool,         // Can this token be used for fees?
}

impl ERC20TokenInfo {
    /// Convert token amount to human readable format
    pub fn format_amount(&self, amount: u64) -> String {
        let decimal_multiplier = 10_u64.pow(self.decimals as u32);
        let token_amount = amount as f64 / decimal_multiplier as f64;
        format!("{:.precision$} {}", token_amount, self.symbol, precision = self.decimals as usize)
    }
    
    /// Convert human readable amount to token units
    pub fn parse_amount(&self, amount_str: &str) -> Result<u64> {
        let amount: f64 = amount_str.parse()
            .map_err(|_| QoraNetError::TokenError("Invalid amount format".to_string()))?;
        
        let decimal_multiplier = 10_u64.pow(self.decimals as u32);
        Ok((amount * decimal_multiplier as f64) as u64)
    }
}

/// Multi-token balance supporting QOR + ERC-20s
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBalance {
    pub balances: HashMap<Address, u64>, // token_address -> amount
}

impl TokenBalance {
    pub fn new() -> Self {
        Self {
            balances: HashMap::new(),
        }
    }
    
    /// Get QOR balance (native token)
    pub fn get_qor_balance(&self) -> u64 {
        self.balances.get(&Address::native_qor()).copied().unwrap_or(0)
    }
    
    /// Get ERC-20 token balance
    pub fn get_token_balance(&self, token_address: &Address) -> u64 {
        self.balances.get(token_address).copied().unwrap_or(0)
    }
    
    /// Add tokens to balance
    pub fn add_tokens(&mut self, token_address: Address, amount: u64) -> Result<()> {
        let current = self.balances.get(&token_address).copied().unwrap_or(0);
        let new_balance = current.checked_add(amount)
            .ok_or_else(|| QoraNetError::InvalidTransaction("Token balance overflow".to_string()))?;
        self.balances.insert(token_address, new_balance);
        Ok(())
    }
    
    /// Subtract tokens from balance
    pub fn subtract_tokens(&mut self, token_address: Address, amount: u64) -> Result<()> {
        let current = self.balances.get(&token_address).copied().unwrap_or(0);
        let new_balance = current.checked_sub(amount)
            .ok_or_else(|| QoraNetError::InsufficientLiquidity { 
                required: amount, 
                available: current 
            })?;
        self.balances.insert(token_address, new_balance);
        Ok(())
    }
    
    /// Get all non-zero balances
    pub fn get_all_balances(&self) -> Vec<(Address, u64)> {
        self.balances.iter()
            .filter(|(_, &amount)| amount > 0)
            .map(|(addr, &amount)| (addr.clone(), amount))
            .collect()
    }
    
    /// Convert to QOR-compatible balance for legacy support
    pub fn to_qor_balance(&self) -> Balance {
        Balance::new(self.get_qor_balance())
    }
}

impl Default for TokenBalance {
    fn default() -> Self {
        Self::new()
    }
}

/// Legacy QOR-only balance (kept for backward compatibility)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub amount: u64, // Amount in smallest unit (1 QOR = 1_000_000_000 units)
}

impl Balance {
    pub fn new(amount: u64) -> Self {
        Self { amount }
    }
    
    pub fn zero() -> Self {
        Self { amount: 0 }
    }
    
    pub fn from_qor(qor: f64) -> Self {
        Self {
            amount: (qor * 1_000_000_000.0) as u64,
        }
    }
    
    pub fn to_qor(&self) -> f64 {
        self.amount as f64 / 1_000_000_000.0
    }
    
    pub fn add(&mut self, other: u64) -> Result<()> {
        self.amount = self.amount.checked_add(other)
            .ok_or_else(|| QoraNetError::InvalidTransaction("Balance overflow".to_string()))?;
        Ok(())
    }
    
    pub fn subtract(&mut self, other: u64) -> Result<()> {
        self.amount = self.amount.checked_sub(other)
            .ok_or_else(|| QoraNetError::InsufficientLiquidity { 
                required: other, 
                available: self.amount 
            })?;
        Ok(())
    }
    
    /// Convert to multi-token balance
    pub fn to_token_balance(&self) -> TokenBalance {
        let mut token_balance = TokenBalance::new();
        if self.amount > 0 {
            token_balance.balances.insert(Address::native_qor(), self.amount);
        }
        token_balance
    }
}

impl std::fmt::Display for Balance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.9} QOR", self.to_qor())
    }
}

/// Enhanced LP Token supporting multi-token pools
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LPToken {
    pub pool_address: Address,
    pub amount: u64,
    pub token_a: Address,
    pub token_b: Address,
    pub pool_type: PoolType,
}

/// Pool types for different token combinations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PoolType {
    QorErc20,      // QOR paired with ERC-20 token
    Erc20Erc20,    // ERC-20 paired with ERC-20 token
    Native,        // QOR only pools
}

impl LPToken {
    /// Check if this LP token involves QOR
    pub fn has_qor(&self) -> bool {
        self.token_a.is_native_qor() || self.token_b.is_native_qor()
    }
    
    /// Get the non-QOR token address (if any)
    pub fn get_paired_token(&self) -> Option<Address> {
        if self.token_a.is_native_qor() {
            Some(self.token_b.clone())
        } else if self.token_b.is_native_qor() {
            Some(self.token_a.clone())
        } else {
            None // Both are ERC-20 tokens
        }
    }
}

/// Fee payment options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeePayment {
    QOR(u64),                    // Pay with QOR tokens
    ERC20 { 
        token: Address, 
        amount: u64 
    },                           // Pay with ERC-20 token
}

impl FeePayment {
    /// Calculate fee in specified token
    pub fn calculate_fee(fee_usd: f64, token: &Address, token_registry: &TokenRegistry, oracle: &FeeOracle) -> Result<Self> {
        if token.is_native_qor() {
            let qor_price = oracle.get_qor_price()?;
            let fee_amount = usd_to_qor(fee_usd, qor_price);
            Ok(FeePayment::QOR(fee_amount))
        } else {
            let token_info = token_registry.get_token_info(token)
                .ok_or_else(|| QoraNetError::TokenError("Token not found".to_string()))?;
            
            if !token_info.is_fee_token {
                return Err(QoraNetError::TokenError("Token cannot be used for fees".to_string()));
            }
            
            let token_price = oracle.get_token_price(&token_info.symbol)?;
            let fee_amount = usd_to_token(fee_usd, token_price, token_info.decimals);
            
            Ok(FeePayment::ERC20 { 
                token: token.clone(), 
                amount: fee_amount 
            })
        }
    }
}

/// Token registry to manage supported ERC-20 tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenRegistry {
    tokens: HashMap<Address, ERC20TokenInfo>,
    ethereum_to_qora: HashMap<String, Address>, // eth_address -> qora_address
}

impl TokenRegistry {
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
            ethereum_to_qora: HashMap::new(),
        }
    }
    
    /// Register a new ERC-20 token
    pub fn register_erc20(&mut self, token_info: ERC20TokenInfo) -> Result<()> {
        // Check if already registered
        if self.ethereum_to_qora.contains_key(&token_info.ethereum_address) {
            return Err(QoraNetError::InvalidTransaction("Token already registered".to_string()));
        }
        
        let qora_address = token_info.qoranet_address.clone();
        self.ethereum_to_qora.insert(token_info.ethereum_address.clone(), qora_address.clone());
        self.tokens.insert(qora_address, token_info);
        
        Ok(())
    }
    
    /// Get token info by QoraNet address
    pub fn get_token_info(&self, address: &Address) -> Option<&ERC20TokenInfo> {
        self.tokens.get(address)
    }
    
    /// Get QoraNet address from Ethereum address
    pub fn get_qora_address(&self, eth_address: &str) -> Option<&Address> {
        self.ethereum_to_qora.get(eth_address)
    }
    
    /// Get all fee-enabled tokens
    pub fn get_fee_tokens(&self) -> Vec<&ERC20TokenInfo> {
        self.tokens.values()
            .filter(|token| token.is_fee_token)
            .collect()
    }
    
    /// Get all registered tokens
    pub fn get_all_tokens(&self) -> Vec<&ERC20TokenInfo> {
        self.tokens.values().collect()
    }
}

impl Default for TokenRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Application performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppMetrics {
    pub cpu_usage: f64,        // CPU percentage
    pub memory_usage: u64,     // Memory in bytes
    pub uptime: u64,          // Uptime in seconds
    pub requests_served: u64,  // Number of requests served
    pub last_updated: u64,    // Timestamp
}

impl AppMetrics {
    pub fn new() -> Self {
        Self {
            cpu_usage: 0.0,
            memory_usage: 0,
            uptime: 0,
            requests_served: 0,
            last_updated: chrono::Utc::now().timestamp() as u64,
        }
    }
    
    /// Calculate performance score for rewards
    pub fn performance_score(&self) -> f64 {
        let cpu_score = (self.cpu_usage / 100.0).min(1.0);
        let uptime_hours = self.uptime as f64 / 3600.0;
        let uptime_score = (uptime_hours / 24.0).min(1.0); // Max score at 24h uptime
        let request_score = (self.requests_served as f64 / 1000.0).min(1.0);
        
        (cpu_score * 0.4 + uptime_score * 0.3 + request_score * 0.3)
    }
}

impl Default for AppMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Block height type
pub type BlockHeight = u64;

/// Timestamp type  
pub type Timestamp = u64;
