use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use primitive_types::{H160, U256};
use super::{QRC20Error, QRC20Result, QRC20Event};

/// QRC-20 Token Standard - ERC-20 compatible on QoraNet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QRC20Token {
    /// Token metadata
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: U256,
    
    /// Token contract address
    pub contract_address: H160,
    
    /// Balance mapping: address => balance
    pub balances: HashMap<H160, U256>,
    
    /// Allowance mapping: owner => spender => amount
    pub allowances: HashMap<H160, HashMap<H160, U256>>,
    
    /// Owner of the contract
    pub owner: H160,
    
    /// Whether the token is paused
    pub paused: bool,
    
    /// Maximum supply (0 means no limit)
    pub max_supply: U256,
    
    /// Whether the token is mintable
    pub mintable: bool,
    
    /// Whether the token is burnable
    pub burnable: bool,
}

impl QRC20Token {
    /// Create new QRC-20 token
    pub fn new(
        name: String,
        symbol: String,
        decimals: u8,
        total_supply: U256,
        owner: H160,
    ) -> Self {
        let mut balances = HashMap::new();
        balances.insert(owner, total_supply);

        Self {
            name,
            symbol,
            decimals,
            total_supply,
            contract_address: H160::zero(), // Set during deployment
            balances,
            allowances: HashMap::new(),
            owner,
            paused: false,
            max_supply: U256::zero(), // No limit by default
            mintable: true,
            burnable: true,
        }
    }

    /// Create new token with advanced options
    pub fn new_advanced(
        name: String,
        symbol: String,
        decimals: u8,
        total_supply: U256,
        owner: H160,
        max_supply: U256,
        mintable: bool,
        burnable: bool,
    ) -> Self {
        let mut balances = HashMap::new();
        balances.insert(owner, total_supply);

        Self {
            name,
            symbol,
            decimals,
            total_supply,
            contract_address: H160::zero(),
            balances,
            allowances: HashMap::new(),
            owner,
            paused: false,
            max_supply,
            mintable,
            burnable,
        }
    }

    /// Get balance of an address
    pub fn balance_of(&self, account: H160) -> U256 {
        *self.balances.get(&account).unwrap_or(&U256::zero())
    }

    /// Transfer tokens between addresses
    pub fn transfer(&mut self, from: H160, to: H160, amount: U256) -> QRC20Result<QRC20Event> {
        if self.paused {
            return Err(QRC20Error::TokenPaused);
        }

        let from_balance = self.balance_of(from);
        if from_balance < amount {
            return Err(QRC20Error::InsufficientBalance { 
                required: amount, 
                available: from_balance 
            });
        }

        // Update balances
        self.balances.insert(from, from_balance - amount);
        let to_balance = self.balance_of(to);
        self.balances.insert(to, to_balance + amount);

        Ok(QRC20Event::Transfer {
            contract: self.contract_address,
            from,
            to,
            amount,
        })
    }

    /// Approve spender to spend tokens
    pub fn approve(&mut self, owner: H160, spender: H160, amount: U256) -> QRC20Result<QRC20Event> {
        if self.paused {
            return Err(QRC20Error::TokenPaused);
        }

        self.allowances
            .entry(owner)
            .or_insert_with(HashMap::new)
            .insert(spender, amount);

        Ok(QRC20Event::Approval {
            contract: self.contract_address,
            owner,
            spender,
            amount,
        })
    }

    /// Transfer tokens from one address to another (requires allowance)
    pub fn transfer_from(
        &mut self,
        spender: H160,
        from: H160,
        to: H160,
        amount: U256,
    ) -> QRC20Result<QRC20Event> {
        if self.paused {
            return Err(QRC20Error::TokenPaused);
        }

        // Check allowance
        let allowance = self.allowance(from, spender);
        if allowance < amount {
            return Err(QRC20Error::InsufficientAllowance { 
                required: amount, 
                available: allowance 
            });
        }

        // Check balance
        let from_balance = self.balance_of(from);
        if from_balance < amount {
            return Err(QRC20Error::InsufficientBalance { 
                required: amount, 
                available: from_balance 
            });
        }

        // Update allowance
        self.allowances
            .get_mut(&from)
            .unwrap()
            .insert(spender, allowance - amount);

        // Update balances
        self.balances.insert(from, from_balance - amount);
        let to_balance = self.balance_of(to);
        self.balances.insert(to, to_balance + amount);

        Ok(QRC20Event::Transfer {
            contract: self.contract_address,
            from,
            to,
            amount,
        })
    }

