//! QRC-20 Token Standard - ERC-20 compatible tokens on QoraNet
//! 
//! This module provides ERC-20 compatible token functionality for QoraNet,
//! including token deployment, transfers, and bridging capabilities.

pub mod token;
pub mod registry;
pub mod bridge;
pub mod evm_integration;
pub mod rpc;

pub use token::{QRC20Token, QRC20Transaction, QRC20TokenInfo};
pub use registry::QRC20Registry;
pub use bridge::ERC20Bridge;
pub use evm_integration::{QoraNetEVM, EVMTransaction};

use primitive_types::{H160, U256};
use serde::{Deserialize, Serialize};

/// QRC-20 error types
#[derive(Debug, thiserror::Error)]
pub enum QRC20Error {
    #[error("Token not found")]
    TokenNotFound,
    
    #[error("Insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: U256, available: U256 },
    
    #[error("Insufficient allowance: required {required}, available {available}")]
    InsufficientAllowance { required: U256, available: U256 },
    
    #[error("Token is paused")]
    TokenPaused,
    
    #[error("Only owner can perform this action")]
    OnlyOwner,
    
    #[error("Symbol already exists: {symbol}")]
    SymbolExists { symbol: String },
    
    #[error("Invalid address: {address}")]
    InvalidAddress { address: String },
    
    #[error("EVM execution failed: {reason}")]
    EVMExecutionFailed { reason: String },
}

/// Result type for QRC-20 operations
pub type QRC20Result<T> = Result<T, QRC20Error>;

/// QRC-20 event types for logging and subscriptions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QRC20Event {
    /// Token deployed
    Deploy {
        contract: H160,
        deployer: H160,
        name: String,
        symbol: String,
        total_supply: U256,
    },
    
    /// Token transfer
    Transfer {
        contract: H160,
        from: H160,
        to: H160,
        amount: U256,
    },
    
    /// Approval granted
    Approval {
        contract: H160,
        owner: H160,
        spender: H160,
        amount: U256,
    },
    
    /// Tokens minted
    Mint {
        contract: H160,
        to: H160,
        amount: U256,
    },
    
    /// Tokens burned
    Burn {
        contract: H160,
        from: H160,
        amount: U256,
    },
    
    /// Token paused/unpaused
    PauseStatusChanged {
        contract: H160,
        paused: bool,
    },
    
    /// Ownership transferred
    OwnershipTransferred {
        contract: H160,
        old_owner: H160,
        new_owner: H160,
    },
}
