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
                    hash_array.copy_from_slice
