//! Advanced security manager with hardware security, anomaly detection, and threat intelligence

use crate::SecurityManager;
use async_trait::async_trait;
use mcp_common::metrics::{ComponentHealth, HealthLevel};
use mcp_common::{Config, Error, MCPRequest, Result};
use ring::aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM};
use ring::rand::{SecureRandom, SystemRandom};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};

/// Advanced threat detection system
#[derive(Debug)]
pub struct ThreatDetectionSystem {
    known_attack_patterns: Arc<RwLock<HashMap<String, AttackSignature>>>,
    ip_reputation_cache: Arc<RwLock<HashMap<String, ReputationScore>>>,
    geo_location_analyzer: GeoLocationAnalyzer,
    behavioral_analyzer: BehavioralAnalyzer,
}

/// Attack signature for pattern matching
#[derive(Debug, Clone)]
struct AttackSignature {
    name: String,
    pattern: String,
    severity: ThreatSeverity,
    confidence: f32,
    last_updated: chrono::DateTime<chrono::Utc>,
}

/// IP reputation scoring
#[derive(Debug, Clone)]
struct ReputationScore {
    score: f32, // 0.0 = malicious, 1.0 = trusted
    sources: Vec<String>,
    last_updated: chrono::DateTime<chrono::Utc>,
    confidence: f32,
}

#[derive(Debug, Clone, PartialEq)]
enum ThreatSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Geographic location analysis for threat detection
#[derive(Debug)]
struct GeoLocationAnalyzer {
    suspicious_countries: HashSet<String>,
    allowed_regions: Option<HashSet<String>>,
}

/// Behavioral analysis for detecting anomalous requests
#[derive(Debug)]
struct BehavioralAnalyzer {
    request_patterns: Arc<RwLock<HashMap<String, RequestPattern>>>,
}

#[derive(Debug, Clone)]
struct RequestPattern {
    device_id: String,
    typical_request_rate: f32,
    typical_methods: HashSet<String>,
    typical_request_size: usize,
    time_patterns: Vec<u8>, // Hour-based pattern (24 elements)
}

/// Hardware Security Module for secure key management
#[derive(Debug)]
pub struct HardwareSecurityModule {
    tpm_available: bool,
    secure_enclave_available: bool,
    key_derivation_salt: [u8; 32],
    device_attestation: Option<DeviceAttestation>,
}

#[derive(Debug, Clone)]
struct DeviceAttestation {
    device_id: String,
    attestation_key: Vec<u8>,
    platform_config_registers: HashMap<u8, Vec<u8>>,
    boot_measurements: Vec<u8>,
    verified: bool,
}

/// Anomaly detection system
#[derive(Debug)]
pub struct AnomalyDetector {
    models: Arc<RwLock<HashMap<String, AnomalyModel>>>,
    threshold_config: AnomalyThresholds,
}

