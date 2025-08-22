use crate::{Result, QoraNetError, MIN_FEE_USD, MAX_FEE_USD, DEFAULT_FEE_USD, usd_to_qor, qor_to_usd};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::{Duration, Instant};

/// Price oracle for QOR token and fee calculation
#[derive(Debug, Clone)]
pub struct FeeOracle {
    qor_price_usd: f64,
    last_update: Instant,
    update_interval: Duration,
    price_sources: Vec<PriceSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceSource {
    pub name: String,
    pub url: String,
    pub weight: f64, // Weight for price aggregation
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransactionType {
    Transfer,
    ProvideLiquidity,
    RegisterApp,
    ReportMetrics,
    ClaimRewards,
    SmartContract { complexity: ContractComplexity },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ContractComplexity {
    Simple,   // Basic operations
    Medium,   // Moderate computation
    Complex,  // Heavy computation
}

impl FeeOracle {
    pub fn new() -> Self {
        Self {
            qor_price_usd: 1.0, // Default price, will be updated
            last_update: Instant::now(),
            update_interval: Duration::from_secs(60), // Update every minute
            price_sources: vec![
                PriceSource {
                    name: "CoinGecko".to_string(),
                    url: "https://api.coingecko.com/api/v3/simple/price?ids=qor&vs_currencies=usd".to_string(),
                    weight: 0.4,
                },
                PriceSource {
                    name: "CoinMarketCap".to_string(),
                    url: "https://pro-api.coinmarketcap.com/v1/cryptocurrency/quotes/latest".to_string(),
                    weight: 0.4,
                },
                PriceSource {
                    name: "DEX Price".to_string(),
                    url: "internal://dex-price".to_string(),
                    weight: 0.2,
                },
            ],
        }
    }
    
    /// Get current QOR price in USD
    pub fn get_qor_price(&self) -> f64 {
        self.qor_price_usd
    }
    
    /// Update QOR price from external sources
    pub async fn update_price(&mut self) -> Result<()> {
        if self.last_update.elapsed() < self.update_interval {
            return Ok(()); // Too soon to update
        }
        
        let mut total_weighted_price = 0.0;
        let mut total_weight = 0.0;
        
        for source in &self.price_sources {
            if let Ok(price) = self.fetch_price_from_source(source).await {
                total_weighted_price += price * source.weight;
                total_weight += source.weight;
            }
        }
        
        if total_weight > 0.0 {
            self.qor_price_usd = total_weighted_price / total_weight;
            self.last_update = Instant::now();
        }
        
        Ok(())
    }
    
    /// Fetch price from a specific source
    async fn fetch_price_from_source(&self, source: &PriceSource) -> Result<f64> {
        match source.url.as_str() {
            url if url.starts_with("internal://dex-price") => {
                // Get price from internal DEX pools
                self.get_dex_price().await
            },
            _ => {
                // Fetch from external API
                self.fetch_external_price(&source.url).await
            }
        }
    }
    
    /// Get price from internal DEX pools
    async fn get_dex_price(&self) -> Result<f64> {
        // In a real implementation, this would query the DEX pools
        // For now, return a mock price
        Ok(self.qor_price_usd) // Placeholder
    }
    
    /// Fetch price from external API
    async fn fetch_external_price(&self, url: &str) -> Result<f64> {
        // In a real implementation, this would make HTTP requests
        // For now, return a mock price with some variation
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let variation = rng.gen_range(-0.05..0.05); // ±5% variation
        Ok(self.qor_price_usd * (1.0 + variation))
    }
    
    /// Calculate transaction fee in QOR tokens
    pub fn calculate_fee(&self, tx_type: &TransactionType, priority: FeePriority) -> u64 {
        let base_fee_usd = self.get_base_fee_usd(tx_type);
        let priority_multiplier = self.get_priority_multiplier(priority);
        let final_fee_usd = (base_fee_usd * priority_multiplier).clamp(MIN_FEE_USD, MAX_FEE_USD);
        
        usd_to_qor(final_fee_usd, self.qor_price_usd)
    }
    
    /// Get base fee in USD for transaction type
    fn get_base_fee_usd(&self, tx_type: &TransactionType) -> f64 {
        match tx_type {
            TransactionType::Transfer => DEFAULT_FEE_USD,
            TransactionType::ProvideLiquidity => DEFAULT_FEE_USD * 2.0,
            TransactionType::RegisterApp => DEFAULT_FEE_USD * 5.0,
            TransactionType::ReportMetrics => DEFAULT_FEE_USD * 0.5,
            TransactionType::ClaimRewards => DEFAULT_FEE_USD * 1.5,
            TransactionType::SmartContract { complexity } => {
                match complexity {
                    ContractComplexity::Simple => DEFAULT_FEE_USD * 3.0,
                    ContractComplexity::Medium => DEFAULT_FEE_USD * 10.0,
                    ContractComplexity::Complex => DEFAULT_FEE_USD * 50.0,
                }
            }
        }
    }
    
    /// Get priority multiplier
    fn get_priority_multiplier(&self, priority: FeePriority) -> f64 {
        match priority {
            FeePriority::Low => 1.0,
            FeePriority::Medium => 1.5,
            FeePriority::High => 2.0,
            FeePriority::Urgent => 5.0,
        }
    }
    
    /// Validate fee amount
    pub fn validate_fee(&self, fee_qor: u64, tx_type: &TransactionType) -> Result<()> {
        let fee_usd = qor_to_usd(fee_qor, self.qor_price_usd);
        let min_required_usd = self.get_base_fee_usd(tx_type);
        
        if fee_usd < min_required_usd {
            return Err(QoraNetError::InvalidTransaction(
                format!("Fee too low: ${:.6} provided, ${:.6} required", fee_usd, min_required_usd)
            ));
        }
        
        if fee_usd > MAX_FEE_USD {
            return Err(QoraNetError::InvalidTransaction(
                format!("Fee too high: ${:.6} provided, ${:.6} maximum", fee_usd, MAX_FEE_USD)
            ));
        }
        
        Ok(())
    }
    
    /// Get fee estimate for UI
    pub fn get_fee_estimate(&self, tx_type: &TransactionType) -> FeeEstimate {
        FeeEstimate {
            low: self.calculate_fee(tx_type, FeePriority::Low),
            medium: self.calculate_fee(tx_type, FeePriority::Medium),
            high: self.calculate_fee(tx_type, FeePriority::High),
            urgent: self.calculate_fee(tx_type, FeePriority::Urgent),
            qor_price_usd: self.qor_price_usd,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeePriority {
    Low,     // 1x multiplier
    Medium,  // 1.5x multiplier  
    High,    // 2x multiplier
    Urgent,  // 5x multiplier
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeeEstimate {
    pub low: u64,      // QOR amount for low priority
    pub medium: u64,   // QOR amount for medium priority
    pub high: u64,     // QOR amount for high priority
    pub urgent: u64,   // QOR amount for urgent priority
    pub qor_price_usd: f64, // Current QOR price
}

impl FeeEstimate {
    /// Get fee in USD for a specific priority
    pub fn get_usd_fee(&self, priority: FeePriority) -> f64 {
        let qor_amount = match priority {
            FeePriority::Low => self.low,
            FeePriority::Medium => self.medium,
            FeePriority::High => self.high,
            FeePriority::Urgent => self.urgent,
        };
        
        qor_to_usd(qor_amount, self.qor_price_usd)
    }
}

/// Global fee oracle instance
pub struct GlobalFeeOracle {
    oracle: tokio::sync::RwLock<FeeOracle>,
}

impl GlobalFeeOracle {
    pub fn new() -> Self {
        Self {
            oracle: tokio::sync::RwLock::new(FeeOracle::new()),
        }
    }
    
    pub async fn get_fee_estimate(&self, tx_type: &TransactionType) -> FeeEstimate {
        let oracle = self.oracle.read().await;
        oracle.get_fee_estimate(tx_type)
    }
    
    pub async fn calculate_fee(&self, tx_type: &TransactionType, priority: FeePriority) -> u64 {
        let oracle = self.oracle.read().await;
        oracle.calculate_fee(tx_type, priority)
    }
    
    pub async fn validate_fee(&self, fee_qor: u64, tx_type: &TransactionType) -> Result<()> {
        let oracle = self.oracle.read().await;
        oracle.validate_fee(fee_qor, tx_type)
    }
    
    pub async fn update_price(&self) -> Result<()> {
        let mut oracle = self.oracle.write().await;
        oracle.update_price().await
    }
    
    pub async fn get_qor_price(&self) -> f64 {
        let oracle = self.oracle.read().await;
        oracle.get_qor_price()
    }
}
