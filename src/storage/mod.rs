use crate::{Hash, Address, BlockHeight, Result, QoraNetError, Balance};
use crate::consensus::Block;
use crate::transaction::Transaction;
use serde::{Deserialize, Serialize};
use rocksdb::{DB, Options, IteratorMode};
use std::path::Path;
use std::collections::HashMap;

/// Database column families
pub const CF_BLOCKS: &str = "blocks";
pub const CF_TRANSACTIONS: &str = "transactions";
pub const CF_ACCOUNTS: &str = "accounts";
pub const CF_VALIDATORS: &str = "validators";
pub const CF_APPS: &str = "applications";
pub const CF_METADATA: &str = "metadata";

/// Account state information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountState {
    pub address: Address,
    pub balance: Balance,
    pub nonce: u64,
    pub created_at: u64,
    pub last_updated: u64,
}

impl AccountState {
    pub fn new(address: Address) -> Self {
        let now = chrono::Utc::now().timestamp() as u64;
        Self {
            address,
            balance: Balance::zero(),
            nonce: 0,
            created_at: now,
            last_updated: now,
        }
    }
    
    pub fn update_balance(&mut self, new_balance: Balance) {
        self.balance = new_balance;
        self.last_updated = chrono::Utc::now().timestamp() as u64;
    }
    
    pub fn increment_nonce(&mut self) {
        self.nonce += 1;
        self.last_updated = chrono::Utc::now().timestamp() as u64;
    }
}

/// Blockchain storage layer
#[derive(Debug)]
pub struct BlockchainStorage {
    db: DB,
    cache: StorageCache,
}

/// In-memory cache for frequently accessed data
#[derive(Debug)]
struct StorageCache {
    latest_block_hash: Option<Hash>,
    latest_block_height: BlockHeight,
    account_cache: HashMap<Address, AccountState>,
    cache_size_limit: usize,
}

impl StorageCache {
    fn new() -> Self {
        Self {
            latest_block_hash: None,
            latest_block_height: 0,
            account_cache: HashMap::new(),
            cache_size_limit: 10000, // Cache up to 10k accounts
        }
    }
    
    fn cache_account(&mut self, account: AccountState) {
        if self.account_cache.len() >= self.cache_size_limit {
            // Simple eviction: remove oldest entry
            if let Some((oldest_addr, _)) = self.account_cache.iter().min_by_key(|(_, acc)| acc.last_updated) {
                let oldest_addr = oldest_addr.clone();
                self.account_cache.remove(&oldest_addr);
            }
        }
        
        self.account_cache.insert(account.address.clone(), account);
    }
    
    fn get_cached_account(&self, address: &Address) -> Option<&AccountState> {
        self.account_cache.get(address)
    }
    
    fn invalidate_account(&mut self, address: &Address) {
        self.account_cache.remove(address);
    }
}

impl BlockchainStorage {
    /// Open or create blockchain storage
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        
        let column_families = vec![CF_BLOCKS, CF_TRANSACTIONS, CF_ACCOUNTS, CF_VALIDATORS, CF_APPS, CF_METADATA];
        
        let db = DB::open_cf(&opts, path, column_families)
            .map_err(|e| QoraNetError::StorageError(format!("Failed to open database: {}", e)))?;
        
        let mut storage = Self {
            db,
            cache: StorageCache::new(),
        };
        
        // Initialize cache with latest block info
        storage.load_latest_block_info()?;
        