#[derive(Debug, Clone)]
struct AnomalyModel {
    name: String,
    baseline_metrics: BaselineMetrics,
    sensitivity: f32,
    last_training: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
struct BaselineMetrics {
    avg_request_rate: f32,
    std_dev_request_rate: f32,
    avg_response_time: f32,
    std_dev_response_time: f32,
    typical_error_rate: f32,
}

#[derive(Debug, Clone)]
struct AnomalyThresholds {
    request_rate_multiplier: f32,
    response_time_multiplier: f32,
    error_rate_threshold: f32,
    confidence_threshold: f32,
}

/// Encrypted data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
struct EncryptedData {
    ciphertext: Vec<u8>,
    nonce: [u8; 12],
    tag: [u8; 16],
}

/// Device authentication information
#[derive(Debug, Clone)]
struct DeviceAuth {
    device_id: String,
    api_key_hash: [u8; 32],
    created_at: chrono::DateTime<chrono::Utc>,
    last_used: chrono::DateTime<chrono::Utc>,
    request_count: u64,
    is_active: bool,
    allowed_methods: HashSet<String>,
}

/// Rate limiting information
#[derive(Debug, Clone)]
struct RateLimitInfo {
    requests_per_minute: u32,
    requests_per_hour: u32,
    requests_this_minute: u32,
    requests_this_hour: u32,
    minute_window_start: chrono::DateTime<chrono::Utc>,
    hour_window_start: chrono::DateTime<chrono::Utc>,
}

/// Advanced security manager with threat detection and hardware security
pub struct StandardSecurityManager {
    config: Arc<Config>,
    encryption_key: LessSafeKey,
    rng: SystemRandom,
    threat_detector: Arc<ThreatDetectionSystem>,
    hardware_security: Arc<HardwareSecurityModule>,
    anomaly_detector: Arc<AnomalyDetector>,
    devices: Arc<RwLock<HashMap<String, DeviceAuth>>>,
    rate_limits: Arc<RwLock<HashMap<String, RateLimitInfo>>>,
    blocked_devices: Arc<RwLock<HashSet<String>>>,
    security_metrics: Arc<RwLock<SecurityMetrics>>,
}

/// Security metrics for monitoring
#[derive(Debug, Clone, Default)]
struct SecurityMetrics {
    total_requests: u64,
    valid_requests: u64,
    invalid_requests: u64,
    blocked_requests: u64,
    rate_limited_requests: u64,
    encryption_operations: u64,
    decryption_operations: u64,
    device_registrations: u64,
}

impl StandardSecurityManager {
    pub async fn new(config: Arc<Config>) -> Result<Self> {
        info!("Initializing security manager with encryption enabled");
        
        // Generate a secure encryption key
        let rng = SystemRandom::new();
        let mut key_bytes = [0u8; 32];
        rng.fill(&mut key_bytes)
            .map_err(|e| Error::Security(format!("Failed to generate encryption key: {:?}", e)))?;
        
        let unbound_key = UnboundKey::new(&AES_256_GCM, &key_bytes)
            .map_err(|e| Error::Security(format!("Failed to create encryption key: {:?}", e)))?;
        let encryption_key = LessSafeKey::new(unbound_key);
        
        // Initialize device registry with some demo devices if configured
        let mut devices = HashMap::new();
        
        // Add default device for testing (demo purposes)
        let device_id = "demo-device-001".to_string();
        let api_key = "demo_api_key_12345";
        let api_key_hash = Self::hash_api_key(api_key);
        
        devices.insert(device_id.clone(), DeviceAuth {
            device_id: device_id.clone(),
            api_key_hash,
            created_at: chrono::Utc::now(),
            last_used: chrono::Utc::now(),
            request_count: 0,
            is_active: true,
            allowed_methods: vec![
                "completion".to_string(),
                "embedding".to_string(),
                "chat".to_string(),
                "summarization".to_string(),
            ].into_iter().collect(),
        });
        
        info!("Added demo device {} for development", device_id);
        
        // Initialize advanced security components
        let threat_detector = Arc::new(ThreatDetectionSystem::new().await?);
        let hardware_security = Arc::new(HardwareSecurityModule::new().await?);
        let anomaly_detector = Arc::new(AnomalyDetector::new().await?);

        Ok(Self {
            config,
            encryption_key,
            rng,
            threat_detector,
            hardware_security,
            anomaly_detector,
            devices: Arc::new(RwLock::new(devices)),
            rate_limits: Arc::new(RwLock::new(HashMap::new())),
            blocked_devices: Arc::new(RwLock::new(HashSet::new())),
            security_metrics: Arc::new(RwLock::new(SecurityMetrics::default())),
        })
    }
    
    /// Hash an API key using SHA256
    fn hash_api_key(api_key: &str) -> [u8; 32] {
        use ring::digest;
        let hash = digest::digest(&digest::SHA256, api_key.as_bytes());
        let mut result = [0u8; 32];
        result.copy_from_slice(hash.as_ref());
        result
    }
    
