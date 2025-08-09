//! Model cache implementation

/// Simple model cache implementation
pub struct ModelCache {
    cache_size_mb: u32,
    max_models: u32,
}

impl ModelCache {
    pub fn new(cache_size_mb: u32, max_models: u32) -> Self {
        Self {
            cache_size_mb,
            max_models,
        }
    }
}
