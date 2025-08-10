//! Advanced multi-model ensemble engine implementation

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

/// Advanced multi-model ensemble engine
pub struct StandardModelEngine {
    config: Arc<Config>,
    models: Arc<RwLock<HashMap<ModelId, LoadedModel>>>,
    cache: Arc<crate::cache::ModelCache>,
    loaders: Arc<RwLock<HashMap<ModelFormat, Box<dyn ModelLoader>>>>,
    ensembles: Arc<RwLock<HashMap<String, ModelEnsemble>>>,
    performance_tracker: Arc<RwLock<ModelPerformanceTracker>>,
}

/// Multi-model ensemble for improved accuracy and reliability
#[derive(Debug, Clone)]
pub struct ModelEnsemble {
    pub name: String,
    pub primary_model: ModelId,
    pub secondary_models: Vec<ModelId>,
    pub ensemble_strategy: EnsembleStrategy,
    pub confidence_threshold: f32,
    pub performance_weights: HashMap<ModelId, f32>,
}

/// Ensemble strategies for combining model outputs
#[derive(Debug, Clone)]
pub enum EnsembleStrategy {
    /// Use fastest model first, fallback to others if confidence is low
    FastestFirst,
    /// Run multiple models and combine outputs based on confidence
    WeightedVoting { min_agreement: f32 },
    /// Use highest-accuracy model for the specific task type
    TaskSpecialized,
    /// Combine outputs using learned weights
    LearnedWeights,
    /// Use different models for different complexity levels
    ComplexityBased { thresholds: Vec<(f32, ModelId)> },
}