    /// Get allowance amount
    pub fn allowance(&self, owner: H160, spender: H160) -> U256 {
        self.allowances
            .get(&owner)
            .and_then(|allowances| allowances.get(&spender))
            .copied()
            .unwrap_or(U256::zero())
    }

    /// Mint new tokens (only owner)
    pub fn mint(&mut self, caller: H160, to: H160, amount: U256) -> QRC20Result<QRC20Event> {
        if caller != self.owner {
            return Err(QRC20Error::OnlyOwner);
        }

        if !self.mintable {
            return Err(QRC20Error::EVMExecutionFailed { 
                reason: "Token is not mintable".to_string() 
            });
        }

        if self.paused {
            return Err(QRC20Error::TokenPaused);
        }

        // Check max supply
        if !self.max_supply.is_zero() && self.total_supply + amount > self.max_supply {
            return Err(QRC20Error::EVMExecutionFailed { 
                reason: "Would exceed max supply".to_string() 
            });
        }

        let to_balance = self.balance_of(to);
        self.balances.insert(to, to_balance + amount);
        self.total_supply += amount;

        Ok(QRC20Event::Mint {
            contract: self.contract_address,
            to,
            amount,
        })
    }

    /// Burn tokens
    pub fn burn(&mut self, from: H160, amount: U256) -> QRC20Result<QRC20Event> {
        if !self.burnable {
            return Err(QRC20Error::EVMExecutionFailed { 
                reason: "Token is not burnable".to_string() 
            });
        }

        if self.paused {
            return Err(QRC20Error::TokenPaused);
        }

        let from_balance = self.balance_of(from);
        if from_balance < amount {
            return Err(QRC20Error::InsufficientBalance { 
                required: amount, 
                available: from_balance 
            });
        }

        self.balances.insert(from, from_balance - amount);
        self.total_supply -= amount;

        Ok(QRC20Event::Burn {
            contract: self.contract_address,
            from,
            amount,
        })
    }

    /// Pause token transfers (only owner)
    pub fn pause(&mut self, caller: H160) -> QRC20Result<QRC20Event> {
        if caller != self.owner {
            return Err(QRC20Error::OnlyOwner);
        }

        self.paused = true;
        Ok(QRC20Event::PauseStatusChanged {
            contract: self.contract_address,
            paused: true,
        })
    }

    /// Unpause token transfers (only owner)
    pub fn unpause(&mut self, caller: H160) -> QRC20Result<QRC20Event> {
        if caller != self.owner {
            return Err(QRC20Error::OnlyOwner);
        }

        self.paused = false;
        Ok(QRC20Event::PauseStatusChanged {
            contract: self.contract_address,
            paused: false,
        })
    }

    /// Transfer ownership
    pub fn transfer_ownership(&mut self, caller: H160, new_owner: H160) -> QRC20Result<QRC20Event> {
        if caller != self.owner {
            return Err(QRC20Error::OnlyOwner);
        }

        let old_owner = self.owner;
        self.owner = new_owner;
        
        Ok(QRC20Event::OwnershipTransferred {
            contract: self.contract_address,
            old_owner,
            new_owner,
        })
    }

    /// Set contract address (only called during deployment)
    pub fn set_contract_address(&mut self, address: H160) {
        self.contract_address = address;
    }

    /// Get token info for external queries
    pub fn get_info(&self) -> QRC20TokenInfo {
        QRC20TokenInfo {
            name: self.name.clone(),
            symbol: self.symbol.clone(),
            decimals: self.decimals,
            total_supply: self.total_supply,
            contract_address: self.contract_address,
            owner: self.owner,
            paused: self.paused,
            max_supply: self.max_supply,
            mintable: self.mintable,
            burnable: self.burnable,
        }
    }
}

/// QRC-20 Transaction types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QRC20Transaction {
    Deploy {
        name: String,
        symbol: String,
        decimals: u8,
        total_supply: U256,
        max_supply: Option<U256>,
        mintable: Option<bool>,
        burnable: Option<bool>,
    },
    Transfer {
        contract: H160,
        to: H160,
        amount: U256,
    },
    Approve {
        contract: H160,
        spender: H160,
        amount: U256,
    },
    TransferFrom {
        contract: H160,
        from: H160,
        to: H160,
        amount: U256,
    },
    Mint {
        contract: H160,
        to: H160,
        amount: U256,
    },
    Burn {
        contract: H160,
        amount: U256,
    },
    Pause {
        contract: H160,
    },
    Unpause {
        contract: H160,
    },
    TransferOwnership {
        contract: H160,
        new_owner: H160,
    },
}

