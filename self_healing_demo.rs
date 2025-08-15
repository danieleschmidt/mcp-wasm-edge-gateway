//! Self-Healing Pipeline Guard Demo
//! 
//! This demonstrates the autonomous pipeline health monitoring and recovery
//! capabilities implemented in Generation 1.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{info, warn, error, debug};

/// Mock component that implements PipelineAware for demonstration
struct MockComponent {
    id: String,
    healthy: Arc<std::sync::atomic::AtomicBool>,
    metrics: Arc<tokio::sync::RwLock<HashMap<String, f64>>>,
}

impl MockComponent {
    fn new(id: &str) -> Self {
        let mut metrics = HashMap::new();
        metrics.insert("response_time_ms".to_string(), 100.0);
        metrics.insert("error_rate".to_string(), 0.02);
        metrics.insert("memory_usage_mb".to_string(), 64.0);
        
        MockComponent {
            id: id.to_string(),
            healthy: Arc::new(std::sync::atomic::AtomicBool::new(true)),
            metrics: Arc::new(tokio::sync::RwLock::new(metrics)),
        }
    }
    
    /// Simulate a component failure
    async fn simulate_failure(&self) {
        warn!("Simulating failure in component: {}", self.id);
        self.healthy.store(false, std::sync::atomic::Ordering::SeqCst);
        
        // Simulate degraded metrics
        let mut metrics = self.metrics.write().await;
        metrics.insert("response_time_ms".to_string(), 5000.0);
        metrics.insert("error_rate".to_string(), 0.5);
        metrics.insert("memory_usage_mb".to_string(), 400.0);
    }
    
    /// Simulate component recovery
    async fn simulate_recovery(&self) {
        info!("Simulating recovery for component: {}", self.id);
        self.healthy.store(true, std::sync::atomic::Ordering::SeqCst);
        
        // Restore normal metrics
        let mut metrics = self.metrics.write().await;
        metrics.insert("response_time_ms".to_string(), 120.0);
        metrics.insert("error_rate".to_string(), 0.01);
        metrics.insert("memory_usage_mb".to_string(), 72.0);
    }
}

// Mock implementation of PipelineAware trait
#[async_trait::async_trait]
impl mcp_pipeline_guard::PipelineAware for MockComponent {
    async fn is_healthy(&self) -> bool {
        self.healthy.load(std::sync::atomic::Ordering::SeqCst)
    }
    
    async fn get_metrics(&self) -> std::collections::HashMap<String, f64> {
        self.metrics.read().await.clone()
    }
    
    async fn recover(&self) -> mcp_common::Result<()> {
        info!("Recovery triggered for component: {}", self.id);
        self.simulate_recovery().await;
        Ok(())
    }
    
    fn component_id(&self) -> &str {
        &self.id
    }
}

