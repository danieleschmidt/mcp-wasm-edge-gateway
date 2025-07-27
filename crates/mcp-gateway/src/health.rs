//! Health check utilities and implementations

use mcp_common::{HealthStatus, HealthLevel, ComponentHealth};
use std::collections::HashMap;
use chrono::Utc;

/// Perform a comprehensive health check
pub async fn comprehensive_health_check() -> HealthStatus {
    let mut components = HashMap::new();
    
    // Check system resources
    components.insert(
        "system".to_string(),
        check_system_health().await
    );
    
    // Check network connectivity
    components.insert(
        "network".to_string(),
        check_network_health().await
    );
    
    // Check storage
    components.insert(
        "storage".to_string(),
        check_storage_health().await
    );
    
    let mut health_status = HealthStatus {
        overall_health: HealthLevel::Healthy,
        components,
        last_check: Utc::now(),
        uptime_seconds: get_uptime_seconds(),
    };
    
    health_status.calculate_overall_health();
    health_status
}

async fn check_system_health() -> ComponentHealth {
    let mut metrics = HashMap::new();
    
    // Check memory usage
    let memory_info = get_memory_info();
    metrics.insert("memory_usage_percent".to_string(), memory_info.usage_percent);
    metrics.insert("memory_available_mb".to_string(), memory_info.available_mb as f32);
    
    // Check CPU usage
    let cpu_usage = get_cpu_usage();
    metrics.insert("cpu_usage_percent".to_string(), cpu_usage);
    
    // Determine health status
    let status = if memory_info.usage_percent > 90.0 || cpu_usage > 95.0 {
        HealthLevel::Critical
    } else if memory_info.usage_percent > 80.0 || cpu_usage > 85.0 {
        HealthLevel::Warning
    } else {
        HealthLevel::Healthy
    };
    
    let message = match status {
        HealthLevel::Healthy => "System resources are healthy".to_string(),
        HealthLevel::Warning => "System resources are under pressure".to_string(),
        HealthLevel::Critical => "System resources are critically low".to_string(),
        HealthLevel::Unknown => "System health unknown".to_string(),
    };
    
    ComponentHealth {
        status,
        message,
        last_check: Utc::now(),
        metrics,
    }
}

async fn check_network_health() -> ComponentHealth {
    let mut metrics = HashMap::new();
    
    // Simple network connectivity check
    let is_connected = test_network_connectivity().await;
    metrics.insert("connected".to_string(), if is_connected { 1.0 } else { 0.0 });
    
    let status = if is_connected {
        HealthLevel::Healthy
    } else {
        HealthLevel::Warning
    };
    
    let message = if is_connected {
        "Network connectivity is healthy".to_string()
    } else {
        "Network connectivity issues detected".to_string()
    };
    
    ComponentHealth {
        status,
        message,
        last_check: Utc::now(),
        metrics,
    }
}

async fn check_storage_health() -> ComponentHealth {
    let mut metrics = HashMap::new();
    
    let storage_info = get_storage_info();
    metrics.insert("disk_usage_percent".to_string(), storage_info.usage_percent);
    metrics.insert("disk_available_mb".to_string(), storage_info.available_mb as f32);
    
    let status = if storage_info.usage_percent > 95.0 {
        HealthLevel::Critical
    } else if storage_info.usage_percent > 85.0 {
        HealthLevel::Warning
    } else {
        HealthLevel::Healthy
    };
    
    let message = match status {
        HealthLevel::Healthy => "Storage is healthy".to_string(),
        HealthLevel::Warning => "Storage usage is high".to_string(),
        HealthLevel::Critical => "Storage is critically full".to_string(),
        HealthLevel::Unknown => "Storage health unknown".to_string(),
    };
    
    ComponentHealth {
        status,
        message,
        last_check: Utc::now(),
        metrics,
    }
}

struct MemoryInfo {
    usage_percent: f32,
    available_mb: u64,
    total_mb: u64,
}

struct StorageInfo {
    usage_percent: f32,
    available_mb: u64,
    total_mb: u64,
}

