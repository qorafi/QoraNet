use qoranet::{
    consensus::{ConsensusState, ValidatorInfo, Block},
    transaction::TransactionPool,
    storage::BlockchainStorage,
    app_monitor::AppMonitor,
    fee_oracle::GlobalFeeOracle,
    Address, Result, QoraNetError, Balance,
};
use clap::{Arg, Command};
use ed25519_dalek::Keypair;
use rand::rngs::OsRng;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, error, warn};
use tracing_subscriber;

/// QoraNet Validator Node
#[derive(Debug)]
struct ValidatorNode {
    /// Node keypair for signing
    keypair: Keypair,
    
    /// Node address
    address: Address,
    
    /// Blockchain storage
    storage: Arc<RwLock<BlockchainStorage>>,
    
    /// Transaction pool
    tx_pool: Arc<RwLock<TransactionPool>>,
    
    /// Consensus state
    consensus: Arc<RwLock<ConsensusState>>,
    
    /// Application monitor
    app_monitor: Arc<RwLock<AppMonitor>>,
    
    /// Fee oracle
    fee_oracle: Arc<GlobalFeeOracle>,
    
    /// Configuration
    config: ValidatorConfig,
}

#[derive(Debug, Clone)]
struct ValidatorConfig {
    pub data_dir: PathBuf,
    pub min_liquidity_requirement: u64,
    pub min_apps_requirement: usize,
    pub block_time_seconds: u64,
    pub max_block_size: usize,
    pub max_transactions_per_block: usize,
}

impl ValidatorConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./qoranet-data"),
            min_liquidity_requirement: Balance::from_qor(1000.0).amount, // 1000 QOR minimum
            min_apps_requirement: 1, // At least 1 app
            block_time_seconds: 10, // 10 second blocks
            max_block_size: 1024 * 1024, // 1MB max block size
            max_transactions_per_block: 1000,
        }
    }
}

impl ValidatorNode {
    /// Create new validator node
    async fn new(config: ValidatorConfig) -> Result<Self> {
        // Generate or load keypair
        let mut csprng = OsRng;
        let keypair = Keypair::generate(&mut csprng);
        let address = Address::from_pubkey(&keypair.public);
        
        info!("🚀 Starting QoraNet Validator: {}", address);
        
        // Initialize storage
        let storage_path = config.data_dir.join("blockchain");
        std::fs::create_dir_all(&storage_path)?;
        let storage = BlockchainStorage::new(storage_path)?;
        let storage = Arc::new(RwLock::new(storage));
        
        // Initialize transaction pool
        let tx_pool = Arc::new(RwLock::new(TransactionPool::new()));
        
        // Initialize consensus
        let consensus = ConsensusState::new(
            config.min_liquidity_requirement,
            config.min_apps_requirement,
        );
        let consensus = Arc::new(RwLock::new(consensus));
        
        // Initialize application monitor
        let app_monitor = AppMonitor::new(address.clone());
        let app_monitor = Arc::new(RwLock::new(app_monitor));
        
        // Initialize fee oracle
        let fee_oracle = Arc::new(GlobalFeeOracle::new());
        
        // Register self as validator
        let validator_info = ValidatorInfo::new(address.clone());
        consensus.write().await.update_validator(validator_info)?;
        
        Ok(Self {
            keypair,
            address,
            storage,
            tx_pool,
            consensus,
            app_monitor,
            fee_oracle,
            config,
        })
    }
    