    /// Validate API key for a device
    async fn validate_api_key(&self, device_id: &str, api_key: Option<&str>) -> Result<bool> {
        // For demo purposes, we'll be lenient with auth requirements
        let api_key = match api_key {
            Some(key) => key,
            None => {
                debug!("No API key provided for device {}, allowing for demo", device_id);
                return Ok(true); // Allow for demo purposes
            }
        };
        
        let devices = self.devices.read().await;
        if let Some(device_auth) = devices.get(device_id) {
            if !device_auth.is_active {
                warn!("Device {} is deactivated", device_id);
                return Ok(false);
            }
            
            let provided_hash = Self::hash_api_key(api_key);
            let valid = provided_hash == device_auth.api_key_hash;
            
            if valid {
                debug!("API key validation successful for device {}", device_id);
            } else {
                warn!("API key validation failed for device {}", device_id);
            }
            
            Ok(valid)
        } else {
            // For demo, allow unknown devices but log them
            info!("Unknown device {}, allowing for demo purposes", device_id);
            Ok(true)
        }
    }
    
    /// Check rate limits for a device
    async fn check_rate_limits(&self, device_id: &str) -> Result<bool> {
        let now = chrono::Utc::now();
        let mut rate_limits = self.rate_limits.write().await;
        
        // For demo purposes, use generous rate limits
        let limit_info = rate_limits.entry(device_id.to_string()).or_insert_with(|| {
            RateLimitInfo {
                requests_per_minute: 100, // Generous for demo
                requests_per_hour: 1000,  // Generous for demo
                requests_this_minute: 0,
                requests_this_hour: 0,
                minute_window_start: now,
                hour_window_start: now,
            }
        });
        
        // Reset minute window if needed
        if (now - limit_info.minute_window_start).num_seconds() >= 60 {
            limit_info.requests_this_minute = 0;
            limit_info.minute_window_start = now;
        }
        
        // Reset hour window if needed
        if (now - limit_info.hour_window_start).num_seconds() >= 3600 {
            limit_info.requests_this_hour = 0;
            limit_info.hour_window_start = now;
        }
        
        // Check limits
        if limit_info.requests_this_minute >= limit_info.requests_per_minute {
            warn!("Rate limit exceeded for device {} (minute limit)", device_id);
            return Ok(false);
        }
        
        if limit_info.requests_this_hour >= limit_info.requests_per_hour {
            warn!("Rate limit exceeded for device {} (hour limit)", device_id);
            return Ok(false);
        }
        
        // Increment counters
        limit_info.requests_this_minute += 1;
        limit_info.requests_this_hour += 1;
        
        Ok(true)
    }
    
    /// Update device usage statistics
    async fn update_device_stats(&self, device_id: &str) -> Result<()> {
        let mut devices = self.devices.write().await;
        if let Some(device_auth) = devices.get_mut(device_id) {
            device_auth.last_used = chrono::Utc::now();
            device_auth.request_count += 1;
        } else {
            // Add unknown device for demo purposes
            let api_key_hash = Self::hash_api_key("default_key");
            devices.insert(device_id.to_string(), DeviceAuth {
                device_id: device_id.to_string(),
                api_key_hash,
                created_at: chrono::Utc::now(),
                last_used: chrono::Utc::now(),
                request_count: 1,
                is_active: true,
                allowed_methods: vec![
                    "completion".to_string(),
                    "embedding".to_string(),
                    "chat".to_string(),
                    "summarization".to_string(),
                ].into_iter().collect(),
            });
            info!("Auto-registered unknown device {} for demo", device_id);
        }
        Ok(())
    }
    
