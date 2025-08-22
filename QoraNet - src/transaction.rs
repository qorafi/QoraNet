/// Add transaction to pool
    pub async fn add_transaction(&mut self, transaction: Transaction, fee_oracle: &GlobalFeeOracle) -> Result<()> {
        // Validate transaction
        transaction.validate(fee_oracle).await?;
        
        let tx_hash = transaction.hash();
        let signer = transaction.signer.clone();
        
        // Add to pending
        self.pending.insert(tx_hash.clone(), transactionuse crate::{Address, Hash, QoraSignature, Result, QoraNetError, LPToken, AppMetrics, Balance, TransactionType, FeePriority, GlobalFeeOracle};
use serde::{Deserialize, Serialize};
use ed25519_dalek::{Keypair, Signer};

/// Transaction types in QoraNet
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionData {
    /// Transfer tokens between accounts
    Transfer {
        from: Address,
        to: Address,
        amount: u64,
    },
    /// Provide liquidity to DEX pool
    ProvideLiquidity {
        provider: Address,
        lp_tokens: Vec<LPToken>,
    },
    /// Register application for hosting
    RegisterApp {
        owner: Address,
        app_id: String,
        app_type: AppType,
        resource_requirements: ResourceRequirements,
    },
    /// Report application performance metrics
    ReportMetrics {
        validator: Address,
        app_owner: Address,
        app_id: String,
        metrics: AppMetrics,
    },
    /// Claim rewards for liquidity provision and app hosting
    ClaimRewards {
        claimant: Address,
        lp_rewards: u64,
        app_rewards: u64,
    },
}

/// Types of applications that can be hosted
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppType {
    StorageNode,
    OracleService,
    ComputeNode,
    IndexingService,
    RelayNode,
}

/// Resource requirements for applications
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub min_cpu_cores: u32,
    pub min_memory_gb: u32,
    pub min_disk_gb: u32,
    pub min_bandwidth_mbps: u32,
}

/// Complete transaction with signature
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub data: TransactionData,
    pub nonce: u64,
    pub fee_qor: u64,        // Fee amount in QOR tokens
    pub fee_usd: f64,        // Fee amount in USD (for validation)
    pub priority: FeePriority, // Transaction priority
    pub signature: QoraSignature,
    pub signer: Address,
}

impl Transaction {
    /// Create a new transaction with automatic fee calculation
    pub async fn new(
        data: TransactionData, 
        nonce: u64, 
        priority: FeePriority,
        keypair: &Keypair,
        fee_oracle: &GlobalFeeOracle
    ) -> Result<Self> {
        let signer = Address::from_pubkey(&keypair.public);
        
        // Determine transaction type
        let tx_type = match &data {
            TransactionData::Transfer { .. } => TransactionType::Transfer,
            TransactionData::ProvideLiquidity { .. } => TransactionType::ProvideLiquidity,
            TransactionData::RegisterApp { .. } => TransactionType::RegisterApp,
            TransactionData::ReportMetrics { .. } => TransactionType::ReportMetrics,
            TransactionData::ClaimRewards { .. } => TransactionType::ClaimRewards,
        };
        
        // Calculate fee
        let fee_qor = fee_oracle.calculate_fee(&tx_type, priority.clone()).await;
        let fee_estimate = fee_oracle.get_fee_estimate(&tx_type).await;
        let fee_usd = fee_estimate.get_usd_fee(priority.clone());
        
        let mut tx = Self {
            data,
            nonce,
            fee_qor,
            fee_usd,
            priority,
            signature: QoraSignature::from_bytes(&[0u8; 64]).unwrap(), // Placeholder
            signer,
        };
        
        // Sign the transaction
        let message = tx.signing_message();
        tx.signature = keypair.sign(&message);
        
        Ok(tx)
    }
    
    /// Create transaction with custom fee (must still be valid)
    pub async fn new_with_fee(
        data: TransactionData,
        nonce: u64,
        fee_qor: u64,
        priority: FeePriority,
        keypair: &Keypair,
        fee_oracle: &GlobalFeeOracle
    ) -> Result<Self> {
        let signer = Address::from_pubkey(&keypair.public);
        
        // Determine transaction type and validate fee
        let tx_type = match &data {
            TransactionData::Transfer { .. } => TransactionType::Transfer,
            TransactionData::ProvideLiquidity { .. } => TransactionType::ProvideLiquidity,
            TransactionData::RegisterApp { .. } => TransactionType::RegisterApp,
            TransactionData::ReportMetrics { .. } => TransactionType::ReportMetrics,
            TransactionData::ClaimRewards { .. } => TransactionType::ClaimRewards,
        };
        
        // Validate fee
        fee_oracle.validate_fee(fee_qor, &tx_type).await?;
        
        let qor_price = fee_oracle.get_qor_price().await;
        let fee_usd = crate::qor_to_usd(fee_qor, qor_price);
        
        let mut tx = Self {
            data,
            nonce,
            fee_qor,
            fee_usd,
            priority,
            signature: QoraSignature::from_bytes(&[0u8; 64]).unwrap(), // Placeholder
            signer,
        };
        
        // Sign the transaction
        let message = tx.signing_message();
        tx.signature = keypair.sign(&message);
        
        Ok(tx)
    }
    
