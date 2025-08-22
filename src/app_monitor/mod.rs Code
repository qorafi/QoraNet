use crate::{Result, QoraNetError, AppMetrics, Address};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::{Command, Stdio};
use sysinfo::{System, SystemExt, ProcessExt, Pid};
use tokio::time::{Duration, interval, Instant};

/// Application monitoring service
#[derive(Debug)]
pub struct AppMonitor {
    /// Currently monitored applications
    monitored_apps: HashMap<String, MonitoredApp>,
    
    /// System information collector
    system: System,
    
    /// Owner address for this monitor
    owner: Address,
    
    /// Monitoring interval
    monitor_interval: Duration,
}

/// Information about a monitored application
#[derive(Debug, Clone)]
pub struct MonitoredApp {
    pub app_id: String,
    pub app_type: AppType,
    pub process_id: Option<u32>,
    pub command: String,
    pub args: Vec<String>,
    pub start_time: Instant,
    pub metrics: AppMetrics,
    pub resource_requirements: ResourceRequirements,
    pub status: AppStatus,
    pub last_health_check: Instant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppType {
    StorageNode {
        storage_path: String,
        max_storage_gb: u32,
    },
    OracleService {
        data_sources: Vec<String>,
        update_interval_sec: u32,
    },
    ComputeNode {
        supported_frameworks: Vec<String>,
    },
    IndexingService {
        indexed_chains: Vec<String>,
    },
    RelayNode {
        supported_protocols: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceRequirements {
    pub min_cpu_cores: u32,
    pub min_memory_gb: u32,
    pub min_disk_gb: u32,
    pub min_bandwidth_mbps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AppStatus {
    Starting,
    Running,
    Stopped,
    Error(String),
    HealthCheckFailed,
}

impl AppMonitor {
    pub fn new(owner: Address) -> Self {
        Self {
            monitored_apps: HashMap::new(),
            system: System::new_all(),
            owner,
            monitor_interval: Duration::from_secs(30), // Monitor every 30 seconds
        }
    }
    
    /// Register a new application for monitoring
    pub fn register_app(
        &mut self,
        app_id: String,
        app_type: AppType,
        command: String,
        args: Vec<String>,
        resource_requirements: ResourceRequirements,
    ) -> Result<()> {
        // Validate system meets requirements
        self.validate_system_requirements(&resource_requirements)?;
        
        let monitored_app = MonitoredApp {
            app_id: app_id.clone(),
            app_type,
            process_id: None,
            command,
            args,
            start_time: Instant::now(),
            metrics: AppMetrics::new(),
            resource_requirements,
            status: AppStatus::Starting,
            last_health_check: Instant::now(),
        };
        
        self.monitored_apps.insert(app_id, monitored_app);
        Ok(())
    }
    
    /// Start monitoring all registered applications
    pub async fn start_monitoring(&mut self) -> Result<()> {
        // Start all applications
        for app in self.monitored_apps.values_mut() {
            self.start_app(app)?;
        }
        
        // Start monitoring loop
        let mut monitor_timer = interval(self.monitor_interval);
        
        loop {
            monitor_timer.tick().await;
            self.update_all_metrics().await?;
        }
    }
    
    /// Start a specific application
    fn start_app(&self, app: &mut MonitoredApp) -> Result<()> {
        tracing::info!("Starting app: {}", app.app_id);
        
        let mut cmd = Command::new(&app.command);
        cmd.args(&app.args)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());
        
        match cmd.spawn() {
            Ok(child) => {
                app.process_id = Some(child.id());
                app.status = AppStatus::Running;
                tracing::info!("Started app {} with PID {}", app.app_id, child.id());
                Ok(())
            },
            Err(e) => {
                app.status = AppStatus::Error(format!("Failed to start: {}", e));
                Err(QoraNetError::AppMonitorError(format!("Failed to start app {}: {}", app.app_id, e)))
            }
        }
    }
    
    /// Update metrics for all applications
    async fn update_all_metrics(&mut self) -> Result<()> {
        self.system.refresh_all();
        
        for app in self.monitored_apps.values_mut() {
            self.update_app_metrics(app).await?;
        }
        
        Ok(())
    }
    
    /// Update metrics for a specific application
    async fn update_app_metrics(&mut self, app: &mut MonitoredApp) -> Result<()> {
        if let Some(pid) = app.process_id {
            // Check if process is still running
            if let Some(process) = self.system.process(Pid::from(pid as usize)) {
                // Update CPU usage
                app.metrics.cpu_usage = process.cpu_usage() as f64;
                
                // Update memory usage
                app.metrics.memory_usage = process.memory() * 1024; // Convert from KB to bytes
                
                // Update uptime
                app.metrics.uptime = app.start_time.elapsed().as_secs();
                
                // Perform health check
                self.perform_health_check(app).await?;
                
                // Update timestamp
                app.metrics.last_updated = chrono::Utc::now().timestamp() as u64;
                
                tracing::debug!("Updated metrics for {}: CPU {:.2}%, Memory {} MB", 
                    app.app_id, 
                    app.metrics.cpu_usage, 
                    app.metrics.memory_usage / (1024 * 1024)
                );
            } else {
                // Process not found, mark as stopped
                app.status = AppStatus::Stopped;
                app.process_id = None;
                tracing::warn!("App {} process not found, marked as stopped", app.app_id);
            }
        }
        
        Ok(())
    }
    
    /// Perform health check on application
    async fn perform_health_check(&mut self, app: &mut MonitoredApp) -> Result<()> {
        let health_result = match &app.app_type {
            AppType::StorageNode { storage_path, .. } => {
                self.check_storage_node_health(storage_path).await
            },
            AppType::OracleService { .. } => {
                self.check_oracle_health().await
            },
            AppType::ComputeNode { .. } => {
                self.check_compute_node_health().await
            },
            AppType::IndexingService { .. } => {
                self.check_indexing_service_health().await
            },
            AppType::RelayNode { .. } => {
                self.check_relay_node_health().await
            },
        };
        
        match health_result {
            Ok(requests_served) => {
                app.metrics.requests_served += requests_served;
                app.last_health_check = Instant::now();
                if matches!(app.status, AppStatus::HealthCheckFailed) {
                    app.status = AppStatus::Running;
                }
            },
            Err(e) => {
                app.status = AppStatus::HealthCheckFailed;
                tracing::warn!("Health check failed for {}: {}", app.app_id, e);
            }
        }
        
        Ok(())
    }
    
    /// Health check for storage node
    async fn check_storage_node_health(&self, storage_path: &str) -> Result<u64> {
        // Check if storage path exists and is writable
        use std::path::Path;
        if !Path::new(storage_path).exists() {
            return Err(QoraNetError::AppMonitorError("Storage path does not exist".to_string()));
        }
        
        // Mock: return number of files served (in real implementation, query the storage service)
        Ok(10)
    }
    
    /// Health check for oracle service
    async fn check_oracle_health(&self) -> Result<u64> {
        // Mock: check if oracle is responding (in real implementation, query oracle API)
        Ok(5)
    }
    
    /// Health check for compute node
    async fn check_compute_node_health(&self) -> Result<u64> {
        // Mock: check compute jobs processed
        Ok(3)
    }
    
    /// Health check for indexing service
    async fn check_indexing_service_health(&self) -> Result<u64> {
        // Mock: check indexing operations
        Ok(20)
    }
    
    /// Health check for relay node
    async fn check_relay_node_health(&self) -> Result<u64> {
        // Mock: check messages relayed
        Ok(15)
    }
    
    /// Validate system meets application requirements
    fn validate_system_requirements(&mut self, requirements: &ResourceRequirements) -> Result<()> {
        self.system.refresh_all();
        
        // Check CPU cores
        if self.system.physical_core_count().unwrap_or(0) < requirements.min_cpu_cores as usize {
            return Err(QoraNetError::AppMonitorError(
                format!("Insufficient CPU cores: need {}, have {}", 
                    requirements.min_cpu_cores, 
                    self.system.physical_core_count().unwrap_or(0))
            ));
        }
        
        // Check memory (convert from bytes to GB)
        let total_memory_gb = self.system.total_memory() / (1024 * 1024 * 1024);
        if total_memory_gb < requirements.min_memory_gb as u64 {
            return Err(QoraNetError::AppMonitorError(
                format!("Insufficient memory: need {}GB, have {}GB", 
                    requirements.min_memory_gb, 
                    total_memory_gb)
            ));
        }
        
        Ok(())
    }
    
    /// Get metrics for a specific application
    pub fn get_app_metrics(&self, app_id: &str) -> Option<&AppMetrics> {
        self.monitored_apps.get(app_id).map(|app| &app.metrics)
    }
    
    /// Get all application metrics
    pub fn get_all_metrics(&self) -> HashMap<String, AppMetrics> {
        self.monitored_apps
            .iter()
            .map(|(id, app)| (id.clone(), app.metrics.clone()))
            .collect()
    }
    
    /// Stop monitoring a specific application
    pub fn stop_app(&mut self, app_id: &str) -> Result<()> {
        if let Some(app) = self.monitored_apps.get_mut(app_id) {
            if let Some(pid) = app.process_id {
                // Try to terminate the process gracefully
                if let Some(process) = self.system.process(Pid::from(pid as usize)) {
                    process.kill();
                    app.status = AppStatus::Stopped;
                    app.process_id = None;
                    tracing::info!("Stopped app: {}", app_id);
                }
            }
        }
        Ok(())
    }
    
    /// Remove application from monitoring
    pub fn unregister_app(&mut self, app_id: &str) -> Result<()> {
        // Stop the app first
        self.stop_app(app_id)?;
        
        // Remove from monitoring
        self.monitored_apps.remove(app_id);
        tracing::info!("Unregistered app: {}", app_id);
        Ok(())
    }
    
    /// Get system resource utilization
    pub fn get_system_stats(&mut self) -> SystemStats {
        self.system.refresh_all();
        
        SystemStats {
            cpu_usage: self.system.global_cpu_info().cpu_usage() as f64,
            total_memory: self.system.total_memory(),
            used_memory: self.system.used_memory(),
            total_disk: 0, // Would need additional system info
            active_apps: self.monitored_apps.len(),
            running_apps: self.monitored_apps.values()
                .filter(|app| matches!(app.status, AppStatus::Running))
                .count(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemStats {
    pub cpu_usage: f64,
    pub total_memory: u64,
    pub used_memory: u64,
    pub total_disk: u64,
    pub active_apps: usize,
    pub running_apps: usize,
}