    /// Start the validator node
    async fn start(&mut self) -> Result<()> {
        info!("🌊 QoraNet Validator starting...");
        info!("📍 Validator Address: {}", self.address);
        info!("💰 Min Liquidity: {} QOR", Balance::new(self.config.min_liquidity_requirement));
        info!("🖥️  Min Apps: {}", self.config.min_apps_requirement);
        
        // Initialize genesis block if needed
        self.initialize_genesis().await?;
        
        // Start background tasks
        let fee_oracle = Arc::clone(&self.fee_oracle);
        let consensus = Arc::clone(&self.consensus);
        let storage = Arc::clone(&self.storage);
        let tx_pool = Arc::clone(&self.tx_pool);
        let block_time = self.config.block_time_seconds;
        let max_txs = self.config.max_transactions_per_block;
        let validator_address = self.address.clone();
        let keypair = self.keypair.clone();
        
        // Fee oracle update task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                if let Err(e) = fee_oracle.update_price().await {
                    warn!("Failed to update QOR price: {}", e);
                }
            }
        });
        
        // Block production task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(block_time));
            loop {
                interval.tick().await;
                
                match Self::try_produce_block(
                    &consensus,
                    &storage,
                    &tx_pool,
                    &validator_address,
                    max_txs,
                ).await {
                    Ok(Some(block)) => {
                        info!("📦 Produced block #{} with {} transactions", 
                            block.header.height, 
                            block.transactions.len()
                        );
                    },
                    Ok(None) => {
                        // Not selected to produce block this round
                    },
                    Err(e) => {
                        error!("Failed to produce block: {}", e);
                    }
                }
            }
        });
        
        info!("✅ QoraNet Validator started successfully!");
        
        // Keep the main thread alive
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            self.print_status().await;
        }
    }
    
    /// Initialize genesis block if blockchain is empty
    async fn initialize_genesis(&mut self) -> Result<()> {
        let storage = self.storage.read().await;
        let (latest_hash, latest_height) = storage.get_latest_block_info();
        
        if latest_hash.is_none() && latest_height == 0 {
            drop(storage); // Release read lock
            
            info!("🌱 Creating genesis block...");
            let genesis_block = Block::genesis(self.address.clone());
            
            let mut storage = self.storage.write().await;
            storage.store_block(&genesis_block)?;
            
            info!("✅ Genesis block created: {}", genesis_block.hash());
        }
        
        Ok(())
    }
    
    /// Try to produce a block
    async fn try_produce_block(
        consensus: &Arc<RwLock<ConsensusState>>,
        storage: &Arc<RwLock<BlockchainStorage>>,
        tx_pool: &Arc<RwLock<TransactionPool>>,
        validator_address: &Address,
        max_transactions: usize,
    ) -> Result<Option<Block>> {
        let consensus_state = consensus.read().await;
        let (latest_hash, latest_height) = {
            let storage = storage.read().await;
            storage.get_latest_block_info()
        };
        
        let previous_hash = latest_hash.unwrap_or_else(|| crate::Hash::zero());
        let new_height = latest_height + 1;
        
        // Check if this validator is selected to produce the block
        let selected_validator = consensus_state.select_block_producer(previous_hash.as_bytes())?;
        if selected_validator != *validator_address {
            return Ok(None); // Not selected
        }
        
        // Get transactions from pool
        let transactions = {
            let pool = tx_pool.read().await;
            pool.get_transactions_for_block(max_transactions)
        };
        
        // Get network stats
        let total_liquidity = consensus_state.total_network_liquidity();
        let active_apps = consensus_state.total_active_apps() as u32;
        
        drop(consensus_state);
        
        // Create new block
        let block = Block::new(
            previous_hash,
            new_height,
            validator_address.clone(),
            transactions.clone(),
            total_liquidity,
            active_apps,
        );
        
        // Validate and store block
        block.validate(new_height, &previous_hash)?;
        
        {
            let mut storage = storage.write().await;
            storage.store_block(&block)?;
        }
        
        // Remove transactions from pool
        {
            let mut pool = tx_pool.write().await;
            for tx in &transactions {
                pool.remove_transaction(&tx.hash());
            }
        }
        
        // Update consensus height
        {
            let mut consensus_state = consensus.write().await;
            consensus_state.update_height(new_height);
        }
        
        Ok(Some(block))
    }
    
    /// Print node status
    async fn print_status(&self) {
        let (latest_hash, latest_height) = {
            let storage = self.storage.read().await;
            storage.get_latest_block_info()
        };
        
        let pending_txs = {
            let pool = self.tx_pool.read().await;
            pool.pending_count()
        };
        
        let qor_price = self.fee_oracle.get_qor_price().await;
        
        let consensus_stats = {
            let consensus = self.consensus.read().await;
            (
                consensus.validator_count(),
                consensus.eligible_validator_count(),
                consensus.total_network_liquidity(),
                consensus.total_active_apps(),
            )
        };
        
        info!("📊 Node Status:");
        info!("  Latest Block: #{} ({})", 
            latest_height, 
            latest_hash.as_ref().map(|h| h.to_string()).unwrap_or_else(|| "None".to_string())
        );
        info!("  Pending TXs: {}", pending_txs);
        info!("  QOR Price: ${:.6}", qor_price);
        info!("  Validators: {} total, {} eligible", consensus_stats.0, consensus_stats.1);
        info!("  Network Liquidity: {} QOR", Balance::new(consensus_stats.2));
        info!("  Active Apps: {}", consensus_stats.3);
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Parse command line arguments
    let matches = Command::new("qoranet-validator")
        .version(qoranet::VERSION)
        .about("QoraNet Validator Node")
        .arg(
            Arg::new("data-dir")
                .long("data-dir")
                .short('d')
                .help("Data directory for blockchain storage")
                .default_value("./qoranet-data")
        )
        .arg(
            Arg::new("min-liquidity")
                .long("min-liquidity")
                .help("Minimum liquidity requirement in QOR")
                .default_value("1000")
        )
        .arg(
            Arg::new("min-apps")
                .long("min-apps")
                .help("Minimum number of apps required")
                .default_value("1")
        )
        .arg(
            Arg::new("block-time")
                .long("block-time")
                .help("Block time in seconds")
                .default_value("10")
        )
        .get_matches();
    
    // Create configuration
    let mut config = ValidatorConfig::default();
    config.data_dir = PathBuf::from(matches.get_one::<String>("data-dir").unwrap());
    
    if let Some(min_liquidity) = matches.get_one::<String>("min-liquidity") {
        let liquidity_qor: f64 = min_liquidity.parse()
            .map_err(|_| QoraNetError::InvalidTransaction("Invalid min-liquidity value".to_string()))?;
        config.min_liquidity_requirement = Balance::from_qor(liquidity_qor).amount;
    }
    
    if let Some(min_apps) = matches.get_one::<String>("min-apps") {
        config.min_apps_requirement = min_apps.parse()
            .map_err(|_| QoraNetError::InvalidTransaction("Invalid min-apps value".to_string()))?;
    }
    
    if let Some(block_time) = matches.get_one::<String>("block-time") {
        config.block_time_seconds = block_time.parse()
            .map_err(|_| QoraNetError::InvalidTransaction("Invalid block-time value".to_string()))?;
    }
    
    // Create and start validator
    let mut validator = ValidatorNode::new(config).await?;
    validator.start().await?;
    
    Ok(())
}
