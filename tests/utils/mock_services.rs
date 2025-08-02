// Mock services for testing MCP WASM Edge Gateway
// Provides mocked external dependencies for isolated testing

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::sleep;
use serde_json::{json, Value};

/// Mock cloud MCP service
pub struct MockCloudService {
    pub latency_ms: u64,
    pub success_rate: f64,
    pub request_count: Arc<Mutex<usize>>,
    pub responses: Arc<Mutex<HashMap<String, Value>>>,
}

impl MockCloudService {
    pub fn new(latency_ms: u64, success_rate: f64) -> Self {
        Self {
            latency_ms,
            success_rate,
            request_count: Arc::new(Mutex::new(0)),
            responses: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    pub async fn handle_request(&self, request: Value) -> Result<Value, String> {
        // Increment request counter
        {
            let mut count = self.request_count.lock().unwrap();
            *count += 1;
        }
        
        // Simulate latency
        sleep(Duration::from_millis(self.latency_ms)).await;
        
        // Simulate failures based on success rate
        if fastrand::f64() > self.success_rate {
            return Err("Simulated cloud service error".to_string());
        }
        
        // Check for pre-configured responses
        if let Some(messages) = request.get("messages") {
            if let Some(last_message) = messages.as_array().and_then(|arr| arr.last()) {
                if let Some(content) = last_message.get("content").and_then(|c| c.as_str()) {
                    let responses = self.responses.lock().unwrap();
                    if let Some(response) = responses.get(content) {
                        return Ok(response.clone());
                    }
                }
            }
        }
        
        // Default response
        Ok(json!({
            "id": format!("mock-{}", fastrand::u64(..)),
            "object": "chat.completion",
            "created": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            "model": "mock-model",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "This is a mock response from the cloud service."
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 10,
                "completion_tokens": 20,
                "total_tokens": 30
            }
        }))
    }
    
    pub fn set_response(&self, trigger: &str, response: Value) {
        let mut responses = self.responses.lock().unwrap();
        responses.insert(trigger.to_string(), response);
    }
    
    pub fn get_request_count(&self) -> usize {
        *self.request_count.lock().unwrap()
    }
    
    pub fn reset_counters(&self) {
        let mut count = self.request_count.lock().unwrap();
        *count = 0;
    }
}

/// Mock hardware security module
pub struct MockHSM {
    pub keys: Arc<Mutex<HashMap<String, Vec<u8>>>>,
    pub attestation_data: Arc<Mutex<Value>>,
}

impl MockHSM {
    pub fn new() -> Self {
        Self {
            keys: Arc::new(Mutex::new(HashMap::new())),
            attestation_data: Arc::new(Mutex::new(json!({
                "device_id": "mock-device-123",
                "attestation": "mock-attestation-data",
                "timestamp": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            }))),
        }
    }
    
    pub fn generate_key(&self, key_id: &str) -> Result<Vec<u8>, String> {
        let key = (0..32).map(|_| fastrand::u8(..)).collect();
        
        let mut keys = self.keys.lock().unwrap();
        keys.insert(key_id.to_string(), key.clone());
        
        Ok(key)
    }
    
    pub fn get_key(&self, key_id: &str) -> Option<Vec<u8>> {
        let keys = self.keys.lock().unwrap();
        keys.get(key_id).cloned()
    }
    
    pub fn sign_data(&self, _key_id: &str, data: &[u8]) -> Result<Vec<u8>, String> {
        // Mock signature - just hash the data
        let signature = (0..64).map(|i| (data.len() + i) as u8).collect();
        Ok(signature)
    }
    
    pub fn verify_signature(&self, _key_id: &str, _data: &[u8], _signature: &[u8]) -> bool {
        // Mock verification - always succeeds
        true
    }
    
    pub fn get_attestation(&self) -> Value {
        self.attestation_data.lock().unwrap().clone()
    }
    
