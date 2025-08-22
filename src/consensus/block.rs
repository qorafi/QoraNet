use crate::{Hash, Address, BlockHeight, Timestamp, transaction::Transaction, Result, QoraNetError};
use serde::{Deserialize, Serialize};
use chrono::Utc;

/// Block header containing metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Previous block hash
    pub previous_hash: Hash,
    
    /// Merkle root of all transactions
    pub transactions_root: Hash,
    
    /// Block height (sequential number)
    pub height: BlockHeight,
    
    /// Timestamp when block was created
    pub timestamp: Timestamp,
    
    /// Address of validator who produced this block
    pub validator: Address,
    
    /// Total liquidity value at time of block creation
    pub total_liquidity: u64,
    
    /// Number of active applications being hosted
    pub active_apps: u32,
    
    /// Total QOR fees collected in this block
    pub total_fees: u64,
    
    /// Block version for future upgrades
    pub version: u32,
    
    /// Nonce for additional entropy
    pub nonce: u64,
}

impl BlockHeader {
    pub fn new(
        previous_hash: Hash,
        transactions_root: Hash,
        height: BlockHeight,
        validator: Address,
        total_liquidity: u64,
        active_apps: u32,
        total_fees: u64,
    ) -> Self {
        Self {
            previous_hash,
            transactions_root,
            height,
            timestamp: Utc::now().timestamp() as u64,
            validator,
            total_liquidity,
            active_apps,
            total_fees,
            version: 1,
            nonce: 0,
        }
    }
    
    /// Calculate block hash
    pub fn hash(&self) -> Hash {
        let serialized = bincode::serialize(self).unwrap();
        Hash::new(&serialized)
    }
    
    /// Validate block header
    pub fn validate(&self, expected_height: BlockHeight, expected_previous: &Hash) -> Result<()> {
        if self.height != expected_height {
            return Err(QoraNetError::ConsensusError(
                format!("Invalid block height: expected {}, got {}", expected_height, self.height)
            ));
        }
        
        if self.previous_hash != *expected_previous {
            return Err(QoraNetError::ConsensusError(
                "Invalid previous block hash".to_string()
            ));
        }
        
        // Validate timestamp (not too far in the future)
        let now = Utc::now().timestamp() as u64;
        if self.timestamp > now + 300 { // 5 minutes tolerance
            return Err(QoraNetError::ConsensusError(
                "Block timestamp too far in the future".to_string()
            ));
        }
        
        Ok(())
    }
}

/// Complete block with header and transactions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Block {
    pub header: BlockHeader,
    pub transactions: Vec<Transaction>,
}

impl Block {
    pub fn new(
        previous_hash: Hash,
        height: BlockHeight,
        validator: Address,
        transactions: Vec<Transaction>,
        total_liquidity: u64,
        active_apps: u32,
    ) -> Self {
        // Calculate total fees
        let total_fees: u64 = transactions.iter().map(|tx| tx.fee_qor).sum();
        
        // Calculate merkle root of transactions
        let transactions_root = Self::calculate_transactions_root(&transactions);
        
        let header = BlockHeader::new(
            previous_hash,
            transactions_root,
            height,
            validator,
            total_liquidity,
            active_apps,
            total_fees,
        );
        
        Self {
            header,
            transactions,
        }
    }
    
    /// Calculate merkle root of transactions
    fn calculate_transactions_root(transactions: &[Transaction]) -> Hash {
        if transactions.is_empty() {
            return Hash::zero();
        }
        
        let mut hashes: Vec<Hash> = transactions.iter().map(|tx| tx.hash()).collect();
        
        // Build merkle tree
        while hashes.len() > 1 {
            let mut next_level = Vec::new();
            
            for chunk in hashes.chunks(2) {
                let combined = if chunk.len() == 2 {
                    let mut combined_data = Vec::new();
                    combined_data.extend_from_slice(chunk[0].as_bytes());
                    combined_data.extend_from_slice(chunk[1].as_bytes());
                    Hash::new(&combined_data)
                } else {
                    // Odd number, hash with itself
                    let mut combined_data = Vec::new();
                    combined_data.extend_from_slice(chunk[0].as_bytes());
                    combined_data.extend_from_slice(chunk[0].as_bytes());
                    Hash::new(&combined_data)
                };
                next_level.push(combined);
            }
            
            hashes = next_level;
        }
        
        hashes[0].clone()
    }
    
    /// Get block hash
    pub fn hash(&self) -> Hash {
        self.header.hash()
    }
    
    /// Get block size in bytes
    pub fn size(&self) -> usize {
        bincode::serialize(self).unwrap().len()
    }
    
    /// Validate entire block
    pub fn validate(&self, expected_height: BlockHeight, expected_previous: &Hash) -> Result<()> {
        // Validate header
        self.header.validate(expected_height, expected_previous)?;
        
        // Validate transactions root
        let calculated_root = Self::calculate_transactions_root(&self.transactions);
        if calculated_root != self.header.transactions_root {
            return Err(QoraNetError::ConsensusError(
                "Invalid transactions root".to_string()
            ));
        }
        
        // Validate total fees
        let calculated_fees: u64 = self.transactions.iter().map(|tx| tx.fee_qor).sum();
        if calculated_fees != self.header.total_fees {
            return Err(QoraNetError::ConsensusError(
                "Invalid total fees".to_string()
            ));
        }
        
        // Validate individual transactions
        for tx in &self.transactions {
            tx.verify_signature()?;
        }
        
        Ok(())
    }
    
    /// Get transaction by hash
    pub fn get_transaction(&self, tx_hash: &Hash) -> Option<&Transaction> {
        self.transactions.iter().find(|tx| &tx.hash() == tx_hash)
    }
    
    /// Get all transaction hashes
    pub fn transaction_hashes(&self) -> Vec<Hash> {
        self.transactions.iter().map(|tx| tx.hash()).collect()
    }
}

/// Genesis block creation
impl Block {
    pub fn genesis(genesis_validator: Address) -> Self {
        Self::new(
            Hash::zero(),  // No previous block
            0,            // Height 0
            genesis_validator,
            Vec::new(),   // No transactions
            0,            // No initial liquidity
            0,            // No initial apps
        )
    }
}

/// Block statistics for monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockStats {
    pub height: BlockHeight,
    pub timestamp: Timestamp,
    pub validator: Address,
    pub transaction_count: usize,
    pub total_fees_qor: u64,
    pub total_fees_usd: f64,
    pub block_size_bytes: usize,
    pub total_liquidity: u64,
    pub active_apps: u32,
    pub processing_time_ms: u64,
}

impl BlockStats {
    pub fn from_block(block: &Block, fee_usd: f64, processing_time_ms: u64) -> Self {
        Self {
            height: block.header.height,
            timestamp: block.header.timestamp,
            validator: block.header.validator.clone(),
            transaction_count: block.transactions.len(),
            total_fees_qor: block.header.total_fees,
            total_fees_usd: fee_usd,
            block_size_bytes: block.size(),
            total_liquidity: block.header.total_liquidity,
            active_apps: block.header.active_apps,
            processing_time_ms,
        }
    }
}