    /// Get the message that should be signed
    pub fn signing_message(&self) -> Vec<u8> {
        let mut message = Vec::new();
        message.extend_from_slice(&bincode::serialize(&self.data).unwrap());
        message.extend_from_slice(&self.nonce.to_le_bytes());
        message.extend_from_slice(&self.fee_qor.to_le_bytes());
        message.extend_from_slice(&self.fee_usd.to_le_bytes());
        message.extend_from_slice(&bincode::serialize(&self.priority).unwrap());
        message.extend_from_slice(&self.signer.as_bytes());
        message
    }
    
    /// Verify transaction signature
    pub fn verify_signature(&self) -> Result<()> {
        use ed25519_dalek::{PublicKey, Verifier};
        
        let pubkey = PublicKey::from_bytes(&self.signer.0)
            .map_err(|e| QoraNetError::InvalidTransaction(format!("Invalid pubkey: {}", e)))?;
            
        let message = self.signing_message();
        
        pubkey.verify(&message, &self.signature)
            .map_err(|e| QoraNetError::InvalidTransaction(format!("Invalid signature: {}", e)))?;
            
        Ok(())
    }
    
    /// Get transaction hash
    pub fn hash(&self) -> Hash {
        let serialized = bincode::serialize(self).unwrap();
        Hash::new(&serialized)
    }
    
    /// Validate transaction logic
    pub async fn validate(&self, fee_oracle: &GlobalFeeOracle) -> Result<()> {
        // Verify signature first
        self.verify_signature()?;
        
        // Validate fee
        let tx_type = match &self.data {
            TransactionData::Transfer { .. } => TransactionType::Transfer,
            TransactionData::ProvideLiquidity { .. } => TransactionType::ProvideLiquidity,
            TransactionData::RegisterApp { .. } => TransactionType::RegisterApp,
            TransactionData::ReportMetrics { .. } => TransactionType::ReportMetrics,
            TransactionData::ClaimRewards { .. } => TransactionType::ClaimRewards,
        };
        
        fee_oracle.validate_fee(self.fee_qor, &tx_type).await?;
        
        // Validate transaction-specific logic
        match &self.data {
            TransactionData::Transfer { amount, .. } => {
                if *amount == 0 {
                    return Err(QoraNetError::InvalidTransaction("Transfer amount cannot be zero".to_string()));
                }
            },
            TransactionData::ProvideLiquidity { lp_tokens, .. } => {
                if lp_tokens.is_empty() {
                    return Err(QoraNetError::InvalidTransaction("LP tokens cannot be empty".to_string()));
                }
                for lp_token in lp_tokens {
                    if lp_token.amount == 0 {
                        return Err(QoraNetError::InvalidTransaction("LP token amount cannot be zero".to_string()));
                    }
                }
            },
            TransactionData::RegisterApp { app_id, resource_requirements, .. } => {
                if app_id.is_empty() {
                    return Err(QoraNetError::InvalidTransaction("App ID cannot be empty".to_string()));
                }
                if resource_requirements.min_cpu_cores == 0 {
                    return Err(QoraNetError::InvalidTransaction("Minimum CPU cores must be > 0".to_string()));
                }
            },
            TransactionData::ReportMetrics { metrics, .. } => {
                if metrics.cpu_usage > 100.0 {
                    return Err(QoraNetError::InvalidTransaction("CPU usage cannot exceed 100%".to_string()));
                }
            },
            TransactionData::ClaimRewards { lp_rewards, app_rewards, .. } => {
                if *lp_rewards == 0 && *app_rewards == 0 {
                    return Err(QoraNetError::InvalidTransaction("Cannot claim zero rewards".to_string()));
                }
            },
        }
        
        Ok(())
    }
}

/// Transaction pool for pending transactions
#[derive(Debug)]
pub struct TransactionPool {
    pending: std::collections::HashMap<Hash, Transaction>,
    by_signer: std::collections::HashMap<Address, Vec<Hash>>,
}

impl TransactionPool {
    pub fn new() -> Self {
        Self {
            pending: std::collections::HashMap::new(),
            by_signer: std::collections::HashMap::new(),
        }
    }
    
    /// Add transaction to pool
    pub fn add_transaction(&mut self, transaction: Transaction) -> Result<()> {
        // Validate transaction
        transaction.validate()?;
        
        let tx_hash = transaction.hash();
        let signer = transaction.signer.clone();
        
        // Add to pending
        self.pending.insert(tx_hash.clone(), transaction);
        
        // Add to by_signer index
        self.by_signer
            .entry(signer)
            .or_insert_with(Vec::new)
            .push(tx_hash);
            
        Ok(())
    }
    
    /// Remove transaction from pool
    pub fn remove_transaction(&mut self, tx_hash: &Hash) -> Option<Transaction> {
        if let Some(transaction) = self.pending.remove(tx_hash) {
            // Remove from by_signer index
            if let Some(tx_hashes) = self.by_signer.get_mut(&transaction.signer) {
                tx_hashes.retain(|h| h != tx_hash);
                if tx_hashes.is_empty() {
                    self.by_signer.remove(&transaction.signer);
                }
            }
            Some(transaction)
        } else {
            None
        }
    }
    
    /// Get transactions for block creation
    pub fn get_transactions_for_block(&self, max_count: usize) -> Vec<Transaction> {
        self.pending
            .values()
            .take(max_count)
            .cloned()
            .collect()
    }
    
    /// Get pending transaction count
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }
}
