//! HTTP middleware for the gateway server

use axum::{
    extract::Request,
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::Response,
};
use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;
use tower::{Layer, Service};
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Request ID middleware to add unique IDs to requests
#[derive(Clone)]
pub struct RequestIdLayer;

impl RequestIdLayer {
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for RequestIdLayer {
    type Service = RequestIdMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestIdMiddleware {
            inner,
        }
    }
}

#[derive(Clone)]
pub struct RequestIdMiddleware<S> {
    inner: S,
}

impl<S> Service<Request> for RequestIdMiddleware<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: Request) -> Self::Future {
        let request_id = Uuid::new_v4().to_string();
        
        // Safely add request ID header
        if let Ok(header_value) = HeaderValue::from_str(&request_id) {
            request.headers_mut().insert("x-request-id", header_value);
        } else {
            warn!("Failed to create header value for request ID: {}", request_id);
        }

        let future = self.inner.call(request);
        let request_id_clone = request_id.clone();
        
        Box::pin(async move {
            let mut response = future.await?;
            
            // Safely add response ID header
            if let Ok(header_value) = HeaderValue::from_str(&request_id_clone) {
                response.headers_mut().insert("x-request-id", header_value);
            } else {
                warn!("Failed to create response header value for request ID: {}", request_id_clone);
            }
            
            Ok(response)
        })
    }
}

/// Client request tracking for rate limiting
#[derive(Debug, Clone)]
struct ClientRateLimit {
    requests: Vec<u64>, // Unix timestamps of requests
    blocked_until: Option<u64>, // Unix timestamp when client is unblocked
}

impl ClientRateLimit {
    fn new() -> Self {
        Self {
            requests: Vec::new(),
            blocked_until: None,
        }
    }

    fn is_blocked(&self) -> bool {
        if let Some(blocked_until) = self.blocked_until {
            let now = SystemTime::now().duration_since(UNIX_EPOCH)
                .unwrap_or_default().as_secs();
            blocked_until > now
        } else {
            false
        }
    }

    fn add_request(&mut self, timestamp: u64) {
        self.requests.push(timestamp);
    }

    fn cleanup_old_requests(&mut self, window_seconds: u64) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap_or_default().as_secs();
        let cutoff = now.saturating_sub(window_seconds);
        self.requests.retain(|&ts| ts > cutoff);
    }

    fn block_client(&mut self, block_duration_seconds: u64) {
        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap_or_default().as_secs();
        self.blocked_until = Some(now + block_duration_seconds);
    }
}

/// Rate limiting middleware
#[derive(Clone)]
pub struct RateLimitLayer {
    requests_per_window: u32,
    window_seconds: u64,
    clients: Arc<RwLock<HashMap<String, ClientRateLimit>>>,
}

impl RateLimitLayer {
    pub fn new(requests_per_window: u32, window_seconds: u64) -> Self {
        let clients = Arc::new(RwLock::new(HashMap::new()));
        
        // Spawn cleanup task to remove old client data
        let clients_cleanup = clients.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300)); // Cleanup every 5 minutes
            loop {
                interval.tick().await;
                let mut clients = clients_cleanup.write().await;
                let now = SystemTime::now().duration_since(UNIX_EPOCH)
                    .unwrap_or_default().as_secs();
                
                // Remove clients with no recent activity
                clients.retain(|_, client| {
                    if client.requests.is_empty() {
                        return false;
                    }
                    let last_request = client.requests.iter().max().unwrap_or(&0);
                    now - last_request < 3600 // Keep data for 1 hour
                });
                
                debug!("Rate limiter cleanup: {} active clients", clients.len());
            }
        });
        
        Self {
            requests_per_window,
            window_seconds,
            clients,
        }
    }
    
    async fn check_rate_limit(&self, client_id: &str) -> bool {
        let mut clients = self.clients.write().await;
        let client = clients.entry(client_id.to_string()).or_insert_with(ClientRateLimit::new);
        
        // Check if client is currently blocked
        if client.is_blocked() {
            return false;
        }
        
        // Clean up old requests
        client.cleanup_old_requests(self.window_seconds);
        
        // Check if adding this request would exceed the rate limit
        if client.requests.len() >= self.requests_per_window as usize {
            // Block the client for twice the window duration
            client.block_client(self.window_seconds * 2);
            warn!("Rate limit exceeded for client {}, blocking for {}s", client_id, self.window_seconds * 2);
            return false;
        }
        
        // Add this request
        let now = SystemTime::now().duration_since(UNIX_EPOCH)
            .unwrap_or_default().as_secs();
        client.add_request(now);
        
        debug!("Client {} now has {} requests in window", client_id, client.requests.len());
        true
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitMiddleware {
            inner,
            layer: self.clone(),
        }
    }
}

