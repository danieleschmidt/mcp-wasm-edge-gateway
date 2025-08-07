//! Standard model engine implementation

use crate::ModelEngine;
use crate::loaders::{create_model_loader, LoadedModel, ModelLoader};
use async_trait::async_trait;
use mcp_common::metrics::{ComponentHealth, HealthLevel};
use mcp_common::{Config, Error, MCPRequest, MCPResponse, ModelId, ModelFormat, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

/// Standard implementation of the model engine
pub struct StandardModelEngine {
    config: Arc<Config>,
    models: Arc<RwLock<HashMap<ModelId, LoadedModel>>>,
    cache: Arc<crate::cache::ModelCache>,
    loaders: Arc<RwLock<HashMap<ModelFormat, Box<dyn ModelLoader>>>>,
}

impl StandardModelEngine {
    pub async fn new(config: Arc<Config>) -> Result<Self> {
        let cache = Arc::new(crate::cache::ModelCache::new(
            config.models.cache_size_mb,
            config.models.max_models_in_memory,
        ));

        // Initialize model loaders for supported formats
        let mut loaders = HashMap::new();
        
        // Add GGML loader
        let ggml_loader = create_model_loader(&ModelFormat::GGML)?;
        loaders.insert(ModelFormat::GGML, ggml_loader);
        
        // Add other format loaders (currently fallback to GGML)
        let onnx_loader = create_model_loader(&ModelFormat::ONNX)?;
        loaders.insert(ModelFormat::ONNX, onnx_loader);
        
        let tflite_loader = create_model_loader(&ModelFormat::TensorFlowLite)?;
        loaders.insert(ModelFormat::TensorFlowLite, tflite_loader);

        Ok(Self {
            config,
            models: Arc::new(RwLock::new(HashMap::new())),
            cache,
            loaders: Arc::new(RwLock::new(loaders)),
        })
    }

    /// Select the best model for a given request
    async fn select_model(&self, request: &MCPRequest, model_id: &ModelId) -> Result<ModelId> {
        let models = self.models.read().await;
        
        // If the requested model is loaded, use it
        if models.contains_key(model_id) {
            return Ok(model_id.clone());
        }
        
        // Find a suitable loaded model that supports the requested method
        for (id, model) in models.iter() {
            if model.metadata.supported_methods.contains(&request.method) {
                debug!(
                    "Using alternative model {} for request {} (method: {})", 
                    id, request.id, request.method
                );
                return Ok(id.clone());
            }
        }
        
        // If no suitable model is loaded, return the requested model ID
        // The caller will load it if necessary
        Ok(model_id.clone())
    }
    
    /// Get model path from configuration
    fn get_model_path(&self, model_id: &ModelId) -> PathBuf {
        let mut path = PathBuf::from(&self.config.models.models_directory);
        path.push(format!("{}.ggml", model_id)); // Default to GGML format
        path
    }
    
    /// Detect model format from file path
    fn detect_model_format(&self, path: &PathBuf) -> ModelFormat {
        if let Some(extension) = path.extension().and_then(|s| s.to_str()) {
            match extension.to_lowercase().as_str() {
                "ggml" | "bin" => ModelFormat::GGML,
                "onnx" => ModelFormat::ONNX,
                "tflite" => ModelFormat::TensorFlowLite,
                _ => ModelFormat::GGML, // Default
            }
        } else {
            ModelFormat::GGML // Default
        }
    }

    /// Execute a model inference using real model loaders
    async fn execute_inference(
        &self,
        request: &MCPRequest,
        model_id: &ModelId,
    ) -> Result<serde_json::Value> {
        debug!("Executing inference with model: {}", model_id);

        // Get the loaded model
        let model = {
            let models = self.models.read().await;
            models.get(model_id)
                .cloned()
                .ok_or_else(|| Error::Model(format!("Model {} not loaded", model_id)))?
        };

        // Get the appropriate loader
        let loaders = self.loaders.read().await;
        let loader = loaders.get(&model.format)
            .ok_or_else(|| Error::Model(format!("No loader available for format {:?}", model.format)))?;

        // Execute inference using the real loader
        let result = loader.execute_inference(&model, &request.method, &request.params).await?;

        // Update model usage statistics
        {
            let mut models = self.models.write().await;
            if let Some(loaded_model) = models.get_mut(model_id) {
                loaded_model.last_used = chrono::Utc::now();
                loaded_model.execution_count += 1;
                
                // Update average inference time if available
                if let Some(inference_time) = result.get("inference_time_ms").and_then(|v| v.as_f64()) {
                    let current_avg = loaded_model.average_inference_time_ms;
                    let count = loaded_model.execution_count as f32;
                    loaded_model.average_inference_time_ms = 
                        (current_avg * (count - 1.0) + inference_time as f32) / count;
                }
            }
        }

        Ok(result)
    }

}

#[async_trait]
impl ModelEngine for StandardModelEngine {
    async fn process_request(
        &self,
        request: &MCPRequest,
        model_id: &ModelId,
    ) -> Result<MCPResponse> {
        debug!("Processing request {} with model {}", request.id, model_id);

        // Ensure model is loaded
        self.load_model(model_id).await?;

        // Select the best model (might be different from requested)
        let selected_model = self.select_model(request, model_id).await?;

        // Execute the inference
        match self.execute_inference(request, &selected_model).await {
            Ok(result) => {
                info!("Request {} processed successfully", request.id);
                Ok(MCPResponse {
                    id: request.id,
                    result: Some(result),
                    error: None,
                    timestamp: chrono::Utc::now(),
                })
            },
            Err(e) => {
                error!("Request {} failed: {}", request.id, e);
                Ok(MCPResponse {
                    id: request.id,
                    result: None,
                    error: Some(mcp_common::MCPError {
                        code: -1,
                        message: e.to_string(),
                        data: None,
                    }),
                    timestamp: chrono::Utc::now(),
                })
            },
        }
    }

    async fn load_model(&self, model_id: &ModelId) -> Result<()> {
        let mut models = self.models.write().await;

        if models.contains_key(model_id) {
            debug!("Model {} already loaded", model_id);
            return Ok(());
        }

        info!("Loading model: {}", model_id);

        // Get model path and detect format
        let model_path = self.get_model_path(model_id);
        let format = self.detect_model_format(&model_path);
        
        info!("Model path: {:?}, detected format: {:?}", model_path, format);

        // Check if model file exists, if not create a dummy file for demo purposes
        if !model_path.exists() {
            warn!("Model file {:?} not found, creating dummy model for demonstration", model_path);
            
            // Create parent directory if it doesn't exist
            if let Some(parent) = model_path.parent() {
                tokio::fs::create_dir_all(parent).await
                    .map_err(|e| Error::Model(format!("Failed to create model directory: {}", e)))?;
            }
            
            // Create a dummy model file (1KB for demo)
            let dummy_data = vec![0u8; 1024];
            tokio::fs::write(&model_path, dummy_data).await
                .map_err(|e| Error::Model(format!("Failed to create dummy model file: {}", e)))?;
        }

        // Get appropriate loader
        let loaders = self.loaders.read().await;
        let loader = loaders.get(&format)
            .ok_or_else(|| Error::Model(format!("No loader available for format {:?}", format)))?;

        // Estimate memory usage first
        let estimated_memory = loader.estimate_memory_usage(&model_path).await?;
        
        // Check memory constraints
        let total_memory_usage: u32 = models.values().map(|m| m.memory_usage_mb).sum();

        if total_memory_usage + estimated_memory > self.config.models.cache_size_mb {
            // Need to unload some models - collect keys first to avoid borrow checker issues
            let mut models_by_usage: Vec<_> = models
                .iter()
                .map(|(id, model)| (id.clone(), model.last_used))
                .collect();
            models_by_usage.sort_by_key(|(_, last_used)| *last_used);

            // Calculate how much memory we need to free
            let memory_needed = (total_memory_usage + estimated_memory)
                .saturating_sub(self.config.models.cache_size_mb);

            let mut freed_memory = 0u32;
            let mut models_to_unload = Vec::new();
            
            // Unload least recently used models
            for (id, _) in models_by_usage {
                if freed_memory >= memory_needed {
                    break;
                }
                if let Some(model) = models.get(&id) {
                    freed_memory += model.memory_usage_mb;
                    models_to_unload.push(id);
                }
            }
            
            // Actually unload the models
            for id in models_to_unload {
                warn!("Unloading model {} to free {}MB memory", id, models.get(&id).map(|m| m.memory_usage_mb).unwrap_or(0));
                
                // Unload using the appropriate loader
                if let Some(model_to_unload) = models.get(&id) {
                    if let Some(unload_loader) = loaders.get(&model_to_unload.format) {
                        if let Err(e) = unload_loader.unload(model_to_unload).await {
                            warn!("Failed to properly unload model {}: {}", id, e);
                        }
                    }
                }
                
                models.remove(&id);
            }
        }

        // Check model count limit
        if models.len() >= self.config.models.max_models_in_memory as usize {
            return Err(Error::Model("Too many models loaded".to_string()));
        }

        // Load the model using the appropriate loader
        let loaded_model = loader.load(model_id, &model_path).await?;

        models.insert(model_id.clone(), loaded_model);
        info!("Model {} loaded successfully ({}MB)", model_id, estimated_memory);

        Ok(())
    }

    async fn unload_model(&self, model_id: &ModelId) -> Result<()> {
        let mut models = self.models.write().await;

        if let Some(model) = models.get(model_id) {
            // Get the appropriate loader and unload the model
            let loaders = self.loaders.read().await;
            if let Some(loader) = loaders.get(&model.format) {
                if let Err(e) = loader.unload(model).await {
                    warn!("Failed to properly unload model {}: {}", model_id, e);
                }
            }
            
            // Remove from our tracking
            models.remove(model_id);
            info!("Model {} unloaded successfully", model_id);
            Ok(())
        } else {
            Err(Error::Model(format!("Model {} not loaded", model_id)))
        }
    }

    async fn health_check(&self) -> Result<ComponentHealth> {
        let models = self.models.read().await;

        let mut health_metrics = HashMap::new();
        let total_memory_usage: u32 = models.values().map(|m| m.memory_usage_mb).sum();
        let memory_usage_percent =
            (total_memory_usage as f32 / self.config.models.cache_size_mb as f32) * 100.0;

        health_metrics.insert("loaded_models".to_string(), models.len() as f32);
        health_metrics.insert("memory_usage_mb".to_string(), total_memory_usage as f32);
        health_metrics.insert("memory_usage_percent".to_string(), memory_usage_percent);
        health_metrics.insert(
            "max_models".to_string(),
            self.config.models.max_models_in_memory as f32,
        );

        let status = if memory_usage_percent > 95.0
            || models.len() >= self.config.models.max_models_in_memory as usize
        {
            HealthLevel::Critical
        } else if memory_usage_percent > 85.0 {
            HealthLevel::Warning
        } else {
            HealthLevel::Healthy
        };

        let message = match status {
            HealthLevel::Healthy => "Model engine is operating normally".to_string(),
            HealthLevel::Warning => "Model engine memory usage is high".to_string(),
            HealthLevel::Critical => "Model engine is at capacity".to_string(),
            HealthLevel::Unknown => "Model engine status unknown".to_string(),
        };

        Ok(ComponentHealth {
            status,
            message,
            last_check: chrono::Utc::now(),
            metrics: health_metrics,
        })
    }

    async fn shutdown(&self) -> Result<()> {
        info!("Shutting down model engine");

        let mut models = self.models.write().await;
        models.clear();

        Ok(())
    }
}
