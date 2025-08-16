//! Input validation and sanitization for MCP Edge Gateway

use mcp_common::{Error, Result, MCPRequest};
use regex::Regex;
use std::collections::HashMap;
use tracing::{debug, warn};

/// Input validation configuration
#[derive(Debug, Clone)]
pub struct ValidationConfig {
    /// Maximum request size in bytes
    pub max_request_size: usize,
    /// Maximum number of parameters
    pub max_parameters: usize,
    /// Maximum string length
    pub max_string_length: usize,
    /// Maximum nesting depth for JSON
    pub max_nesting_depth: u32,
    /// Enable content sanitization
    pub enable_sanitization: bool,
    /// Blocked content patterns
    pub blocked_patterns: Vec<String>,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self {
            max_request_size: 1024 * 1024, // 1MB
            max_parameters: 100,
            max_string_length: 10000,
            max_nesting_depth: 10,
            enable_sanitization: true,
            blocked_patterns: vec![
                // SQL injection patterns
                r"(?i)(union|select|insert|update|delete|drop|create|alter)\s+".to_string(),
                // Script injection patterns
                r"(?i)<script[^>]*>.*?</script>".to_string(),
                r"(?i)javascript:".to_string(),
                // Path traversal patterns
                r"\.\.[\\/]".to_string(),
                // Command injection patterns
                r"[;&|`]".to_string(),
            ],
        }
    }
}

/// Input validator for security
pub struct InputValidator {
    config: ValidationConfig,
    blocked_patterns: Vec<Regex>,
}

impl InputValidator {
    /// Create a new input validator
    pub fn new(config: ValidationConfig) -> Result<Self> {
        let mut blocked_patterns = Vec::new();
        
        for pattern in &config.blocked_patterns {
            match Regex::new(pattern) {
                Ok(regex) => blocked_patterns.push(regex),
                Err(e) => {
                    warn!("Invalid regex pattern '{}': {}", pattern, e);
                }
            }
        }

        Ok(Self {
            config,
            blocked_patterns,
        })
    }

    /// Validate an MCP request
    pub async fn validate_request(&self, request: &MCPRequest) -> Result<()> {
        debug!("Validating request {}", request.id);

        // Check request size
        let request_size = serde_json::to_string(request)
            .map_err(|e| Error::Security(format!("Failed to serialize request: {}", e)))?
            .len();

        if request_size > self.config.max_request_size {
            return Err(Error::Security(format!(
                "Request size {} exceeds maximum {}",
                request_size, self.config.max_request_size
            )));
        }

        // Check number of parameters
        if request.params.len() > self.config.max_parameters {
            return Err(Error::Security(format!(
                "Parameter count {} exceeds maximum {}",
                request.params.len(), self.config.max_parameters
            )));
        }

        // Validate method name
        self.validate_string(&request.method, "method")?;

        // Validate device ID
        self.validate_string(&request.device_id, "device_id")?;

        // Validate parameters
        self.validate_parameters(&request.params)?;

        // Check for blocked content patterns
        if self.config.enable_sanitization {
            self.check_blocked_patterns(request)?;
        }

        debug!("Request {} validation passed", request.id);
        Ok(())
    }

    /// Validate a string field
    fn validate_string(&self, value: &str, field_name: &str) -> Result<()> {
        // Check length
        if value.len() > self.config.max_string_length {
            return Err(Error::Security(format!(
                "Field '{}' length {} exceeds maximum {}",
                field_name, value.len(), self.config.max_string_length
            )));
        }

        // Check for null bytes
        if value.contains('\0') {
            return Err(Error::Security(format!(
                "Field '{}' contains null bytes",
                field_name
            )));
        }

        // Check for control characters (except common whitespace)
        for ch in value.chars() {
            if ch.is_control() && !matches!(ch, '\t' | '\n' | '\r') {
                return Err(Error::Security(format!(
                    "Field '{}' contains invalid control character",
                    field_name
                )));
            }
        }

        Ok(())
    }

    /// Validate JSON parameters
    fn validate_parameters(&self, params: &HashMap<String, serde_json::Value>) -> Result<()> {
        for (key, value) in params {
            self.validate_string(key, "parameter_key")?;
            self.validate_json_value(value, 0)?;
        }
        Ok(())
    }