        Ok(storage)
    }
    
    /// Store a block
    pub fn store_block(&mut self, block: &Block) -> Result<()> {
        let block_hash = block.hash();
        let serialized_block = bincode::serialize(block)
            .map_err(|e| QoraNetError::StorageError(format!("Failed to serialize block: {}", e)))?;
        
        // Store block
        let cf_blocks = self.db.cf_handle(CF_BLOCKS)
            .ok_or_else(|| QoraNetError::StorageError("Blocks column family not found".to_string()))?;
        
        self.db.put_cf(cf_blocks, block_hash.as_bytes(), &serialized_block)
            .map_err(|e| QoraNetError::StorageError(format!("Failed to store block: {}", e)))?;
        
        // Store block hash by height for quick lookup
        self.db.put_cf(cf_blocks, format!("height:{}", block.header.height).as_bytes(), block_hash.as_bytes())
            .map_err(|e| QoraNetError::StorageError(format!("Failed to store block height mapping: {}", e)))?;
        
        // Store individual transactions
        self.store_block_transactions(&block.transactions)?;
        
        // Update cache
        self.cache.latest_block_hash = Some(block_hash);
        self.cache.latest_block_height = block.header.height;
        
        // Update metadata
        self.update_metadata("latest_block_hash", block_hash.as_bytes())?;
        self.update_metadata("latest_block_height", &block.header.height.to_le_bytes())?;
        
        Ok(())
    }
    
    /// Store transactions from a block
    fn store_block_transactions(&self, transactions: &[Transaction]) -> Result<()> {
        let cf_transactions = self.db.cf_handle(CF_TRANSACTIONS)
            .ok_or_else(|| QoraNetError::StorageError("Transactions column family not found".to_string()))?;
        
        for tx in transactions {
            let tx_hash = tx.hash();
            let serialized_tx = bincode::serialize(tx)
                .map_err(|e| QoraNetError::StorageError(format!("Failed to serialize transaction: {}", e)))?;
            
            self.db.put_cf(cf_transactions, tx_hash.as_bytes(), &serialized_tx)
                .map_err(|e| QoraNetError::StorageError(format!("Failed to store transaction: {}", e)))?;
        }
        
        Ok(())
    }
    
    /// Get block by hash
    pub fn get_block(&self, block_hash: &Hash) -> Result<Option<Block>> {
        let cf_blocks = self.db.cf_handle(CF_BLOCKS)
            .ok_or_else(|| QoraNetError::StorageError("Blocks column family not found".to_string()))?;
        
        match self.db.get_cf(cf_blocks, block_hash.as_bytes()) {
            Ok(Some(data)) => {
                let block = bincode::deserialize(&data)
                    .map_err(|e| QoraNetError::StorageError(format!("Failed to deserialize block: {}", e)))?;
                Ok(Some(block))
            },
            Ok(None) => Ok(None),
            Err(e) => Err(QoraNetError::StorageError(format!("Failed to get block: {}", e))),
        }
    }
    
    /// Get block by height
    pub fn get_block_by_height(&self, height: BlockHeight) -> Result<Option<Block>> {
        let cf_blocks = self.db.cf_handle(CF_BLOCKS)
            .ok_or_else(|| QoraNetError::StorageError("Blocks column family not found".to_string()))?;
        
        // Get block hash by height
        let height_key = format!("height:{}", height);
        match self.db.get_cf(cf_blocks, height_key.as_bytes()) {
            Ok(Some(hash_bytes)) => {
                if hash_bytes.len() == 32 {
                    let mut hash_array = [0u8; 32];
                    hash_array.copy_from_slice(&hash_bytes);
                    let block_hash = Hash(hash_array);
                    self.get_block(&block_hash)
                } else {
                    Err(QoraNetError::StorageError("Invalid block hash length".to_string()))
                }
            },
            Ok(None) => Ok(None),
            Err(e) => Err(QoraNetError::StorageError(format!("Failed to get block by height: {}", e))),
        }
    }
    
    /// Get transaction by hash
    pub fn get_transaction(&self, tx_hash: &Hash) -> Result<Option<Transaction>> {
        let cf_transactions = self.db.cf_handle(CF_TRANSACTIONS)
            .ok_or_else(|| QoraNetError::StorageError("Transactions column family not found".to_string()))?;
        
        match self.db.get_cf(cf_transactions, tx_hash.as_bytes()) {
            Ok(Some(data)) => {
                let transaction = bincode::deserialize(&data)
                    .map_err(|e| QoraNetError::StorageError(format!("Failed to deserialize transaction: {}", e)))?;
                Ok(Some(transaction))
            },
            Ok(None) => Ok(None),
            Err(e) => Err(QoraNetError::StorageError(format!("Failed to get transaction: {}", e))),
        }
    }
    
    /// Store account state
    pub fn store_account(&mut self, account: &AccountState) -> Result<()> {
        let cf_accounts = self.db.cf_handle(CF_ACCOUNTS)
            .ok_or_else(|| QoraNetError::StorageError("Accounts column family not found".to_string()))?;
        
        let serialized_account = bincode::serialize(account)
            .map_err(|e| QoraNetError::StorageError(format!("Failed to serialize account: {}", e)))?;
        
        self.db.put_cf(cf_accounts, account.address.as_bytes(), &serialized_account)
            .map_err(|e| QoraNetError::StorageError(format!("Failed to store account: {}", e)))?;
        
        // Update cache
        self.cache.cache_account(account.clone());
        
        Ok(())
    }
    
    /// Get account state
    pub fn get_account(&self, address: &Address) -> Result<Option<AccountState>> {
        // Check cache first
        if let Some(account) = self.cache.get_cached_account(address) {
            return Ok(Some(account.clone()));
        }
        
        // Get from database
        let cf_accounts = self.db.cf_handle(CF_ACCOUNTS)
            .ok_or_else(|| QoraNetError::StorageError("Accounts column family not found".to_string()))?;
        
        match self.db.get_cf(cf_accounts, address.as_bytes()) {
            Ok(Some(data)) => {
                let account = bincode::deserialize(&data)
                    .map_err(|e| QoraNetError::StorageError(format!("Failed to deserialize account: {}", e)))?;
                Ok(Some(account))
            },
            Ok(None) => Ok(None),
            Err(e) => Err(QoraNetError::StorageError(format!("Failed to get account: {}", e))),
        }
    }
    
    /// Get or create account state
    pub fn get_or_create_account(&mut self, address: &Address) -> Result<AccountState> {
        match self.get_account(address)? {
            Some(account) => Ok(account),
            None => {
                let new_account = AccountState::new(address.clone());
                self.store_account(&new_account)?;
                Ok(new_account)
            }
        }
    }
    
    /// Update account balance
    pub fn update_account_balance(&mut self, address: &Address, new_balance: Balance) -> Result<()> {
        let mut account = self.get_or_create_account(address)?;
        account.update_balance(new_balance);
        self.store_account(&account)?;
        Ok(())
    }
    
    /// Increment account nonce
    pub fn increment_account_nonce(&mut self, address: &Address) -> Result<u64> {
        let mut account = self.get_or_create_account(address)?;
        account.increment_nonce();
        let new_nonce = account.nonce;
        self.store_account(&account)?;
        Ok(new_nonce)
    }
    
    /// Get latest block info
    pub fn get_latest_block_info(&self) -> (Option<Hash>, BlockHeight) {
        (self.cache.latest_block_hash.clone(), self.cache.latest_block_height)
    }
    
    /// Load latest block info into cache
    fn load_latest_block_info(&mut self) -> Result<()> {
        // Load latest block height
        if let Some(height_bytes) = self.get_metadata("latest_block_height")? {
            if height_bytes.len() == 8 {
                let mut height_array = [0u8; 8];
                height_array.copy_from_slice(&height_bytes);
                self.cache.latest_block_height = u64::from_le_bytes(height_array);
            }
        }
        
        // Load latest block hash
        if let Some(hash_bytes) = self.get_metadata("latest_block_hash")? {
            if hash_bytes.len() == 32 {
                let mut hash_array = [0u8; 32];
                hash_array.copy_from_slice(&hash_bytes);
                self.cache.latest_block_hash = Some(Hash(hash_array));
            }
        }
        
        Ok(())
    }
    
    /// Update metadata
    fn update_metadata(&self, key: &str, value: &[u8]) -> Result<()> {
        let cf_metadata = self.db.cf_handle(CF_METADATA)
            .ok_or_else(|| QoraNetError::StorageError("Metadata column family not found".to_string()))?;
        
        self.db.put_cf(cf_metadata, key.as_bytes(), value)
            .map_err(|e| QoraNetError::StorageError(format!("Failed to update metadata: {}", e)))?;
        
        Ok(())
    }
    
    /// Get metadata
    fn get_metadata(&self, key: &str) -> Result<Option<Vec<u8>>> {
        let cf_metadata = self.db.cf_handle(CF_METADATA)
            .ok_or_else(|| QoraNetError::StorageError("Metadata column family not found".to_string()))?;
        
        match self.db.get_cf(cf_metadata, key.as_bytes()) {
            Ok(data) => Ok(data),
            Err(e) => Err(QoraNetError::StorageError(format!("Failed to get metadata: {}", e))),
        }
    }
    
    /// Get block range
    pub fn get_blocks_range(&self, start_height: BlockHeight, end_height: BlockHeight) -> Result<Vec<Block>> {
        let mut blocks = Vec::new();
        
        for height in start_height..=end_height {
            if let Some(block) = self.get_block_by_height(height)? {
                blocks.push(block);
            }
        }
        
        Ok(blocks)
    }
    
    /// Get recent transactions for an account
    pub fn get_account_transactions(&self, address: &Address, limit: usize) -> Result<Vec<Transaction>> {
        let cf_transactions = self.db.cf_handle(CF_TRANSACTIONS)
            .ok_or_else(|| QoraNetError::StorageError("Transactions column family not found".to_string()))?;
        
        let mut transactions = Vec::new();
        let iter = self.db.iterator_cf(cf_transactions, IteratorMode::Start);
        
        for item in iter {
            match item {
                Ok((_, value)) => {
                    if let Ok(tx) = bincode::deserialize::<Transaction>(&value) {
                        // Check if transaction involves this address
                        let involves_address = match &tx.data {
                            crate::transaction::TransactionData::Transfer { from, to, .. } => {
                                from == address || to == address
                            },
                            crate::transaction::TransactionData::ProvideLiquidity { provider, .. } => {
                                provider == address
                            },
                            crate::transaction::TransactionData::RegisterApp { owner, .. } => {
                                owner == address
                            },
                            crate::transaction::TransactionData::ReportMetrics { app_owner, .. } => {
                                app_owner == address
                            },
                            crate::transaction::TransactionData::ClaimRewards { claimant, .. } => {
                                claimant == address
                            },
                        };
                        
                        if involves_address {
                            transactions.push(tx);
                            if transactions.len() >= limit {
                                break;
                            }
                        }
                    }
                },
                Err(_) => continue,
            }
        }
        
        // Sort by most recent first (would need block timestamp in real implementation)
        transactions.reverse();
        Ok(transactions)
    }
    
    /// Database statistics
    pub fn get_storage_stats(&self) -> Result<StorageStats> {
        let cf_blocks = self.db.cf_handle(CF_BLOCKS).unwrap();
        let cf_transactions = self.db.cf_handle(CF_TRANSACTIONS).unwrap();
        let cf_accounts = self.db.cf_handle(CF_ACCOUNTS).unwrap();
        
        // Count entries (simplified - in production would use more efficient method)
        let mut block_count = 0;
        let mut transaction_count = 0;
        let mut account_count = 0;
        
        for _ in self.db.iterator_cf(cf_blocks, IteratorMode::Start) {
            block_count += 1;
        }
        
        for _ in self.db.iterator_cf(cf_transactions, IteratorMode::Start) {
            transaction_count += 1;
        }
        
        for _ in self.db.iterator_cf(cf_accounts, IteratorMode::Start) {
            account_count += 1;
        }
        
        Ok(StorageStats {
            latest_block_height: self.cache.latest_block_height,
            total_blocks: block_count / 2, // Divide by 2 because we store height mapping too
            total_transactions: transaction_count,
            total_accounts: account_count,
            cache_size: self.cache.account_cache.len(),
        })
    }
    
    /// Flush cache to disk
    pub fn flush(&mut self) -> Result<()> {
        // Invalidate cache to force reload from disk
        self.cache.account_cache.clear();
        self.load_latest_block_info()?;
        Ok(())
    }
}

/// Storage statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageStats {
    pub latest_block_height: BlockHeight,
    pub total_blocks: usize,
    pub total_transactions: usize,
    pub total_accounts: usize,
    pub cache_size: usize,
}