    pub fn set_attestation(&self, attestation: Value) {
        let mut data = self.attestation_data.lock().unwrap();
        *data = attestation;
    }
}

/// Mock model engine for testing
pub struct MockModelEngine {
    pub models: Arc<Mutex<HashMap<String, MockModel>>>,
    pub default_latency_ms: u64,
}

#[derive(Clone)]
pub struct MockModel {
    pub name: String,
    pub size_mb: f64,
    pub latency_ms: u64,
    pub responses: HashMap<String, String>,
    pub error_rate: f64,
}

impl MockModelEngine {
    pub fn new() -> Self {
        let mut engine = Self {
            models: Arc::new(Mutex::new(HashMap::new())),
            default_latency_ms: 100,
        };
        
        // Add default test models
        engine.add_model(MockModel {
            name: "test-model".to_string(),
            size_mb: 1.5,
            latency_ms: 50,
            responses: HashMap::new(),
            error_rate: 0.01,
        });
        
        engine.add_model(MockModel {
            name: "large-model".to_string(),
            size_mb: 5.0,
            latency_ms: 200,
            responses: HashMap::new(),
            error_rate: 0.05,
        });
        
        engine
    }
    
    pub fn add_model(&self, model: MockModel) {
        let mut models = self.models.lock().unwrap();
        models.insert(model.name.clone(), model);
    }
    
    pub async fn process_request(&self, model_name: &str, prompt: &str) -> Result<String, String> {
        let model = {
            let models = self.models.lock().unwrap();
            models.get(model_name).cloned()
                .ok_or_else(|| format!("Model {} not found", model_name))?
        };
        
        // Simulate processing time
        sleep(Duration::from_millis(model.latency_ms)).await;
        
        // Simulate errors
        if fastrand::f64() < model.error_rate {
            return Err("Simulated model processing error".to_string());
        }
        
        // Check for pre-configured responses
        if let Some(response) = model.responses.get(prompt) {
            return Ok(response.clone());
        }
        
        // Generate a mock response based on the prompt
        let response = if prompt.to_lowercase().contains("hello") {
            "Hello! How can I help you today?"
        } else if prompt.to_lowercase().contains("time") {
            "The current time is 12:00 PM (mock time)"
        } else if prompt.to_lowercase().contains("error") {
            return Err("Triggered error response".to_string());
        } else {
            "This is a mock response from the test model."
        };
        
        Ok(response.to_string())
    }
    
    pub fn get_model_info(&self, model_name: &str) -> Option<(f64, u64)> {
        let models = self.models.lock().unwrap();
        models.get(model_name).map(|m| (m.size_mb, m.latency_ms))
    }
    
    pub fn list_models(&self) -> Vec<String> {
        let models = self.models.lock().unwrap();
        models.keys().cloned().collect()
    }
}

/// Mock sensor data generator
pub struct MockSensorData {
    pub start_time: Instant,
    pub update_interval: Duration,
}

impl MockSensorData {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            update_interval: Duration::from_secs(1),
        }
    }
    
    pub fn get_temperature(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        // Generate a sine wave with some noise
        20.0 + 5.0 * (elapsed * 0.1).sin() + fastrand::f64() * 2.0 - 1.0
    }
    
    pub fn get_humidity(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        50.0 + 20.0 * (elapsed * 0.05).cos() + fastrand::f64() * 5.0 - 2.5
    }
    
    pub fn get_pressure(&self) -> f64 {
        1013.25 + fastrand::f64() * 10.0 - 5.0
    }
    
    pub fn get_all_readings(&self) -> Value {
        json!({
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            "temperature_celsius": self.get_temperature(),
            "humidity_percent": self.get_humidity(),
            "pressure_hpa": self.get_pressure(),
            "device_id": "mock-sensor-001"
        })
    }
}

/// Mock network conditions
pub struct MockNetworkConditions {
    pub latency_ms: u64,
    pub packet_loss_rate: f64,
    pub bandwidth_kbps: u64,
    pub is_connected: bool,
}

