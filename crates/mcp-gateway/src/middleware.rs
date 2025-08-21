//! HTTP middleware for the gateway server

use axum::{
    extract::Request,
    http::{HeaderMap, HeaderValue},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use std::time::Instant;
use tower::{Layer, Service};
use tracing::{info, warn};
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

/// Rate limiting middleware
#[derive(Clone)]
pub struct RateLimitLayer {
    requests_per_window: u32,
    window_seconds: u64,
}

impl RateLimitLayer {
    pub fn new(requests_per_window: u32, window_seconds: u64) -> Self {
        Self {
            requests_per_window,
            window_seconds,
        }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitMiddleware {
            inner,
            requests_per_window: self.requests_per_window,
            window_seconds: self.window_seconds,
        }
    }
}

#[derive(Clone)]
pub struct RateLimitMiddleware<S> {
    inner: S,
    requests_per_window: u32,
    window_seconds: u64,
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
        // Simplified rate limiting - in production, use a proper rate limiter
        // with Redis or in-memory store with client IP tracking
        let future = self.inner.call(request);
        Box::pin(async move {
            // For now, just pass through all requests
            // TODO: Implement proper rate limiting with client tracking
            future.await
        })
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

            // TODO: Send metrics to telemetry collector

            Ok(response)
        })
    }
}