/// Demonstration function showing autonomous SDLC implementation
async fn run_self_healing_demo() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    info!("🚀 Starting Self-Healing Pipeline Guard Demo");
    info!("📋 This demonstrates Generation 1: MAKE IT WORK functionality");
    
    // Create mock configuration
    let config = mcp_common::Config::default();
    
    // Initialize pipeline guard
    info!("🛡️ Initializing Pipeline Guard...");
    let pipeline_guard = mcp_pipeline_guard::create_pipeline_guard(config).await?;
    
    // Create mock components
    let router_component = Arc::new(MockComponent::new("router"));
    let model_component = Arc::new(MockComponent::new("model_engine"));
    let queue_component = Arc::new(MockComponent::new("queue"));
    
    // Register components with pipeline guard
    info!("📝 Registering components for monitoring...");
    pipeline_guard.register_component(router_component.clone()).await?;
    pipeline_guard.register_component(model_component.clone()).await?;
    pipeline_guard.register_component(queue_component.clone()).await?;
    
    // Display initial health status
    let health = pipeline_guard.get_health_status().await?;
    info!("✅ Initial Health Status: {:?}", health.status);
    
    let metrics = pipeline_guard.get_pipeline_metrics().await;
    info!("📊 Initial Metrics: {} total components, {} healthy", 
          metrics.get("total_components").unwrap_or(&0.0),
          metrics.get("healthy_components").unwrap_or(&0.0));
    
    // Simulate normal operations for a few seconds
    info!("⏳ Running normal operations...");
    for i in 1..=3 {
        sleep(Duration::from_secs(1)).await;
        debug!("Normal operation cycle {}", i);
    }
    
    // Simulate component failure
    info!("🔥 Simulating component failure...");
    model_component.simulate_failure().await;
    
    // Wait for the pipeline guard to detect and respond to the failure
    info!("🔍 Waiting for pipeline guard to detect failure...");
    sleep(Duration::from_secs(2)).await;
    
    // Force a health check to demonstrate monitoring
    pipeline_guard.force_health_check().await?;
    
    let health_after_failure = pipeline_guard.get_health_status().await?;
    let metrics_after_failure = pipeline_guard.get_pipeline_metrics().await;
    
    info!("⚠️ Health After Failure: {:?}", health_after_failure.status);
    info!("📉 Failed Components: {}", 
          metrics_after_failure.get("failed_components").unwrap_or(&0.0));
    
    // Demonstrate manual recovery trigger
    info!("🔧 Triggering manual recovery...");
    if let Err(e) = pipeline_guard.recover_component("model_engine").await {
        warn!("Recovery attempt failed: {}", e);
    } else {
        info!("✅ Recovery triggered successfully");
    }
    
    // Wait and check recovery
    sleep(Duration::from_secs(1)).await;
    let health_after_recovery = pipeline_guard.get_health_status().await?;
    let metrics_after_recovery = pipeline_guard.get_pipeline_metrics().await;
    
    info!("🎉 Health After Recovery: {:?}", health_after_recovery.status);
    info!("📈 Healthy Components: {}", 
          metrics_after_recovery.get("healthy_components").unwrap_or(&0.0));
    
    // Demonstrate multiple failure scenario
    info!("⚡ Simulating multiple component failures...");
    router_component.simulate_failure().await;
    queue_component.simulate_failure().await;
    
    sleep(Duration::from_secs(1)).await;
    pipeline_guard.force_health_check().await?;
    
    let health_critical = pipeline_guard.get_health_status().await?;
    info!("🚨 Critical System Health: {:?}", health_critical.status);
    
    // Demonstrate automatic recovery would kick in here
    info!("🤖 In production, automatic recovery would be triggered");
    
    // Manual recovery for demo
    router_component.simulate_recovery().await;
    queue_component.simulate_recovery().await;
    
    sleep(Duration::from_secs(1)).await;
    pipeline_guard.force_health_check().await?;
    
    let final_health = pipeline_guard.get_health_status().await?;
    let final_metrics = pipeline_guard.get_pipeline_metrics().await;
    
    info!("✨ Final Health Status: {:?}", final_health.status);
    info!("🎯 Final Metrics Summary:");
    info!("   • Total Components: {}", final_metrics.get("total_components").unwrap_or(&0.0));
    info!("   • Healthy Components: {}", final_metrics.get("healthy_components").unwrap_or(&0.0));
    info!("   • Failed Components: {}", final_metrics.get("failed_components").unwrap_or(&0.0));
    info!("   • Uptime: {} seconds", final_metrics.get("uptime_seconds").unwrap_or(&0.0));
    
    info!("🏁 Demo completed successfully!");
    info!("🔮 Next: Generation 2 will add robust error handling and comprehensive monitoring");
    
    Ok(())
}

#[tokio::main]
async fn main() {
    match run_self_healing_demo().await {
        Ok(_) => {
            println!("\n🎉 Self-Healing Pipeline Guard Demo completed successfully!");
            println!("🚀 Pipeline Guard Generation 1 is working!");
        },
        Err(e) => {
            eprintln!("❌ Demo failed: {}", e);
            std::process::exit(1);
        }
    }
}