#[derive(Clone)]
pub struct RateLimitMiddleware<S> {
    inner: S,
    layer: RateLimitLayer,
}

impl<S> Service<Request> for RateLimitMiddleware<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        // Extract client identifier (IP address or API key)
        let client_id = extract_client_id(&request);
        let layer = self.layer.clone();
        let future = self.inner.call(request);
        
        Box::pin(async move {
            // Check rate limit
            if !layer.check_rate_limit(&client_id).await {
                // Rate limit exceeded, return 429 Too Many Requests
                let mut response = Response::new(axum::body::Body::from("Rate limit exceeded"));
                *response.status_mut() = StatusCode::TOO_MANY_REQUESTS;
                response.headers_mut().insert(
                    "Retry-After", 
                    HeaderValue::from_str(&layer.window_seconds.to_string()).unwrap_or_default()
                );
                return Ok(response);
            }
            
            // Process request normally
            future.await
        })
    }
}

/// Extract client identifier from request for rate limiting
fn extract_client_id(request: &Request) -> String {
    // Try to get API key from Authorization header first
    if let Some(auth_header) = request.headers().get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                return auth_str[7..].to_string(); // Use API key as identifier
            }
        }
    }
    
    // Try to get client IP from X-Forwarded-For header
    if let Some(forwarded) = request.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(ip) = forwarded_str.split(',').next() {
                return ip.trim().to_string();
            }
        }
    }
    
    // Try to get client IP from X-Real-IP header
    if let Some(real_ip) = request.headers().get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            return ip_str.to_string();
        }
    }
    
    // Fallback to remote address if available
    if let Some(connect_info) = request.extensions().get::<axum::extract::ConnectInfo<std::net::SocketAddr>>() {
        return connect_info.0.ip().to_string();
    }
    
    // Ultimate fallback to unknown client
    "unknown".to_string()
}

/// Collect and send request metrics to telemetry system
async fn collect_request_metrics(
    method: &str,
    path: &str,
    status: u16,
    duration: Duration,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Create metrics data structure
    let metrics = serde_json::json!({
        "timestamp": SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis(),
        "method": method,
        "path": path,
        "status_code": status,
        "duration_ms": duration.as_millis(),
        "service": "mcp-gateway",
        "version": env!("CARGO_PKG_VERSION"),
    });

    // In a real implementation, this would send to a metrics aggregation service
    // For now, we'll use structured logging that can be ingested by monitoring systems
    debug!(
        metrics = ?metrics,
        "Request metrics collected"
    );

    // Future enhancement: Send to Prometheus, InfluxDB, or CloudWatch
    // Example implementation would be:
    // telemetry_client.send_metric("http_request_duration", duration.as_millis(), tags).await?;
    // telemetry_client.increment_counter("http_requests_total", tags).await?;

    Ok(())
}
}

/// Metrics collection middleware
#[derive(Clone)]
pub struct MetricsLayer;

impl MetricsLayer {
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for MetricsLayer {
    type Service = MetricsMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MetricsMiddleware {
            inner,
        }
    }
}

#[derive(Clone)]
pub struct MetricsMiddleware<S> {
    inner: S,
}

impl<S> Service<Request> for MetricsMiddleware<S>
where
    S: Service<Request, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request) -> Self::Future {
        let start_time = Instant::now();
        let method = request.method().to_string();
        let path = request.uri().path().to_string();

        let future = self.inner.call(request);
        Box::pin(async move {
            let response = future.await?;
            let duration = start_time.elapsed();
            let status = response.status().as_u16();

            // Log request metrics
            info!(
                method = %method,
                path = %path,
                status = %status,
                duration_ms = %duration.as_millis(),
                "HTTP request completed"
            );

            // Send metrics to telemetry collector
            if let Err(e) = collect_request_metrics(&method, &path, status, duration).await {
                warn!("Failed to collect metrics: {}", e);
            }

            Ok(response)
        })
    }
}
