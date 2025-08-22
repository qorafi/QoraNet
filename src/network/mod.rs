use crate::{Hash, Address, Result, QoraNetError};
use crate::consensus::Block;
use crate::transaction::Transaction;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc};
use tracing::{info, warn, debug};

/// Network message types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    /// New transaction broadcast
    NewTransaction(Transaction),
    
    /// New block broadcast  
    NewBlock(Block),
    
    /// Request for block by hash
    BlockRequest(Hash),
    
    /// Block response
    BlockResponse(Option<Block>),
    
    /// Request for transaction by hash
    TransactionRequest(Hash),
    
    /// Transaction response
    TransactionResponse(Option<Transaction>),
    
    /// Peer discovery
    PeerDiscovery {
        peer_id: String,
        address: String,
        port: u16,
    },
    
    /// Validator announcement
    ValidatorAnnouncement {
        validator: Address,
        stake: u64,
        apps_count: u32,
    },
    
    /// App metrics broadcast
    AppMetrics {
        validator: Address,
        app_id: String,
        metrics: crate::AppMetrics,
    },
    
    /// Ping for connectivity check
    Ping {
        timestamp: u64,
        peer_id: String,
    },
    
    /// Pong response
    Pong {
        timestamp: u64,
        peer_id: String,
    },
}

/// Peer information
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub peer_id: String,
    pub address: String,
    pub port: u16,
    pub last_seen: SystemTime,
    pub validator_address: Option<Address>,
    pub stake: u64,
    pub apps_count: u32,
    pub ping_ms: Option<u64>,
    pub connection_status: ConnectionStatus,
}

#[derive(Debug, Clone)]
pub enum ConnectionStatus {
    Connected,
    Connecting,
    Disconnected,
    Failed(String),
}

/// Network manager for P2P communication
#[derive(Debug)]
pub struct NetworkManager {
    /// Our peer ID
    peer_id: String,
    
    /// Our validator address
    validator_address: Address,
    
    /// Known peers
    peers: HashMap<String, PeerInfo>,
    
    /// Message broadcaster
    message_tx: broadcast::Sender<NetworkMessage>,
    
    /// Message receiver
    message_rx: broadcast::Receiver<NetworkMessage>,
    
    /// Outgoing message queue
    outgoing_tx: mpsc::UnboundedSender<(String, NetworkMessage)>, // (peer_id, message)
    outgoing_rx: mpsc::UnboundedReceiver<(String, NetworkMessage)>,
    
    /// Network configuration
    config: NetworkConfig,
}

#[derive(Debug, Clone)]
pub struct NetworkConfig {
    pub listen_port: u16,
    pub max_peers: usize,
    pub connection_timeout: Duration,
    pub ping_interval: Duration,
    pub bootstrap_peers: Vec<String>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_port: 8080,
            max_peers: 100,
            connection_timeout: Duration::from_secs(10),
            ping_interval: Duration::from_secs(30),
            bootstrap_peers: Vec::new(),
        }
    }
}

impl NetworkManager {
    /// Create new network manager
    pub fn new(validator_address: Address, config: NetworkConfig) -> Self {
        let peer_id = format!("qora-{}", hex::encode(&validator_address.0[..8]));
        let (message_tx, message_rx) = broadcast::channel(1000);
        let (outgoing_tx, outgoing_rx) = mpsc::unbounded_channel();
        
        Self {
            peer_id,
            validator_address,
            peers: HashMap::new(),
            message_tx,
            message_rx,
            outgoing_tx,
            outgoing_rx,
            config,
        }
    }
    
    /// Start the network manager
    pub async fn start(&mut self) -> Result<()> {
        info!("🌐 Starting QoraNet P2P network...");
        info!("📡 Peer ID: {}", self.peer_id);
        info!("🔗 Listening on port: {}", self.config.listen_port);
        
        // Start message processing task
        let message_tx = self.message_tx.clone();
        let outgoing_tx = self.outgoing_tx.clone();
        let peer_id = self.peer_id.clone();
        
        tokio::spawn(async move {
            Self::message_processor(message_tx, outgoing_tx, peer_id).await;
        });
        
        // Start peer discovery
        self.start_peer_discovery().await?;
        
        // Start ping task
        self.start_ping_task().await;
        
        info!("✅ Network manager started");
        Ok(())
    }
    