/// Track model performance for intelligent ensemble decisions
#[derive(Debug, Default)]
pub struct ModelPerformanceTracker {
    pub accuracy_scores: HashMap<ModelId, Vec<f32>>,
    pub latency_history: HashMap<ModelId, Vec<u64>>,
    pub success_rates: HashMap<ModelId, (u64, u64)>, // (successes, total)
    pub task_specialization: HashMap<String, ModelId>, // task_type -> best_model
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
            ensembles: Arc::new(RwLock::new(HashMap::new())),
            performance_tracker: Arc::new(RwLock::new(ModelPerformanceTracker::default())),
        })
    }

    /// Create a new model ensemble for improved performance
    pub async fn create_ensemble(
        &self,
        name: String,
        primary_model: ModelId,
        secondary_models: Vec<ModelId>,
        strategy: EnsembleStrategy,
    ) -> Result<()> {
        let ensemble = ModelEnsemble {
            name: name.clone(),
            primary_model: primary_model.clone(),
            secondary_models: secondary_models.clone(),
            ensemble_strategy: strategy,
            confidence_threshold: 0.8, // Default threshold
            performance_weights: HashMap::new(),
        };

        let mut ensembles = self.ensembles.write().await;
        ensembles.insert(name.clone(), ensemble);

        info!(
            "Created ensemble '{}' with primary model '{}' and {} secondary models",
            name,
            primary_model,
            secondary_models.len()
        );

        Ok(())
    }

    /// Execute request using ensemble of models for improved accuracy
    async fn execute_with_ensemble(
        &self,
        request: &MCPRequest,
        ensemble_name: &str,
    ) -> Result<MCPResponse> {
        let ensemble = {
            let ensembles = self.ensembles.read().await;
            ensembles.get(ensemble_name)
                .ok_or_else(|| Error::Model(format!("Ensemble '{}' not found", ensemble_name)))?
                .clone()
        };

        match ensemble.ensemble_strategy {
            EnsembleStrategy::FastestFirst => {
                self.execute_fastest_first(&ensemble, request).await
            },
            EnsembleStrategy::WeightedVoting { min_agreement } => {
                self.execute_weighted_voting(&ensemble, request, min_agreement).await
            },
            EnsembleStrategy::TaskSpecialized => {
                self.execute_task_specialized(&ensemble, request).await
            },
            EnsembleStrategy::LearnedWeights => {
                self.execute_learned_weights(&ensemble, request).await
            },
            EnsembleStrategy::ComplexityBased { ref thresholds } => {
                self.execute_complexity_based(&ensemble, request, thresholds).await
            },
        }
    }

    /// Execute using fastest-first strategy with fallback
    async fn execute_fastest_first(
        &self,
        ensemble: &ModelEnsemble,
        request: &MCPRequest,
    ) -> Result<MCPResponse> {
        // Try primary model first
        let start_time = std::time::Instant::now();
        match self.execute_single_model(request, &ensemble.primary_model).await {
            Ok(response) => {
                let latency = start_time.elapsed().as_millis() as u64;
                
                // Check confidence (simplified heuristic)
                let confidence = self.estimate_response_confidence(&response);
                
                if confidence >= ensemble.confidence_threshold {
                    self.update_model_performance(&ensemble.primary_model, confidence, latency, true).await;
                    return Ok(response);
                }
                
                debug!("Primary model confidence too low ({:.2}), trying secondary models", confidence);
            }
            Err(e) => {
                warn!("Primary model failed: {}", e);
                self.update_model_performance(&ensemble.primary_model, 0.0, 0, false).await;
            }
        }

        // Try secondary models
        for model_id in &ensemble.secondary_models {
            let start_time = std::time::Instant::now();
            match self.execute_single_model(request, model_id).await {
                Ok(response) => {
                    let latency = start_time.elapsed().as_millis() as u64;
                    let confidence = self.estimate_response_confidence(&response);
                    
                    self.update_model_performance(model_id, confidence, latency, true).await;
                    return Ok(response);
                }
                Err(e) => {
                    warn!("Secondary model '{}' failed: {}", model_id, e);
                    self.update_model_performance(model_id, 0.0, 0, false).await;
                }
            }
        }

        Err(Error::Model("All models in ensemble failed".to_string()))
    }

    /// Execute using weighted voting strategy
    async fn execute_weighted_voting(
        &self,
        ensemble: &ModelEnsemble,
        request: &MCPRequest,
        min_agreement: f32,
    ) -> Result<MCPResponse> {
        let mut responses = Vec::new();
        let mut models_to_try = vec![ensemble.primary_model.clone()];
        models_to_try.extend(ensemble.secondary_models.iter().cloned());

        // Execute up to 3 models for voting
        for (i, model_id) in models_to_try.iter().take(3).enumerate() {
            match self.execute_single_model(request, model_id).await {
                Ok(response) => {
                    let confidence = self.estimate_response_confidence(&response);
                    responses.push((response, confidence, model_id.clone()));
                    
                    if i == 0 && confidence >= 0.9 {
                        // High confidence from primary model, use it directly
                        return Ok(responses[0].0.clone());
                    }
                }
                Err(e) => {
                    warn!("Model '{}' failed in voting: {}", model_id, e);
                }
            }
        }

        if responses.is_empty() {
            return Err(Error::Model("No models succeeded in voting ensemble".to_string()));
        }

        // Simple voting: return response with highest confidence
        responses.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        let best_response = &responses[0];
        if best_response.1 >= min_agreement {
            Ok(best_response.0.clone())
        } else {
            // If no single response meets agreement threshold, return the best one
            debug!("No consensus reached (best confidence: {:.2}), returning best response", best_response.1);
            Ok(best_response.0.clone())
        }
    }

    /// Execute using task specialization
    async fn execute_task_specialized(
        &self,
        ensemble: &ModelEnsemble,
        request: &MCPRequest,
    ) -> Result<MCPResponse> {
        let task_type = request.method.clone();
        let tracker = self.performance_tracker.read().await;
        
        // Find best model for this task type
        let best_model = tracker.task_specialization.get(&task_type)
            .unwrap_or(&ensemble.primary_model)
            .clone();
        
        drop(tracker);

        // Execute with best model for this task
        match self.execute_single_model(request, &best_model).await {
            Ok(response) => Ok(response),
            Err(_) => {
                // Fallback to primary model if specialized model fails
                self.execute_single_model(request, &ensemble.primary_model).await
            }
        }
    }

    /// Execute using learned weights
    async fn execute_learned_weights(
        &self,
        ensemble: &ModelEnsemble,
        request: &MCPRequest,
    ) -> Result<MCPResponse> {
        let tracker = self.performance_tracker.read().await;
        let mut model_scores = Vec::new();

        // Calculate scores for all models
        for model_id in std::iter::once(&ensemble.primary_model).chain(&ensemble.secondary_models) {
            let success_rate = if let Some((successes, total)) = tracker.success_rates.get(model_id) {
                if *total > 0 { *successes as f32 / *total as f32 } else { 0.5 }
            } else { 0.5 };

            let avg_accuracy = if let Some(scores) = tracker.accuracy_scores.get(model_id) {
                if !scores.is_empty() { 
                    scores.iter().sum::<f32>() / scores.len() as f32 
                } else { 0.5 }
            } else { 0.5 };

            let combined_score = (success_rate * 0.6) + (avg_accuracy * 0.4);
            model_scores.push((model_id.clone(), combined_score));
        }
        
        drop(tracker);

        // Sort by score and try best model
        model_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        
        for (model_id, _score) in model_scores {
            match self.execute_single_model(request, &model_id).await {
                Ok(response) => return Ok(response),
                Err(_) => continue,
            }
        }

        Err(Error::Model("All models failed in learned weights ensemble".to_string()))
    }

    /// Execute using complexity-based routing
    async fn execute_complexity_based(
        &self,
        ensemble: &ModelEnsemble,
        request: &MCPRequest,
        thresholds: &[(f32, ModelId)],
    ) -> Result<MCPResponse> {
        // Simple complexity estimation (could be enhanced)
        let complexity = self.estimate_request_complexity(request);
        
        // Find appropriate model based on complexity
        let selected_model = thresholds.iter()
            .find(|(threshold, _)| complexity >= *threshold)
            .map(|(_, model)| model)
            .unwrap_or(&ensemble.primary_model);

        match self.execute_single_model(request, selected_model).await {
            Ok(response) => Ok(response),
            Err(_) => {
                // Fallback to primary model
                self.execute_single_model(request, &ensemble.primary_model).await
            }
        }
    }

    /// Execute request with a single model
    async fn execute_single_model(&self, request: &MCPRequest, model_id: &ModelId) -> Result<MCPResponse> {
        // This would call the actual model execution logic
        // For now, return a placeholder implementation
        debug!("Executing request with model: {}", model_id);
        
        // TODO: Implement actual model execution
        Ok(MCPResponse {
            id: request.id.clone(),
            result: Some(serde_json::json!({
                "content": format!("Response from model {}", model_id),
                "model_used": model_id,
                "confidence": 0.85
            })),
            error: None,
        })
    }

    /// Estimate response confidence using heuristics
    fn estimate_response_confidence(&self, response: &MCPResponse) -> f32 {
        // Simplified confidence estimation
        // In a real implementation, this would analyze the response content
        if let Some(result) = &response.result {
            if let Some(confidence) = result.get("confidence").and_then(|c| c.as_f64()) {
                return confidence as f32;
            }
        }
        
        // Default confidence if not specified
        0.75
    }

    /// Estimate request complexity for routing decisions
    fn estimate_request_complexity(&self, request: &MCPRequest) -> f32 {
        let mut complexity = 0.0;
        
        // Method-based complexity
        complexity += match request.method.as_str() {
            "completion" => 0.8,
            "chat" => 0.7,
            "embedding" => 0.3,
            _ => 0.5,
        };

        // Parameter complexity
        complexity += (request.params.len() as f32) * 0.05;

        complexity.min(1.0)
    }

    /// Update model performance tracking
    async fn update_model_performance(
        &self,
        model_id: &ModelId,
        accuracy: f32,
        latency_ms: u64,
        success: bool,
    ) {
        let mut tracker = self.performance_tracker.write().await;

        // Update accuracy scores
        tracker.accuracy_scores
            .entry(model_id.clone())
            .or_default()
            .push(accuracy);

        // Keep only recent scores (last 100)
        if let Some(scores) = tracker.accuracy_scores.get_mut(model_id) {
            if scores.len() > 100 {
                scores.drain(0..10);
            }
        }

        // Update latency history
        if latency_ms > 0 {
            tracker.latency_history
                .entry(model_id.clone())
                .or_default()
                .push(latency_ms);

            // Keep only recent latency data
            if let Some(latencies) = tracker.latency_history.get_mut(model_id) {
                if latencies.len() > 100 {
                    latencies.drain(0..10);
                }
            }
        }

        // Update success rates
        let (successes, total) = tracker.success_rates
            .entry(model_id.clone())
            .or_insert((0, 0));
        
        *total += 1;
        if success {
            *successes += 1;
        }
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
        let params_value = serde_json::to_value(&request.params)
            .map_err(|e| Error::Model(format!("Failed to serialize params: {}", e)))?;
        let result = loader.execute_inference(&model, &request.method, &params_value).await?;

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