    /// Validate JSON value recursively
    fn validate_json_value(&self, value: &serde_json::Value, depth: u32) -> Result<()> {
        if depth > self.config.max_nesting_depth {
            return Err(Error::Security(format!(
                "JSON nesting depth {} exceeds maximum {}",
                depth, self.config.max_nesting_depth
            )));
        }

        match value {
            serde_json::Value::String(s) => {
                self.validate_string(s, "parameter_value")?;
            }
            serde_json::Value::Array(arr) => {
                if arr.len() > self.config.max_parameters {
                    return Err(Error::Security(format!(
                        "Array length {} exceeds maximum {}",
                        arr.len(), self.config.max_parameters
                    )));
                }
                for item in arr {
                    self.validate_json_value(item, depth + 1)?;
                }
            }
            serde_json::Value::Object(obj) => {
                if obj.len() > self.config.max_parameters {
                    return Err(Error::Security(format!(
                        "Object key count {} exceeds maximum {}",
                        obj.len(), self.config.max_parameters
                    )));
                }
                for (key, val) in obj {
                    self.validate_string(key, "object_key")?;
                    self.validate_json_value(val, depth + 1)?;
                }
            }
            _ => {} // Numbers, booleans, null are safe
        }

        Ok(())
    }

    /// Check for blocked content patterns
    fn check_blocked_patterns(&self, request: &MCPRequest) -> Result<()> {
        let request_text = serde_json::to_string(request)
            .map_err(|e| Error::Security(format!("Failed to serialize request: {}", e)))?;

        for (i, pattern) in self.blocked_patterns.iter().enumerate() {
            if pattern.is_match(&request_text) {
                return Err(Error::Security(format!(
                    "Request contains blocked content pattern {}",
                    i + 1
                )));
            }
        }

        Ok(())
    }

    /// Sanitize a string by removing potentially dangerous content
    pub fn sanitize_string(&self, input: &str) -> String {
        let mut sanitized = input.to_string();

        // Remove null bytes
        sanitized = sanitized.replace('\0', "");

        // Remove or escape potentially dangerous characters
        sanitized = sanitized.replace('<', "&lt;");
        sanitized = sanitized.replace('>', "&gt;");
        sanitized = sanitized.replace('&', "&amp;");
        sanitized = sanitized.replace('"', "&quot;");
        sanitized = sanitized.replace('\'', "&#x27;");

        // Remove control characters (except common whitespace)
        sanitized = sanitized.chars()
            .filter(|&ch| !ch.is_control() || matches!(ch, '\t' | '\n' | '\r'))
            .collect();

        // Limit length
        if sanitized.len() > self.config.max_string_length {
            sanitized.truncate(self.config.max_string_length);
        }

        sanitized
    }

    /// Check if an IP address is potentially malicious
    pub fn validate_ip_address(&self, ip: &str) -> Result<()> {
        // Basic IP format validation
        let ip_parts: Vec<&str> = ip.split('.').collect();
        if ip_parts.len() != 4 {
            return Err(Error::Security("Invalid IP address format".to_string()));
        }

        for part in ip_parts {
            match part.parse::<u8>() {
                Ok(_) => {}
                Err(_) => return Err(Error::Security("Invalid IP address format".to_string())),
            }
        }

        // Check for private/reserved ranges that shouldn't be accessing externally
        if self.is_internal_ip(ip) {
            warn!("Request from internal IP address: {}", ip);
        }

        // Check for known malicious patterns
        let malicious_patterns = [
            "0.0.0.0",
            "127.0.0.1", // Localhost attempts
            "169.254.", // Link-local
            "224.", // Multicast
            "255.255.255.255", // Broadcast
        ];

        for pattern in &malicious_patterns {
            if ip.starts_with(pattern) {
                return Err(Error::Security(format!(
                    "Blocked IP address pattern: {}",
                    ip
                )));
            }
        }

        Ok(())
    }

    /// Check if IP is in internal/private range
    fn is_internal_ip(&self, ip: &str) -> bool {
        // RFC 1918 private ranges
        let private_ranges = [
            "10.",      // 10.0.0.0/8
            "172.16.",  // 172.16.0.0/12 (simplified check)
            "172.17.", "172.18.", "172.19.", "172.20.",
            "172.21.", "172.22.", "172.23.", "172.24.",
            "172.25.", "172.26.", "172.27.", "172.28.",
            "172.29.", "172.30.", "172.31.",
            "192.168.", // 192.168.0.0/16
        ];

        private_ranges.iter().any(|&range| ip.starts_with(range))
    }