impl MockNetworkConditions {
    pub fn new() -> Self {
        Self {
            latency_ms: 10,
            packet_loss_rate: 0.0,
            bandwidth_kbps: 1000,
            is_connected: true,
        }
    }
    
    pub fn simulate_poor_network(&mut self) {
        self.latency_ms = 500;
        self.packet_loss_rate = 0.1;
        self.bandwidth_kbps = 56;
    }
    
    pub fn simulate_offline(&mut self) {
        self.is_connected = false;
    }
    
    pub fn restore_network(&mut self) {
        self.latency_ms = 10;
        self.packet_loss_rate = 0.0;
        self.bandwidth_kbps = 1000;
        self.is_connected = true;
    }
    
    pub async fn simulate_request_delay(&self) {
        if !self.is_connected {
            // Simulate timeout
            sleep(Duration::from_secs(30)).await;
            return;
        }
        
        // Simulate packet loss
        if fastrand::f64() < self.packet_loss_rate {
            sleep(Duration::from_millis(self.latency_ms * 3)).await;
            return;
        }
        
        // Normal latency
        sleep(Duration::from_millis(self.latency_ms)).await;
    }
}

/// Mock metrics collector
pub struct MockMetricsCollector {
    pub metrics: Arc<Mutex<HashMap<String, f64>>>,
    pub counters: Arc<Mutex<HashMap<String, u64>>>,
}

impl MockMetricsCollector {
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(Mutex::new(HashMap::new())),
            counters: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    pub fn record_gauge(&self, name: &str, value: f64) {
        let mut metrics = self.metrics.lock().unwrap();
        metrics.insert(name.to_string(), value);
    }
    
    pub fn increment_counter(&self, name: &str) {
        let mut counters = self.counters.lock().unwrap();
        *counters.entry(name.to_string()).or_insert(0) += 1;
    }
    
    pub fn get_metric(&self, name: &str) -> Option<f64> {
        let metrics = self.metrics.lock().unwrap();
        metrics.get(name).copied()
    }
    
    pub fn get_counter(&self, name: &str) -> u64 {
        let counters = self.counters.lock().unwrap();
        counters.get(name).copied().unwrap_or(0)
    }
    
    pub fn export_metrics(&self) -> Value {
        let metrics = self.metrics.lock().unwrap();
        let counters = self.counters.lock().unwrap();
        
        json!({
            "gauges": *metrics,
            "counters": *counters,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        })
    }
    
    pub fn reset(&self) {
        let mut metrics = self.metrics.lock().unwrap();
        let mut counters = self.counters.lock().unwrap();
        metrics.clear();
        counters.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_mock_cloud_service() {
        let service = MockCloudService::new(50, 0.9);
        
        let request = json!({
            "messages": [{"role": "user", "content": "Hello"}]
        });
        
        let result = service.handle_request(request).await;
        assert!(result.is_ok());
        assert_eq!(service.get_request_count(), 1);
    }
    
    #[test]
    fn test_mock_hsm() {
        let hsm = MockHSM::new();
        
        let key = hsm.generate_key("test-key").unwrap();
        assert_eq!(key.len(), 32);
        
        let retrieved_key = hsm.get_key("test-key").unwrap();
        assert_eq!(key, retrieved_key);
        
        let signature = hsm.sign_data("test-key", b"test data").unwrap();
        assert_eq!(signature.len(), 64);
        
        assert!(hsm.verify_signature("test-key", b"test data", &signature));
    }
    
    #[tokio::test]
    async fn test_mock_model_engine() {
        let engine = MockModelEngine::new();
        
        let models = engine.list_models();
        assert!(models.contains(&"test-model".to_string()));
        
        let response = engine.process_request("test-model", "Hello").await;
        assert!(response.is_ok());
        assert!(response.unwrap().contains("Hello"));
    }
}