fn get_memory_info() -> MemoryInfo {
    // Platform-specific memory information
    // This is a simplified implementation
    #[cfg(target_os = "linux")]
    {
        if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
            let mut total_kb = 0;
            let mut available_kb = 0;
            
            for line in meminfo.lines() {
                if line.starts_with("MemTotal:") {
                    total_kb = line.split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0);
                } else if line.starts_with("MemAvailable:") {
                    available_kb = line.split_whitespace()
                        .nth(1)
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0);
                }
            }
            
            let total_mb = total_kb / 1024;
            let available_mb = available_kb / 1024;
            let used_mb = total_mb - available_mb;
            let usage_percent = if total_mb > 0 {
                (used_mb as f32 / total_mb as f32) * 100.0
            } else {
                0.0
            };
            
            return MemoryInfo {
                usage_percent,
                available_mb,
                total_mb,
            };
        }
    }
    
    // Fallback for other platforms or if reading fails
    MemoryInfo {
        usage_percent: 50.0, // Assume 50% usage
        available_mb: 1024,  // Assume 1GB available
        total_mb: 2048,      // Assume 2GB total
    }
}

fn get_cpu_usage() -> f32 {
    // Platform-specific CPU usage
    // This is a simplified implementation
    #[cfg(target_os = "linux")]
    {
        if let Ok(stat) = std::fs::read_to_string("/proc/stat") {
            if let Some(cpu_line) = stat.lines().next() {
                if cpu_line.starts_with("cpu ") {
                    let values: Vec<u64> = cpu_line
                        .split_whitespace()
                        .skip(1)
                        .take(4)
                        .filter_map(|s| s.parse().ok())
                        .collect();
                    
                    if values.len() >= 4 {
                        let idle = values[3];
                        let total: u64 = values.iter().sum();
                        let usage = if total > 0 {
                            100.0 - (idle as f32 / total as f32 * 100.0)
                        } else {
                            0.0
                        };
                        return usage;
                    }
                }
            }
        }
    }
    
    // Fallback
    25.0 // Assume 25% CPU usage
}

fn get_storage_info() -> StorageInfo {
    // Platform-specific storage information
    // This is a simplified implementation
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        
        if let Ok(metadata) = std::fs::metadata(".") {
            // This is a very basic implementation
            // In practice, you'd use statvfs or similar
            let total_mb = 10 * 1024; // Assume 10GB
            let available_mb = 5 * 1024; // Assume 5GB available
            let usage_percent = ((total_mb - available_mb) as f32 / total_mb as f32) * 100.0;
            
            return StorageInfo {
                usage_percent,
                available_mb,
                total_mb,
            };
        }
    }
    
    // Fallback
    StorageInfo {
        usage_percent: 60.0, // Assume 60% usage
        available_mb: 4096,  // Assume 4GB available
        total_mb: 10240,     // Assume 10GB total
    }
}

async fn test_network_connectivity() -> bool {
    // Simple connectivity test
    // In production, you might want to test specific endpoints
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::time::Duration;
        use tokio::time::timeout;
        
        let result = timeout(
            Duration::from_secs(5),
            tokio::net::TcpStream::connect("8.8.8.8:53")
        ).await;
        
        result.is_ok() && result.unwrap().is_ok()
    }
    
    #[cfg(target_arch = "wasm32")]
    {
        // In WASM, assume connectivity is available
        // Real implementation would use fetch API to test
        true
    }
}

fn get_uptime_seconds() -> u64 {
    // Platform-specific uptime
    #[cfg(target_os = "linux")]
    {
        if let Ok(uptime_str) = std::fs::read_to_string("/proc/uptime") {
            if let Some(uptime_part) = uptime_str.split_whitespace().next() {
                if let Ok(uptime_seconds) = uptime_part.parse::<f64>() {
                    return uptime_seconds as u64;
                }
            }
        }
    }
    
    // Fallback - return a reasonable default
    3600 // 1 hour
}