/// QRC-20 token information for external queries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QRC20TokenInfo {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub total_supply: U256,
    pub contract_address: H160,
    pub owner: H160,
    pub paused: bool,
    pub max_supply: U256,
    pub mintable: bool,
    pub burnable: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_creation() {
        let owner = H160::from_low_u64_be(1);
        let token = QRC20Token::new(
            "Test Token".to_string(),
            "TEST".to_string(),
            18,
            U256::from(1000000),
            owner,
        );

        assert_eq!(token.name, "Test Token");
        assert_eq!(token.symbol, "TEST");
        assert_eq!(token.decimals, 18);
        assert_eq!(token.total_supply, U256::from(1000000));
        assert_eq!(token.balance_of(owner), U256::from(1000000));
    }

    #[test]
    fn test_transfer() {
        let owner = H160::from_low_u64_be(1);
        let recipient = H160::from_low_u64_be(2);
        let mut token = QRC20Token::new(
            "Test Token".to_string(),
            "TEST".to_string(),
            18,
            U256::from(1000),
            owner,
        );

        let result = token.transfer(owner, recipient, U256::from(100));
        assert!(result.is_ok());
        assert_eq!(token.balance_of(owner), U256::from(900));
        assert_eq!(token.balance_of(recipient), U256::from(100));
    }

    #[test]
    fn test_insufficient_balance() {
        let owner = H160::from_low_u64_be(1);
        let recipient = H160::from_low_u64_be(2);
        let mut token = QRC20Token::new(
            "Test Token".to_string(),
            "TEST".to_string(),
            18,
            U256::from(100),
            owner,
        );

        let result = token.transfer(owner, recipient, U256::from(200));
        assert!(result.is_err());
        matches!(result.unwrap_err(), QRC20Error::InsufficientBalance { .. });
    }

    #[test]
    fn test_approve_and_transfer_from() {
        let owner = H160::from_low_u64_be(1);
        let spender = H160::from_low_u64_be(2);
        let recipient = H160::from_low_u64_be(3);
        let mut token = QRC20Token::new(
            "Test Token".to_string(),
            "TEST".to_string(),
            18,
            U256::from(1000),
            owner,
        );

        // Approve
        let result = token.approve(owner, spender, U256::from(200));
        assert!(result.is_ok());
        assert_eq!(token.allowance(owner, spender), U256::from(200));

        // Transfer from
        let result = token.transfer_from(spender, owner, recipient, U256::from(100));
        assert!(result.is_ok());
        assert_eq!(token.balance_of(owner), U256::from(900));
        assert_eq!(token.balance_of(recipient), U256::from(100));
        assert_eq!(token.allowance(owner, spender), U256::from(100));
    }

    #[test]
    fn test_mint_and_burn() {
        let owner = H160::from_low_u64_be(1);
        let user = H160::from_low_u64_be(2);
        let mut token = QRC20Token::new(
            "Test Token".to_string(),
            "TEST".to_string(),
            18,
            U256::from(1000),
            owner,
        );

        // Mint
        let result = token.mint(owner, user, U256::from(500));
        assert!(result.is_ok());
        assert_eq!(token.balance_of(user), U256::from(500));
        assert_eq!(token.total_supply, U256::from(1500));

        // Burn
        let result = token.burn(user, U256::from(200));
        assert!(result.is_ok());
        assert_eq!(token.balance_of(user), U256::from(300));
        assert_eq!(token.total_supply, U256::from(1300));
    }

    #[test]
    fn test_pause_functionality() {
        let owner = H160::from_low_u64_be(1);
        let recipient = H160::from_low_u64_be(2);
        let mut token = QRC20Token::new(
            "Test Token".to_string(),
            "TEST".to_string(),
            18,
            U256::from(1000),
            owner,
        );

        // Pause
        let result = token.pause(owner);
        assert!(result.is_ok());
        assert!(token.paused);

        // Try to transfer while paused
        let result = token.transfer(owner, recipient, U256::from(100));
        assert!(result.is_err());
        matches!(result.unwrap_err(), QRC20Error::TokenPaused);

        // Unpause
        let result = token.unpause(owner);
        assert!(result.is_ok());
        assert!(!token.paused);

        // Transfer should work now
        let result = token.transfer(owner, recipient, U256::from(100));
        assert!(result.is_ok());
    }
}
