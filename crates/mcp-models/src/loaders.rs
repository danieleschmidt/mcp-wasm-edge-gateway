//! Model loaders for different formats

/// Model loader trait
pub trait ModelLoader {
    fn load(&self, path: &str) -> Result<(), Box<dyn std::error::Error>>;
}