    /// Process incoming messages
    async fn message_processor(
        message_tx: broadcast::Sender<NetworkMessage>,
        outgoing_tx: mpsc::UnboundedSender<(String, NetworkMessage)>,
        peer_id: String,
    ) {
        // This would be connected to actual libp2p or TCP networking
        // For now, it's a placeholder that shows the message flow
        
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            // Process network messages here
        }
    }
    
    /// Start peer discovery process
    async fn start_peer_discovery(&mut self) -> Result<()> {
        info!("🔍 Starting peer discovery...");
        
        // Connect to bootstrap peers
        for bootstrap_peer in &self.config.bootstrap_peers.clone() {
            if let Err(e) = self.connect_to_peer(bootstrap_peer).await {
                warn!("Failed to connect to bootstrap peer {}: {}", bootstrap_peer, e);
            }
        }
        
        // Broadcast our presence
        let discovery_msg = NetworkMessage::PeerDiscovery {
            peer_id: self.peer_id.clone(),
            address: "127.0.0.1".to_string(), // Would use actual IP
            port: self.config.listen_port,
        };
        
        self.broadcast_message(discovery_msg).await?;
        
        Ok(())
    }
    
    /// Connect to a specific peer
    async fn connect_to_peer(&mut self, peer_address: &str) -> Result<()> {
        debug!("Connecting to peer: {}", peer_address);
        
        // Parse address (simplified)
        let parts: Vec<&str> = peer_address.split(':').collect();
        if parts.len() != 2 {
            return Err(QoraNetError::NetworkError("Invalid peer address format".to_string()));
        }
        
        let address = parts[0].to_string();
        let port: u16 = parts[1].parse()
            .map_err(|_| QoraNetError::NetworkError("Invalid port number".to_string()))?;
        
        let peer_id = format!("peer-{}-{}", address, port);
        
        let peer_info = PeerInfo {
            peer_id: peer_id.clone(),
            address,
            port,
            last_seen: SystemTime::now(),
            validator_address: None,
            stake: 0,
            apps_count: 0,
            ping_ms: None,
            connection_status: ConnectionStatus::Connecting,
        };
        
        self.peers.insert(peer_id.clone(), peer_info);
        
        // In a real implementation, this would establish a TCP/libp2p connection
        info!("📡 Connected to peer: {}", peer_id);
        
        Ok(())
    }
    
    /// Start periodic ping task
    async fn start_ping_task(&self) {
        let peers = self.peers.clone();
        let ping_interval = self.config.ping_interval;
        let message_tx = self.message_tx.clone();
        let peer_id = self.peer_id.clone();
        
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(ping_interval);
            
            loop {
                interval.tick().await;
                
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as u64;
                
                let ping_msg = NetworkMessage::Ping {
                    timestamp,
                    peer_id: peer_id.clone(),
                };
                
                if let Err(e) = message_tx.send(ping_msg) {
                    warn!("Failed to send ping: {}", e);
                }
            }
        });
    }
    
    /// Broadcast message to all peers
    pub async fn broadcast_message(&self, message: NetworkMessage) -> Result<()> {
        debug!("Broadcasting message: {:?}", message);
        
        for peer_id in self.peers.keys() {
            if let Err(e) = self.outgoing_tx.send((peer_id.clone(), message.clone())) {
                warn!("Failed to queue message for peer {}: {}", peer_id, e);
            }
        }
        
        Ok(())
    }
    
    /// Send message to specific peer
    pub async fn send_to_peer(&self, peer_id: &str, message: NetworkMessage) -> Result<()> {
        debug!("Sending message to peer {}: {:?}", peer_id, message);
        
        if !self.peers.contains_key(peer_id) {
            return Err(QoraNetError::NetworkError(format!("Peer not found: {}", peer_id)));
        }
        
        if let Err(e) = self.outgoing_tx.send((peer_id.to_string(), message)) {
            return Err(QoraNetError::NetworkError(format!("Failed to send message: {}", e)));
        }
        
        Ok(())
    }
    
    /// Handle incoming transaction
    pub async fn handle_new_transaction(&mut self, transaction: Transaction) -> Result<()> {
        info!("📥 Received new transaction: {}", transaction.hash());
        
        // Validate transaction
        // In a real implementation, this would be more comprehensive
        transaction.verify_signature()?;
        
        // Broadcast to other peers
        let msg = NetworkMessage::NewTransaction(transaction);
        self.broadcast_message(msg).await?;
        
        Ok(())
    }
    
    /// Handle incoming block
    pub async fn handle_new_block(&mut self, block: Block) -> Result<()> {
        info!("📥 Received new block #{}: {}", block.header.height, block.hash());
        
        // Basic validation
        // In a real implementation, this would be more comprehensive
        let expected_height = 0; // Would get from local blockchain
        let expected_previous = Hash::zero(); // Would get from local blockchain
        block.validate(expected_height, &expected_previous)?;
        
        // Broadcast to other peers (excluding sender)
        let msg = NetworkMessage::NewBlock(block);
        self.broadcast_message(msg).await?;
        
        Ok(())
    }
    
    /// Handle peer discovery message
    pub async fn handle_peer_discovery(&mut self, peer_id: String, address: String, port: u16) -> Result<()> {
        if peer_id == self.peer_id {
            return Ok((); // Ignore our own discovery message
        }
        
        info!("🔍 Discovered peer: {} at {}:{}", peer_id, address, port);
        
        let peer_info = PeerInfo {
            peer_id: peer_id.clone(),
            address,
            port,
            last_seen: SystemTime::now(),
            validator_address: None,
            stake: 0,
            apps_count: 0,
            ping_ms: None,
            connection_status: ConnectionStatus::Connected,
        };
        
        self.peers.insert(peer_id, peer_info);
        
        Ok(())
    }
    
    /// Handle validator announcement
    pub async fn handle_validator_announcement(&mut self, validator: Address, stake: u64, apps_count: u32) -> Result<()> {
        info!("👤 Validator announcement: {} with {} QOR stake, {} apps", 
            validator, 
            crate::Balance::new(stake), 
            apps_count
        );
        
        // Find peer and update validator info
        for peer in self.peers.values_mut() {
            if peer.validator_address.as_ref() == Some(&validator) {
                peer.stake = stake;
                peer.apps_count = apps_count;
                peer.last_seen = SystemTime::now();
                break;
            }
        }
        
        Ok(())
    }
    
    /// Get network statistics
    pub fn get_network_stats(&self) -> NetworkStats {
        let connected_peers = self.peers.values()
            .filter(|p| matches!(p.connection_status, ConnectionStatus::Connected))
            .count();
        
        let total_stake: u64 = self.peers.values().map(|p| p.stake).sum();
        let total_apps: u32 = self.peers.values().map(|p| p.apps_count).sum();
        
        let avg_ping = {
            let pings: Vec<u64> = self.peers.values()
                .filter_map(|p| p.ping_ms)
                .collect();
            
            if pings.is_empty() {
                None
            } else {
                Some(pings.iter().sum::<u64>() / pings.len() as u64)
            }
        };
        
        NetworkStats {
            peer_id: self.peer_id.clone(),
            connected_peers,
            total_peers: self.peers.len(),
            total_stake,
            total_apps,
            average_ping_ms: avg_ping,
        }
    }
    
    /// Get list of connected peers
    pub fn get_peers(&self) -> Vec<&PeerInfo> {
        self.peers.values().collect()
    }
    
    /// Subscribe to network messages
    pub fn subscribe(&self) -> broadcast::Receiver<NetworkMessage> {
        self.message_tx.subscribe()
    }
}

/// Network statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStats {
    pub peer_id: String,
    pub connected_peers: usize,
    pub total_peers: usize,
    pub total_stake: u64,
    pub total_apps: u32,
    pub average_ping_ms: Option<u64>,
}
