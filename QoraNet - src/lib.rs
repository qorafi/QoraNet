pub mod consensus;
pub mod validator;
pub mod network;
pub mod transaction;
pub mod storage;
pub mod rpc;
pub mod app_monitor;
pub mod rewards;

use ed25519_dalek::{Keypair, PublicKey, Signature};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

/// QOR token balance
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
}

impl std::fmt::Display for Balance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.9} QOR", self.to_qor())
    }
}

/// LP Token representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LPToken {
    pub pool_address: Address,
    pub amount: u64,
    pub token_a: Address,
    pub token_b: Address,
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

/// Block height type
pub type BlockHeight = u64;

/// Timestamp type  
pub type Timestamp = u64;
