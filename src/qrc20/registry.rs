use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use primitive_types::{H160, U256};
use super::{QRC20Token, QRC20Transaction, QRC20Error, QRC20Result, QRC20Event};

/// QRC-20 Registry - manages all tokens on QoraNet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QRC20Registry {
    /// All registered tokens: contract_address => token
    pub tokens: HashMap<H160, QRC20Token>,
    
    /// Token symbol to address mapping for quick lookup
    pub symbol_to_address: HashMap<String, H160>,
    
    /// Token name to address mapping
    pub name_to_address: HashMap<String, H160>,
    
    /// Next contract address counter
    pub next_contract_id: u64,
    
    /// Registry owner (can be governance contract later)
    pub registry_owner: H160,
}

impl QRC20Registry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            tokens: HashMap::new(),
            symbol_to_address: HashMap::new(),
            name_to_address: HashMap::new(),
            next_contract_id: 1000, // Start from 1000 to avoid conflicts
            registry_owner: H160::zero(), // Set to governance later
        }
    }

    /// Create new registry with owner
    pub fn with_owner(owner: H160) -> Self {
        let mut registry = Self::new();
        registry.registry_owner = owner;
        registry
    }

    /// Deploy new QRC-20 token
    pub fn deploy_token(
        &mut self,
        deployer: H160,
        name: String,
        symbol: String,
        decimals: u8,
        total_supply: U256,
    ) -> QRC20Result<H160> {
        self.deploy_token_advanced(
            deployer,
            name,
            symbol,
            decimals,
            total_supply,
            None,    // No max supply limit
            Some(true),  // Mintable by default
            Some(true),  // Burnable by default
        )
    }

    /// Deploy new QRC-20 token with advanced options
    pub fn deploy_token_advanced(
        &mut self,
        deployer: H160,
        name: String,
        symbol: String,
        decimals: u8,
        total_supply: U256,
        max_supply: Option<U256>,
        mintable: Option<bool>,
        burnable: Option<bool>,
    ) -> QRC20Result<H160> {
        // Check if symbol already exists
        if self.symbol_to_address.contains_key(&symbol) {
            return Err(QRC20Error::SymbolExists { symbol });
        }

        // Check if name already exists
        if self.name_to_address.contains_key(&name) {
            return Err(QRC20Error::EVMExecutionFailed { 
                reason: format!("Token name '{}' already exists", name)
            });
        }

        // Generate contract address
        let contract_address = H160::from_low_u64_be(self.next_contract_id);
        self.next_contract_id += 1;

        // Create token
        let mut token = if let Some(max_supply) = max_supply {
            QRC20Token::new_advanced(
                name.clone(),
                symbol.clone(),
                decimals,
                total_supply,
                deployer,
                max_supply,
                mintable.unwrap_or(true),
                burnable.unwrap_or(true),
            )
        } else {
            QRC20Token::new(name.clone(), symbol.clone(), decimals, total_supply, deployer)
        };

        token.set_contract_address(contract_address);

        // Register token
        self.tokens.insert(contract_address, token);
        self.symbol_to_address.insert(symbol, contract_address);
        self.name_to_address.insert(name, contract_address);

        tracing::info!(
            "Deployed QRC-20 token: {} ({}) at address {:?}",
            name,
            symbol,
            contract_address
        );

        Ok(contract_address)
    }

    /// Execute QRC-20 transaction
    pub fn execute_transaction(
        &mut self,
        caller: H160,
        tx: QRC20Transaction,
    ) -> QRC20Result<QRC20Event> {
        match tx {
            QRC20Transaction::Deploy { 
                name, 
                symbol, 
                decimals, 
                total_supply,
                max_supply,
                mintable,
                burnable,
            } => {
                let contract_address = self.deploy_token_advanced(
                    caller, 
                    name.clone(), 
                    symbol.clone(), 
                    decimals, 
                    total_supply,
                    max_supply,
                    mintable,
                    burnable,
                )?;

                Ok(QRC20Event::Deploy {
                    contract: contract_address,
                    deployer: caller,
                    name,
                    symbol,
                    total_supply,
                })
            }
            
            QRC20Transaction::Transfer { contract, to, amount } => {
                let token = self.tokens.get_mut(&contract)
                    .ok_or(QRC20Error::TokenNotFound)?;
                token.transfer(caller, to, amount)
            }
            
            QRC20Transaction::Approve { contract, spender, amount } => {
                let token = self.tokens.get_mut(&contract)
                    .ok_or(QRC20Error::TokenNotFound)?;
                token.approve(caller, spender, amount)
            }
            
            QRC20Transaction::TransferFrom { contract, from, to, amount } => {
                let token = self.tokens.get_mut(&contract)
                    .ok_or(QRC20Error::TokenNotFound)?;
                token.transfer_from(caller, from, to, amount)
            }
            
            QRC20Transaction::Mint { contract, to, amount } => {
                let token = self.tokens.get_mut(&contract)
                    .ok_or(QRC20Error::TokenNotFound)?;
                token.mint(caller, to, amount)
            }
            
            QRC20Transaction::Burn { contract, amount } => {
                let token = self.tokens.get_mut(&contract)
                    .ok_or(QRC20Error::TokenNotFound)?;
                token.burn(caller, amount)
            }

            QRC20Transaction::Pause { contract } => {
                let token = self.tokens.get_mut(&contract)
                    .ok_or(QRC20Error::TokenNotFound)?;
                token.pause(caller)
            }

            QRC20Transaction::Unpause { contract } => {
                let token = self.tokens.get_mut(&contract)
                    .ok_or(QRC20Error::TokenNotFound)?;
                token.unpause(caller)
            }

            QRC20Transaction::TransferOwnership { contract, new_owner } => {
                let token = self.tokens.get_mut(&contract)
                    .ok_or(QRC20Error::TokenNotFound)?;
                token.transfer_ownership(caller, new_owner)
            }
        }
    }

    /// Get token by address
    pub fn get_token(&self, address: H160) -> Option<&QRC20Token> {
        self.tokens.get(&address)
    }

    /// Get mutable token by address
    pub fn get_token_mut(&mut self, address: H160) -> Option<&mut QRC20Token> {
        self.tokens.get_mut(&address)
    }

    /// Get token by symbol
    pub fn get_token_by_symbol(&self, symbol: &str) -> Option<&QRC20Token> {
        self.symbol_to_address
            .get(symbol)
            .and_then(|addr| self.tokens.get(addr))
    }

    /// Get token by name
    pub fn get_token_by_name(&self, name: &str) -> Option<&QRC20Token> {
        self.name_to_address
            .get(name)
            .and_then(|addr| self.tokens.get(addr))
    }

    /// List all tokens
    pub fn list_tokens(&self) -> Vec<&QRC20Token> {
        self.tokens.values().collect()
    }

    /// Get token count
    pub fn token_count(&self) -> usize {
        self.tokens.len()
    }

    /// Check if token exists
    pub fn token_exists(&self, address: H160) -> bool {
        self.tokens.contains_key(&address)
    }

    /// Check if symbol is available
    pub fn is_symbol_available(&self, symbol: &str) -> bool {
        !self.symbol_to_address.contains_key(symbol)
    }

    /// Check if name is available
    pub fn is_name_available(&self, name: &str) -> bool {
        !self.name_to_address.contains_key(name)
    }

    /// Get all token addresses
    pub fn get_all_addresses(&self) -> Vec<H160> {
        self.tokens.keys().copied().collect()
    }

    /// Get tokens by owner
    pub fn get_tokens_by_owner(&self, owner: H160) -> Vec<&QRC20Token> {
        self.tokens
            .values()
            .filter(|token| token.owner == owner)
            .collect()
    }

    /// Get total supply of all tokens (for analytics)
    pub fn get_total_value_locked(&self) -> U256 {
        self.tokens
            .values()
            .fold(U256::zero(), |acc, token| acc + token.total_supply)
    }

    /// Remove token (for emergency situations only)
    pub fn remove_token(&mut self, caller: H160, contract: H160) -> QRC20Result<()> {
        // Only registry owner can remove tokens
        if caller != self.registry_owner && !self.registry_owner.is_zero() {
            return Err(QRC20Error::OnlyOwner);
        }

        if let Some(token) = self.tokens.remove(&contract) {
            self.symbol_to_address.remove(&token.symbol);
            self.name_to_address.remove(&token.name);
            
            tracing::warn!(
                "Removed QRC-20 token: {} ({}) at address {:?}",
                token.name,
                token.symbol,
                contract
            );
        }

        Ok(())
    }

    /// Update registry owner
    pub fn transfer_registry_ownership(&mut self, caller: H160, new_owner: H160) -> QRC20Result<()> {
        if caller != self.registry_owner && !self.registry_owner.is_zero() {
            return Err(QRC20Error::OnlyOwner);
        }

        let old_owner = self.registry_owner;
        self.registry_owner = new_owner;

        tracing::info!(
            "Registry ownership transferred from {:?} to {:?}",
            old_owner,
            new_owner
        );

        Ok(())
    }
}