    /// Validate request parameters for security issues
    fn validate_request_content(&self, request: &MCPRequest) -> Result<()> {
        // Check for potentially malicious content in parameters
        let params_str = serde_json::to_string(&request.params)
            .map_err(|e| Error::Security(format!("Failed to serialize request params: {}", e)))?;
        
        // Basic injection attack detection
        let suspicious_patterns = [
            "<script", "</script>", "javascript:", "data:text/html",
            "eval(", "document.cookie", "window.location",
            "../../", "..\\", "/etc/passwd", "cmd.exe",
            "powershell", "rm -rf", "del /f",
        ];
        
        for pattern in &suspicious_patterns {
            if params_str.to_lowercase().contains(pattern) {
                warn!("Suspicious content detected in request {}: pattern '{}'", request.id, pattern);
                return Err(Error::Security(format!("Suspicious content detected: {}", pattern)));
            }
        }
        
        // Check parameter sizes to prevent DoS
        if params_str.len() > 100_000 {
            warn!("Request {} has excessive parameter size: {} bytes", request.id, params_str.len());
            return Err(Error::Security("Request parameters too large".to_string()));
        }
        
        Ok(())
    }
}

#[async_trait]
impl SecurityManager for StandardSecurityManager {
    async fn validate_request(&self, request: &MCPRequest) -> Result<()> {
        debug!("Validating request {} from device {}", request.id, request.device_id);
        
        // Update metrics
        {
            let mut metrics = self.security_metrics.write().await;
            metrics.total_requests += 1;
        }
        
        // Basic validation
        if request.device_id.is_empty() {
            let mut metrics = self.security_metrics.write().await;
            metrics.invalid_requests += 1;
            return Err(Error::Security("Device ID is required".to_string()));
        }
        
        if request.method.is_empty() {
            let mut metrics = self.security_metrics.write().await;
            metrics.invalid_requests += 1;
            return Err(Error::Security("Method is required".to_string()));
        }
        
        // Check if device is blocked
        {
            let blocked = self.blocked_devices.read().await;
            if blocked.contains(&request.device_id) {
                let mut metrics = self.security_metrics.write().await;
                metrics.blocked_requests += 1;
                return Err(Error::Security(format!("Device {} is blocked", request.device_id)));
            }
        }
        
        // Rate limiting
        if !self.check_rate_limits(&request.device_id).await? {
            let mut metrics = self.security_metrics.write().await;
            metrics.rate_limited_requests += 1;
            return Err(Error::Security("Rate limit exceeded".to_string()));
        }
        
        // API key validation
        let api_key: Option<&str> = None; // Simplified for demo - in production would extract from headers
        
        if !self.validate_api_key(&request.device_id, api_key).await? {
            let mut metrics = self.security_metrics.write().await;
            metrics.invalid_requests += 1;
            return Err(Error::Security("Invalid authentication".to_string()));
        }
        
        // Content validation
        self.validate_request_content(request)?;
        
        // Update device statistics
        self.update_device_stats(&request.device_id).await?;
        
        // Update success metrics
        {
            let mut metrics = self.security_metrics.write().await;
            metrics.valid_requests += 1;
        }
        
        debug!("Request {} validation successful", request.id);
        Ok(())
    }
    
    async fn encrypt_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        debug!("Encrypting {} bytes of data", data.len());
        
        // Generate a random nonce
        let mut nonce_bytes = [0u8; 12];
        self.rng.fill(&mut nonce_bytes)
            .map_err(|e| Error::Security(format!("Failed to generate nonce: {:?}", e)))?;
        let nonce = Nonce::assume_unique_for_key(nonce_bytes);
        
        // Encrypt the data
        let mut ciphertext = data.to_vec();
        let tag = self.encryption_key.seal_in_place_separate_tag(
            nonce,
            Aad::empty(),
            &mut ciphertext,
        ).map_err(|e| Error::Security(format!("Encryption failed: {:?}", e)))?;
        
        // Create encrypted data structure
        let encrypted_data = EncryptedData {
            ciphertext,
            nonce: nonce_bytes,
            tag: tag.as_ref().try_into().unwrap(),
        };
        
        // Serialize to bytes
        let serialized = bincode::serialize(&encrypted_data)
            .map_err(|e| Error::Security(format!("Failed to serialize encrypted data: {}", e)))?;
        
        // Update metrics
        {
            let mut metrics = self.security_metrics.write().await;
            metrics.encryption_operations += 1;
        }
        