    /// Rate limiting check (simplified)
    pub fn check_rate_limit(&self, client_id: &str, requests_per_minute: u32) -> Result<()> {
        // This is a simplified implementation
        // In production, you'd use a proper rate limiting algorithm like token bucket
        if requests_per_minute > 100 {
            return Err(Error::Security(format!(
                "Rate limit exceeded for client {}: {} requests/minute",
                client_id, requests_per_minute
            )));
        }
        Ok(())
    }
}

/// Content sanitization utilities
pub struct ContentSanitizer;

impl ContentSanitizer {
    /// Remove potentially dangerous HTML/XML content
    pub fn sanitize_html(input: &str) -> String {
        let dangerous_tags = [
            "script", "iframe", "object", "embed", "form", "input",
            "textarea", "button", "select", "option", "meta", "link",
            "style", "base", "applet", "marquee", "bgsound",
        ];

        let mut sanitized = input.to_string();

        // Remove dangerous tags
        for tag in &dangerous_tags {
            let patterns = [
                format!(r"(?i)<{}\b[^>]*>.*?</{}>", tag, tag),
                format!(r"(?i)<{}\b[^>]*/>", tag),
                format!(r"(?i)<{}\b[^>]*>", tag),
            ];

            for pattern in &patterns {
                if let Ok(regex) = Regex::new(pattern) {
                    sanitized = regex.replace_all(&sanitized, "").to_string();
                }
            }
        }

        // Remove javascript: URLs
        if let Ok(regex) = Regex::new(r"(?i)javascript\s*:") {
            sanitized = regex.replace_all(&sanitized, "").to_string();
        }

        // Remove on* event handlers
        if let Ok(regex) = Regex::new(r#"(?i)\s+on\w+\s*=\s*["'][^"']*["']"#) {
            sanitized = regex.replace_all(&sanitized, "").to_string();
        }

        sanitized
    }

    /// Sanitize for SQL context (basic)
    pub fn sanitize_sql(input: &str) -> String {
        input
            .replace('\'', "''")  // Escape single quotes
            .replace('\\', "\\\\") // Escape backslashes
            .replace('\0', "")     // Remove null bytes
    }

    /// Sanitize file paths
    pub fn sanitize_path(input: &str) -> String {
        input
            .replace("..", "")     // Remove path traversal
            .replace('\\', "/")    // Normalize separators
            .chars()
            .filter(|&c| c.is_alphanumeric() || matches!(c, '/' | '-' | '_' | '.'))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcp_common::types::MCPRequest;
    use uuid::Uuid;

    fn create_test_request() -> MCPRequest {
        MCPRequest {
            id: Uuid::new_v4(),
            device_id: "test-device".to_string(),
            method: "test".to_string(),
            params: HashMap::new(),
            context: None,
            timestamp: chrono::Utc::now(),
        }
    }

    #[tokio::test]
    async fn test_valid_request() {
        let validator = InputValidator::new(ValidationConfig::default()).unwrap();
        let request = create_test_request();
        
        assert!(validator.validate_request(&request).await.is_ok());
    }

    #[tokio::test]
    async fn test_blocked_patterns() {
        let validator = InputValidator::new(ValidationConfig::default()).unwrap();
        let mut request = create_test_request();
        
        // Add SQL injection attempt
        request.params.insert(
            "query".to_string(),
            serde_json::Value::String("SELECT * FROM users".to_string())
        );
        
        assert!(validator.validate_request(&request).await.is_err());
    }

    #[tokio::test]
    async fn test_oversized_request() {
        let mut config = ValidationConfig::default();
        config.max_string_length = 10;
        
        let validator = InputValidator::new(config).unwrap();
        let mut request = create_test_request();
        request.method = "very_long_method_name_that_exceeds_limit".to_string();
        
        assert!(validator.validate_request(&request).await.is_err());
    }

    #[test]
    fn test_sanitize_html() {
        let input = "<script>alert('xss')</script><p>Safe content</p>";
        let sanitized = ContentSanitizer::sanitize_html(input);
        
        assert!(!sanitized.contains("<script>"));
        assert!(sanitized.contains("<p>Safe content</p>"));
    }

    #[test]
    fn test_ip_validation() {
        let validator = InputValidator::new(ValidationConfig::default()).unwrap();
        
        assert!(validator.validate_ip_address("192.168.1.1").is_ok());
        assert!(validator.validate_ip_address("invalid.ip").is_err());
        assert!(validator.validate_ip_address("0.0.0.0").is_err());
    }
}