impl Default for QRC20Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = QRC20Registry::new();
        assert_eq!(registry.token_count(), 0);
        assert_eq!(registry.next_contract_id, 1000);
    }

    #[test]
    fn test_token_deployment() {
        let mut registry = QRC20Registry::new();
        let deployer = H160::from_low_u64_be(1);
        
        let contract = registry.deploy_token(
            deployer,
            "Test Token".to_string(),
            "TEST".to_string(),
            18,
            U256::from(1000000),
        ).unwrap();
        
        assert_eq!(registry.token_count(), 1);
        assert!(registry.token_exists(contract));
        
        let token = registry.get_token(contract).unwrap();
        assert_eq!(token.name, "Test Token");
        assert_eq!(token.symbol, "TEST");
        assert_eq!(token.balance_of(deployer), U256::from(1000000));
    }

    #[test]
    fn test_duplicate_symbol_rejection() {
        let mut registry = QRC20Registry::new();
        let deployer = H160::from_low_u64_be(1);
        
        // Deploy first token
        let _contract1 = registry.deploy_token(
            deployer,
            "Test Token 1".to_string(),
            "TEST".to_string(),
            18,
            U256::from(1000000),
        ).unwrap();

        // Try to deploy second token with same symbol
        let result = registry.deploy_token(
            deployer,
            "Test Token 2".to_string(),
            "TEST".to_string(), // Same symbol
            18,
            U256::from(1000000),
        );

        assert!(result.is_err());
        matches!(result.unwrap_err(), QRC20Error::SymbolExists { .. });
    }

    #[test]
    fn test_token_lookup() {
        let mut registry = QRC20Registry::new();
        let deployer = H160::from_low_u64_be(1);
        
        let contract = registry.deploy_token(
            deployer,
            "Test Token".to_string(),
            "TEST".to_string(),
            18,
            U256::from(1000000),
        ).unwrap();

        // Test symbol lookup
        let token_by_symbol = registry.get_token_by_symbol("TEST").unwrap();
        assert_eq!(token_by_symbol.contract_address, contract);

        // Test name lookup
        let token_by_name = registry.get_token_by_name("Test Token").unwrap();
        assert_eq!(token_by_name.contract_address, contract);
    }

    #[test]
    fn test_transaction_execution() {
        let mut registry = QRC20Registry::new();
        let deployer = H160::from_low_u64_be(1);
        let recipient = H160::from_low_u64_be(2);
        
        // Deploy token
        let deploy_tx = QRC20Transaction::Deploy {
            name: "Test Token".to_string(),
            symbol: "TEST".to_string(),
            decimals: 18,
            total_supply: U256::from(1000),
            max_supply: None,
            mintable: Some(true),
            burnable: Some(true),
        };

        let deploy_event = registry.execute_transaction(deployer, deploy_tx).unwrap();
        let contract = match deploy_event {
            QRC20Event::Deploy { contract, .. } => contract,
            _ => panic!("Expected Deploy event"),
        };

        // Transfer tokens
        let transfer_tx = QRC20Transaction::Transfer {
            contract,
            to: recipient,
            amount: U256::from(100),
        };

        let transfer_event = registry.execute_transaction(deployer, transfer_tx).unwrap();
        match transfer_event {
            QRC20Event::Transfer { from, to, amount, .. } => {
                assert_eq!(from, deployer);
                assert_eq!(to, recipient);
                assert_eq!(amount, U256::from(100));
            }
            _ => panic!("Expected Transfer event"),
        }

        // Verify balances
        let token = registry.get_token(contract).unwrap();
        assert_eq!(token.balance_of(deployer), U256::from(900));
        assert_eq!(token.balance_of(recipient), U256::from(100));
    }

    #[test]
    fn test_advanced_deployment() {
        let mut registry = QRC20Registry::new();
        let deployer = H160::from_low_u64_be(1);
        
        let contract = registry.deploy_token_advanced(
            deployer,
            "Limited Token".to_string(),
            "LTD".to_string(),
            6,
            U256::from(1000),
            Some(U256::from(10000)), // Max supply
            Some(false), // Not mintable
            Some(false), // Not burnable
        ).unwrap();

        let token = registry.get_token(contract).unwrap();
        assert_eq!(token.max_supply, U256::from(10000));
        assert!(!token.mintable);
        assert!(!token.burnable);
    }

    #[test]
    fn test_tokens_by_owner() {
        let mut registry = QRC20Registry::new();
        let owner1 = H160::from_low_u64_be(1);
        let owner2 = H160::from_low_u64_be(2);
        
        // Deploy tokens by owner1
        let _contract1 = registry.deploy_token(
            owner1,
            "Token 1".to_string(),
            "TK1".to_string(),
            18,
            U256::from(1000),
        ).unwrap();

        let _contract2 = registry.deploy_token(
            owner1,
            "Token 2".to_string(),
            "TK2".to_string(),
            18,
            U256::from(2000),
        ).unwrap();

        // Deploy token by owner2
        let _contract3 = registry.deploy_token(
            owner2,
            "Token 3".to_string(),
            "TK3".to_string(),
            18,
            U256::from(3000),
        ).unwrap();

        let owner1_tokens = registry.get_tokens_by_owner(owner1);
        let owner2_tokens = registry.get_tokens_by_owner(owner2);

        assert_eq!(owner1_tokens.len(), 2);
        assert_eq!(owner2_tokens.len(), 1);
    }
}