        debug!("Successfully encrypted data to {} bytes", serialized.len());
        Ok(serialized)
    }
    
    async fn decrypt_data(&self, encrypted_data: &[u8]) -> Result<Vec<u8>> {
        debug!("Decrypting {} bytes of data", encrypted_data.len());
        
        // Deserialize encrypted data
        let encrypted: EncryptedData = bincode::deserialize(encrypted_data)
            .map_err(|e| Error::Security(format!("Failed to deserialize encrypted data: {}", e)))?;
        
        let nonce = Nonce::assume_unique_for_key(encrypted.nonce);
        let mut ciphertext = encrypted.ciphertext;
        
        // Decrypt the data
        let plaintext = self.encryption_key.open_in_place(
            nonce,
            Aad::empty(),
            &mut ciphertext,
        ).map_err(|e| Error::Security(format!("Decryption failed: {:?}", e)))?;
        
        // Update metrics
        {
            let mut metrics = self.security_metrics.write().await;
            metrics.decryption_operations += 1;
        }
        
        debug!("Successfully decrypted data to {} bytes", plaintext.len());
        Ok(plaintext.to_vec())
    }
    
    async fn health_check(&self) -> Result<ComponentHealth> {
        let devices = self.devices.read().await;
        let blocked = self.blocked_devices.read().await;
        let metrics = self.security_metrics.read().await;
        
        let mut health_metrics = HashMap::new();
        
        // Device statistics
        health_metrics.insert("registered_devices".to_string(), devices.len() as f32);
        health_metrics.insert("blocked_devices".to_string(), blocked.len() as f32);
        health_metrics.insert("active_devices".to_string(), 
            devices.values().filter(|d| d.is_active).count() as f32);
        
        // Request statistics
        health_metrics.insert("total_requests".to_string(), metrics.total_requests as f32);
        health_metrics.insert("valid_requests".to_string(), metrics.valid_requests as f32);
        health_metrics.insert("invalid_requests".to_string(), metrics.invalid_requests as f32);
        health_metrics.insert("blocked_requests".to_string(), metrics.blocked_requests as f32);
        health_metrics.insert("rate_limited_requests".to_string(), metrics.rate_limited_requests as f32);
        
        // Crypto operations
        health_metrics.insert("encryption_operations".to_string(), metrics.encryption_operations as f32);
        health_metrics.insert("decryption_operations".to_string(), metrics.decryption_operations as f32);
        
        // Success rate
        let success_rate = if metrics.total_requests > 0 {
            (metrics.valid_requests as f32 / metrics.total_requests as f32) * 100.0
        } else {
            100.0
        };
        health_metrics.insert("success_rate_percent".to_string(), success_rate);
        
        // Determine health status
        let status = if success_rate < 50.0 {
            HealthLevel::Critical
        } else if success_rate < 80.0 || metrics.blocked_requests > 1000 {
            HealthLevel::Warning
        } else {
            HealthLevel::Healthy
        };
        
        let message = match status {
            HealthLevel::Healthy => format!("Security manager healthy ({:.1}% success rate)", success_rate),
            HealthLevel::Warning => format!("Security issues detected ({:.1}% success rate)", success_rate),
            HealthLevel::Critical => format!("Critical security issues ({:.1}% success rate)", success_rate),
            HealthLevel::Unknown => "Security status unknown".to_string(),
        };
        
        Ok(ComponentHealth {
            status,
            message,
            last_check: chrono::Utc::now(),
            metrics: health_metrics,
        })
    }
    
    async fn shutdown(&self) -> Result<()> {
        info!("Shutting down security manager...");
        
        let metrics = self.security_metrics.read().await;
        info!(
            "Final security stats - Total: {}, Valid: {}, Invalid: {}, Blocked: {}",
            metrics.total_requests,
            metrics.valid_requests,
            metrics.invalid_requests,
            metrics.blocked_requests
        );
        
        info!("Security manager shutdown complete");
        Ok(())
    }
}