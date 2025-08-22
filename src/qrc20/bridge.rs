use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use primitive_types::{H160, H256, U256};
use super::{QRC20Registry, QRC20Error, QRC20Result, QRC20Event};

/// Bridge for ERC-20 to QRC-20 conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ERC20Bridge {
    /// Ethereum to QoraNet token mapping
    pub eth_to_qora_mapping: HashMap<H160, H160>,
    
    /// QoraNet to Ethereum token mapping  
    pub qora_to_eth_mapping: HashMap<H160, H160>,
    
    /// Locked tokens on Ethereum side
    pub locked_eth_tokens: HashMap<H160, U256>,
    
    /// Minted tokens on QoraNet side
    pub minted_qora_tokens: HashMap<H160, U256>,
    
    /// Bridge transactions for tracking
    pub bridge_transactions: HashMap<H256, BridgeTransaction>,
    
    /// Bridge operators (can process bridge requests)
    pub bridge_operators: Vec<H160>,
    
    /// Minimum confirmations required
    pub min_confirmations: u64,
    
    /// Bridge fee percentage (basis points, e.g., 100 = 1%)
    pub bridge_fee_bp: u16,
    
    /// Bridge treasury address
    pub bridge_treasury: H160,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeTransaction {
    pub id: H256,
    pub eth_tx_hash: Option<H256>,
    pub qora_tx_hash: Option<H256>,
    pub user: H160,
    pub eth_token: H160,
    pub qora_token: H160,
    pub amount: U256,
    pub direction: BridgeDirection,
    pub status: BridgeStatus,
    pub confirmations: u64,
    pub timestamp: u64,
    pub fee_paid: U256,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BridgeDirection {
    EthereumToQoraNet,
    QoraNetToEthereum,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BridgeStatus {
    Pending,
    Confirmed,
    Completed,
    Failed,
    Cancelled,
}

impl ERC20Bridge {
    pub fn new() -> Self {
        Self {
            eth_to_qora_mapping: HashMap::new(),
            qora_to_eth_mapping: HashMap::new(),
            locked_eth_tokens: HashMap::new(),
            minted_qora_tokens: HashMap::new(),
            bridge_transactions: HashMap::new(),
            bridge_operators: Vec::new(),
            min_confirmations: 12, // Ethereum blocks
            bridge_fee_bp: 50, // 0.5% bridge fee
            bridge_treasury: H160::zero(),
        }
    }

    pub fn new_with_config(
        operators: Vec<H160>,
        min_confirmations: u64,
        bridge_fee_bp: u16,
        treasury: H160,
    ) -> Self {
        Self {
            eth_to_qora_mapping: HashMap::new(),
            qora_to_eth_mapping: HashMap::new(),
            locked_eth_tokens: HashMap::new(),
            minted_qora_tokens: HashMap::new(),
            bridge_transactions: HashMap::new(),
            bridge_operators: operators,
            min_confirmations,
            bridge_fee_bp,
            bridge_treasury: treasury,
        }
    }

    /// Bridge ERC-20 token from Ethereum to QoraNet
    pub fn bridge_from_ethereum(
        &mut self,
        registry: &mut QRC20Registry,
        eth_token: H160,
        user: H160,
        amount: U256,
        token_name: String,
        token_symbol: String,
        decimals: u8,
        eth_tx_hash: H256,
        confirmations: u64,
    ) -> QRC20Result<H160> {
        // Calculate bridge fee
        let fee = self.calculate_bridge_fee(amount);
        let net_amount = amount.saturating_sub(fee);

        if net_amount.is_zero() {
            return Err(QRC20Error::EVMExecutionFailed { 
                reason: "Amount too small after fees".to_string() 
            });
        }

        let qora_token = if let Some(existing_token) = self.eth_to_qora_mapping.get(&eth_token) {
            // Token already bridged, mint tokens to user
            let token = registry.get_token_mut(*existing_token)
                .ok_or(QRC20Error::TokenNotFound)?;
            
            // Mint net amount (after fee)
            token.mint(token.owner, user, net_amount)?;
            *existing_token
        } else {
            // First time bridging, deploy new QRC-20
            let qora_token = registry.deploy_token(
                user, // User becomes initial owner, but should be bridge contract in production
                format!("Bridged {}", token_name),
                format!("b{}", token_symbol),
                decimals,
                U256::zero(), // Start with 0 supply
            )?;
            
            // Create mapping
            self.eth_to_qora_mapping.insert(eth_token, qora_token);
            self.qora_to_eth_mapping.insert(qora_token, eth_token);
            
            // Mint initial tokens to user
            let token = registry.get_token_mut(qora_token).unwrap();
            token.mint(token.owner, user, net_amount)?;
            
            tracing::info!(
                "Created bridge mapping: ETH token {:?} -> QRC-20 token {:?}",
                eth_token,
                qora_token
            );

            qora_token
        };

        // Update locked amounts
        let locked = self.locked_eth_tokens.get(&eth_token).unwrap_or(&U256::zero());
        self.locked_eth_tokens.insert(eth_token, locked + amount);

        // Update minted amounts
        let minted = self.minted_qora_tokens.get(&qora_token).unwrap_or(&U256::zero());
        self.minted_qora_tokens.insert(qora_token, minted + net_amount);

        // Create bridge transaction record
        let bridge_tx = BridgeTransaction {
            id: H256::random(),
            eth_tx_hash: Some(eth_tx_hash),
            qora_tx_hash: None,
            user,
            eth_token,
            qora_token,
            amount,
            direction: BridgeDirection::EthereumToQoraNet,
            status: if confirmations >= self.min_confirmations {
                BridgeStatus::Completed
            } else {
                BridgeStatus::Confirmed
            },
            confirmations,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            fee_paid: fee,
        };

        self.bridge_transactions.insert(bridge_tx.id, bridge_tx);

        tracing::info!(
            "Bridged {} {} from Ethereum to QoraNet (net: {} after fee: {})",
            amount, token_symbol, net_amount, fee
        );

        Ok(qora_token)
    }

    /// Bridge QRC-20 token back to Ethereum
    pub fn bridge_to_ethereum(
        &mut self,
        registry: &mut QRC20Registry,
        qora_token: H160,
        user: H160,
        amount: U256,
    ) -> QRC20Result<H160> {
        // Check if this is a bridged token
        let eth_token = *self.qora_to_eth_mapping.get(&qora_token)
            .ok_or(QRC20Error::EVMExecutionFailed { 
                reason: "Token is not bridged from Ethereum".to_string() 
            })?;

        // Calculate bridge fee
        let fee = self.calculate_bridge_fee(amount);
        let net_amount = amount.saturating_sub(fee);

        if net_amount.is_zero() {
            return Err(QRC20Error::EVMExecutionFailed { 
                reason: "Amount too small after fees".to_string() 
            });
        }

        // Check user has enough tokens
        let token = registry.get_token(qora_token)
            .ok_or(QRC20Error::TokenNotFound)?;
        
        if token.balance_of(user) < amount {
            return Err(QRC20Error::InsufficientBalance {
                required: amount,
                available: token.balance_of(user),
            });
        }

        // Burn QRC-20 tokens from user
        let token = registry.get_token_mut(qora_token).unwrap();
        token.burn(user, amount)?;

        // Update locked amounts (decrease as tokens are released on Ethereum)
        let locked = self.locked_eth_tokens.get(&eth_token).unwrap_or(&U256::zero());
        if *locked < net_amount {
            return Err(QRC20Error::EVMExecutionFailed { 
                reason: "Insufficient locked tokens".to_string() 
            });
        }
        self.locked_eth_tokens.insert(eth_token, locked - net_amount);

        // Update minted amounts (decrease as tokens are burned)
        let minted = self.minted_qora_tokens.get(&qora_token).unwrap_or(&U256::zero());
        self.minted_qora_tokens.insert(qora_token, minted.saturating_sub(amount));

        // Create bridge transaction record
        let bridge_tx = BridgeTransaction {
            id: H256::random(),
            eth_tx_hash: None, // Will be set when processed on Ethereum
            qora_tx_hash: Some(H256::random()), // Mock QoraNet tx hash
            user,
            eth_token,
            qora_token,
            amount,
            direction: BridgeDirection::QoraNetToEthereum,
            status: BridgeStatus::Pending, // Needs to be processed on Ethereum
            confirmations: 0,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            fee_paid: fee,
        };

        self.bridge_transactions.insert(bridge_tx.id, bridge_tx);

        tracing::info!(
            "Initiated bridge from QoraNet to Ethereum: {} tokens (net: {} after fee: {})",
            amount, net_amount, fee
        );

        Ok(eth_token)
    }

    /// Calculate bridge fee
    fn calculate_bridge_fee(&self, amount: U256) -> U256 {
        amount * U256::from(self.bridge_fee_bp) / U256::from(10000)
    }

    /// Add bridge operator
    pub fn add_operator(&mut self, caller: H160, operator: H160) -> QRC20Result<()> {
        if !self.is_operator(caller) && !self.bridge_treasury.is_zero() && caller != self.bridge_treasury {
            return Err(QRC20Error::OnlyOwner);
        }

        if !self.bridge_operators.contains(&operator) {
            self.bridge_operators.push(operator);
            tracing::info!("Added bridge operator: {:?}", operator);
        }

        Ok(())
    }

    /// Remove bridge operator
    pub fn remove_operator(&mut self, caller: H160, operator: H160) -> QRC20Result<()> {
        if !self.is_operator(caller) && !self.bridge_treasury.is_zero() && caller != self.bridge_treasury {
            return Err(QRC20Error::OnlyOwner);
        }

        self.bridge_operators.retain(|&op| op != operator);
        tracing::info!("Removed bridge operator: {:?}", operator);

        Ok(())
    }

    /// Check if address is a bridge operator
    pub fn is_operator(&self, address: H160) -> bool {
        self.bridge_operators.contains(&address)
    }

    /// Update bridge transaction status
    pub fn update_transaction_status(
        &mut self,
        caller: H160,
        tx_id: H256,
        status: BridgeStatus,
        eth_tx_hash: Option<H256>,
        confirmations: Option<u64>,
    ) -> QRC20Result<()> {
        if !self.is_operator(caller) {
            return Err(QRC20Error::OnlyOwner);
        }

        let bridge_tx = self.bridge_transactions.get_mut(&tx_id)
            .ok_or(QRC20Error::EVMExecutionFailed { 
                reason: "Bridge transaction not found".to_string() 
            })?;

        bridge_tx.status = status;
        
        if let Some(hash) = eth_tx_hash {
            bridge_tx.eth_tx_hash = Some(hash);
        }
        
        if let Some(conf) = confirmations {
            bridge_tx.confirmations = conf;
        }

        Ok(())
    }

    /// Get bridge transaction
    pub fn get_transaction(&self, tx_id: H256) -> Option<&BridgeTransaction> {
        self.bridge_transactions.get(&tx_id)
    }

    /// Get user's bridge transactions
    pub fn get_user_transactions(&self, user: H160) -> Vec<&BridgeTransaction> {
        self.bridge_transactions
            .values()
            .filter(|tx| tx.user == user)
            .collect()
    }

    /// Get pending bridge transactions
    pub fn get_pending_transactions(&self) -> Vec<&BridgeTransaction> {
        self.bridge_transactions
            .values()
            .filter(|tx| matches!(tx.status, BridgeStatus::Pending))
            .collect()
    }

    /// Get bridge statistics
    pub fn get_bridge_stats(&self) -> BridgeStats {
        let total_locked: U256 = self.locked_eth_tokens.values().sum();
        let total_minted: U256 = self.minted_qora_tokens.values().sum();
        
        let total_transactions = self.bridge_transactions.len();
        let completed_transactions = self.bridge_transactions
            .values()
            .filter(|tx| matches!(tx.status, BridgeStatus::Completed))
            .count();
        
        let pending_transactions = self.bridge_transactions
            .values()
            .filter(|tx| matches!(tx.status, BridgeStatus::Pending))
            .count();

        let failed_transactions = self.bridge_transactions
            .values()
            .filter(|tx| matches!(tx.status, BridgeStatus::Failed))
            .count();

        let total_volume: U256 = self.bridge_transactions
            .values()
            .map(|tx| tx.amount)
            .sum();

        let total_fees: U256 = self.bridge_transactions
            .values()
            .map(|tx| tx.fee_paid)
            .sum();

        BridgeStats {
            total_locked,
            total_minted,
            total_transactions,
            completed_transactions,
            pending_transactions,
            failed_transactions,
            total_volume,
            total_fees,
            unique_tokens: self.eth_to_qora_mapping.len(),
        }
    }

    /// Set bridge configuration
    pub fn set_config(
        &mut self,
        caller: H160,
        min_confirmations: Option<u64>,
        bridge_fee_bp: Option<u16>,
        treasury: Option<H160>,
    ) -> QRC20Result<()> {
        if !self.bridge_treasury.is_zero() && caller != self.bridge_treasury {
            return Err(QRC20Error::OnlyOwner);
        }

        if let Some(conf) = min_confirmations {
            self.min_confirmations = conf;
        }

        if let Some(fee) = bridge_fee_bp {
            if fee > 1000 { // Max 10% fee
                return Err(QRC20Error::EVMExecutionFailed { 
                    reason: "Bridge fee too high".to_string() 
                });
            }
            self.bridge_fee_bp = fee;
        }

        if let Some(treasury) = treasury {
            self.bridge_treasury = treasury;
        }

        Ok(())
    }

    /// Emergency pause bridge
    pub fn emergency_pause(&mut self, caller: H160) -> QRC20Result<()> {
        if !self.is_operator(caller) && caller != self.bridge_treasury {
            return Err(QRC20Error::OnlyOwner);
        }

        // Mark all pending transactions as cancelled
        for tx in self.bridge_transactions.values_mut() {
            if matches!(tx.status, BridgeStatus::Pending) {
                tx.status = BridgeStatus::Cancelled;
            }
        }

        tracing::warn!("Bridge emergency pause activated by {:?}", caller);
        Ok(())
    }

    /// Get token mapping
    pub fn get_eth_to_qora_mapping(&self) -> &HashMap<H160, H160> {
        &self.eth_to_qora_mapping
    }

    /// Get reverse token mapping
    pub fn get_qora_to_eth_mapping(&self) -> &HashMap<H160, H160> {
        &self.qora_to_eth_mapping
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeStats {
    pub total_locked: U256,
    pub total_minted: U256,
    pub total_transactions: usize,
    pub completed_transactions: usize,
    pub pending_transactions: usize,
    pub failed_transactions: usize,
    pub total_volume: U256,
    pub total_fees: U256,
    pub unique_tokens: usize,
}

impl Default for ERC20Bridge {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bridge_creation() {
        let bridge = ERC20Bridge::new();
        assert_eq!(bridge.min_confirmations, 12);
        assert_eq!(bridge.bridge_fee_bp, 50);
        assert!(bridge.bridge_operators.is_empty());
    }

    #[test]
    fn test_bridge_fee_calculation() {
        let bridge = ERC20Bridge::new();
        let amount = U256::from(1000);
        let fee = bridge.calculate_bridge_fee(amount);
        
        // 0.5% of 1000 = 5
        assert_eq!(fee, U256::from(5));
    }

    #[test]
    fn test_operator_management() {
        let mut bridge = ERC20Bridge::new();
        let admin = H160::from_low_u64_be(1);
        let operator = H160::from_low_u64_be(2);

        // Set treasury as admin
        bridge.bridge_treasury = admin;

        // Add operator
        let result = bridge.add_operator(admin, operator);
        assert!(result.is_ok());
        assert!(bridge.is_operator(operator));

        // Remove operator
        let result = bridge.remove_operator(admin, operator);
        assert!(result.is_ok());
        assert!(!bridge.is_operator(operator));
    }

    #[test]
    fn test_bridge_from_ethereum() {
        let mut bridge = ERC20Bridge::new();
        let mut registry = QRC20Registry::new();
        
        let user = H160::from_low_u64_be(1);
        let eth_token = H160::from_low_u64_be(999);
        let amount = U256::from(1000);
        let eth_tx_hash = H256::random();

        let qora_token = bridge.bridge_from_ethereum(
            &mut registry,
            eth_token,
            user,
            amount,
            "USDC".to_string(),
            "USDC".to_string(),
            6,
            eth_tx_hash,
            12,
        ).unwrap();

        // Check token was created and user has balance (minus fee)
        let token = registry.get_token(qora_token).unwrap();
        let expected_balance = amount - bridge.calculate_bridge_fee(amount);
        assert_eq!(token.balance_of(user), expected_balance);
        assert_eq!(token.symbol, "bUSDC");

        // Check mappings were created
        assert_eq!(bridge.eth_to_qora_mapping[&eth_token], qora_token);
        assert_eq!(bridge.qora_to_eth_mapping[&qora_token], eth_token);

        // Check locked amounts
        assert_eq!(bridge.locked_eth_tokens[&eth_token], amount);
    }

    #[test]
    fn test_bridge_to_ethereum() {
        let mut bridge = ERC20Bridge::new();
        let mut registry = QRC20Registry::new();
        
        let user = H160::from_low_u64_be(1);
        let eth_token = H160::from_low_u64_be(999);
        let amount = U256::from(1000);
        let bridge_amount = U256::from(500);

        // First bridge from Ethereum to create the token
        let qora_token = bridge.bridge_from_ethereum(
            &mut registry,
            eth_token,
            user,
            amount,
            "USDC".to_string(),
            "USDC".to_string(),
            6,
            H256::random(),
            12,
        ).unwrap();

        // Get initial balance
        let initial_balance = registry.get_token(qora_token).unwrap().balance_of(user);

        // Bridge back to Ethereum
        let result = bridge.bridge_to_ethereum(&mut registry, qora_token, user, bridge_amount);
        assert!(result.is_ok());

        // Check balance was reduced
        let final_balance = registry.get_token(qora_token).unwrap().balance_of(user);
        assert_eq!(final_balance, initial_balance - bridge_amount);

        // Check locked amounts were updated
        let expected_locked = amount - (bridge_amount - bridge.calculate_bridge_fee(bridge_amount));
        assert_eq!(bridge.locked_eth_tokens[&eth_token], expected_locked);
    }

    #[test]
    fn test_bridge_stats() {
        let mut bridge = ERC20Bridge::new();
        let mut registry = QRC20Registry::new();
        
        let user1 = H160::from_low_u64_be(1);
        let user2 = H160::from_low_u64_be(2);
        let eth_token1 = H160::from_low_u64_be(998);
        let eth_token2 = H160::from_low_u64_be(999);

        // Bridge multiple tokens
        let _qora_token1 = bridge.bridge_from_ethereum(
            &mut registry,
            eth_token1,
            user1,
            U256::from(1000),
            "USDC".to_string(),
            "USDC".to_string(),
            6,
            H256::random(),
            12,
        ).unwrap();

        let _qora_token2 = bridge.bridge_from_ethereum(
            &mut registry,
            eth_token2,
            user2,
            U256::from(2000),
            "USDT".to_string(),
            "USDT".to_string(),
            6,
            H256::random(),
            12,
        ).unwrap();

        let stats = bridge.get_bridge_stats();
        assert_eq!(stats.unique_tokens, 2);
        assert_eq!(stats.total_volume, U256::from(3000));
        assert_eq!(stats.completed_transactions, 2);
        assert_eq!(stats.total_transactions, 2);
    }
}
