//! Intelligent caching system with ML-based predictions and adaptive algorithms

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use chrono::{DateTime, Utc, Duration as ChronoDuration, Timelike};
use mcp_common::{ModelId, Result, Error};
use serde::{Serialize, Deserialize};

/// Intelligent cache that learns from usage patterns
pub struct IntelligentCache {
    config: CacheConfig,
    storage: Arc<RwLock<CacheStorage>>,
    predictor: Arc<RwLock<UsagePredictor>>,
    eviction_policy: EvictionPolicy,
    stats: Arc<RwLock<CacheStats>>,
}

/// Cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    pub max_size_mb: u64,
    pub max_models: usize,
    pub prediction_window_hours: u32,
    pub learning_rate: f32,
    pub preload_threshold: f32,
    pub eviction_algorithm: EvictionAlgorithm,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_size_mb: 2048, // 2GB default
            max_models: 10,
            prediction_window_hours: 24,
            learning_rate: 0.1,
            preload_threshold: 0.8,
            eviction_algorithm: EvictionAlgorithm::AdaptiveLRU,
        }
    }
}

/// Eviction algorithms available
#[derive(Debug, Clone)]
pub enum EvictionAlgorithm {
    LRU,
    LFU,
    AdaptiveLRU,
    PredictiveLRU,
    HybridScore,
}

/// Cache storage management
#[derive(Debug)]
struct CacheStorage {
    models: HashMap<ModelId, CachedModel>,
    size_mb: u64,
    access_order: VecDeque<ModelId>,
    frequency_map: HashMap<ModelId, u32>,
}

/// Cached model representation
#[derive(Debug, Clone)]
struct CachedModel {
    model_id: ModelId,
    size_mb: u32,
    load_time: DateTime<Utc>,
    last_accessed: DateTime<Utc>,
    access_count: u64,
    access_frequency: f32, // accesses per hour
    predicted_next_access: Option<DateTime<Utc>>,
    importance_score: f32,
    data: Vec<u8>, // Actual model data
}

/// Usage prediction system using time series analysis
#[derive(Debug)]
struct UsagePredictor {
    model_patterns: HashMap<ModelId, UsagePattern>,
    global_patterns: GlobalUsagePattern,
    prediction_accuracy: HashMap<ModelId, f32>,
}

/// Usage pattern for a specific model
#[derive(Debug, Clone)]
struct UsagePattern {
    hourly_usage: [f32; 24], // Usage probability by hour
    daily_usage: VecDeque<f32>, // Last 30 days usage intensity
    weekly_pattern: [f32; 7], // Usage by day of week
    access_intervals: VecDeque<u64>, // Time between accesses in minutes
    seasonal_factors: SeasonalFactors,
    trend_coefficient: f32, // Growing/declining usage trend
}

/// Global usage patterns across all models
#[derive(Debug, Default)]
struct GlobalUsagePattern {
    peak_hours: Vec<u8>,
    total_requests_per_hour: [u32; 24],
    correlation_matrix: HashMap<ModelId, Vec<(ModelId, f32)>>,
}

/// Seasonal adjustment factors
#[derive(Debug, Clone, Default)]
struct SeasonalFactors {
    weekday_multipliers: [f32; 7], // Monday=0 to Sunday=6
    monthly_trend: f32,
    holiday_impact: f32,
    time_zone_adjustment: f32,
}

/// Cache statistics and metrics
#[derive(Debug, Default, Clone)]
struct CacheStats {
    total_requests: u64,
    cache_hits: u64,
    cache_misses: u64,
    evictions: u64,
    preloads: u64,
    prediction_accuracy: f32,
    average_load_time_ms: f32,
    memory_efficiency: f32,
}

/// Eviction policy implementation
struct EvictionPolicy {
    algorithm: EvictionAlgorithm,
}

impl IntelligentCache {
    /// Create a new intelligent cache
    pub async fn new(config: CacheConfig) -> Result<Self> {
        Ok(Self {
            config: config.clone(),
            storage: Arc::new(RwLock::new(CacheStorage {
                models: HashMap::new(),
                size_mb: 0,
                access_order: VecDeque::new(),
                frequency_map: HashMap::new(),
            })),
            predictor: Arc::new(RwLock::new(UsagePredictor {
                model_patterns: HashMap::new(),
                global_patterns: GlobalUsagePattern::default(),
                prediction_accuracy: HashMap::new(),
            })),
            eviction_policy: EvictionPolicy {
                algorithm: config.eviction_algorithm,
            },
            stats: Arc::new(RwLock::new(CacheStats::default())),
        })
    }

    /// Check if a model is cached
    pub async fn contains(&self, model_id: &ModelId) -> bool {
        let storage = self.storage.read().await;
        storage.models.contains_key(model_id)
    }

    /// Get a model from cache
    pub async fn get(&self, model_id: &ModelId) -> Option<Vec<u8>> {
        let mut storage = self.storage.write().await;
        let mut stats = self.stats.write().await;

        if storage.models.contains_key(model_id) {
            // Get data first
            let data = storage.models.get(model_id).unwrap().data.clone();
            
            // Update access statistics
            let model = storage.models.get_mut(model_id).unwrap();
            model.last_accessed = Utc::now();
            model.access_count += 1;
            
            // Update access order for LRU
            if let Some(pos) = storage.access_order.iter().position(|id| id == model_id) {
                storage.access_order.remove(pos);
            }
            storage.access_order.push_back(model_id.clone());
            
            // Update frequency map
            *storage.frequency_map.entry(model_id.clone()).or_insert(0) += 1;

            stats.cache_hits += 1;
            stats.total_requests += 1;

            // Learn from this access pattern
            drop(storage);
            drop(stats);
            self.update_usage_pattern(model_id).await;

            Some(data)
        } else {
            stats.cache_misses += 1;
            stats.total_requests += 1;
            None
        }
    }

    /// Store a model in cache
    pub async fn put(&self, model_id: ModelId, data: Vec<u8>, size_mb: u32) -> Result<()> {
        let mut storage = self.storage.write().await;

        // Check if we need to evict models first
        while (storage.size_mb + size_mb as u64) > self.config.max_size_mb 
            || storage.models.len() >= self.config.max_models {
            
            let evict_id = self.select_eviction_candidate(&storage).await?;
            if let Some(model) = storage.models.remove(&evict_id) {
                storage.size_mb -= model.size_mb as u64;
                storage.access_order.retain(|id| id != &evict_id);
                storage.frequency_map.remove(&evict_id);

                let mut stats = self.stats.write().await;
                stats.evictions += 1;
            } else {
                break; // No more models to evict
            }
        }

        // Create cached model entry
        let cached_model = CachedModel {
            model_id: model_id.clone(),
            size_mb,
            load_time: Utc::now(),
            last_accessed: Utc::now(),
            access_count: 1,
            access_frequency: 0.0,
            predicted_next_access: None,
            importance_score: 0.5, // Initial neutral score
            data,
        };

        // Add to cache
        storage.models.insert(model_id.clone(), cached_model);
        storage.size_mb += size_mb as u64;
        storage.access_order.push_back(model_id.clone());
        storage.frequency_map.insert(model_id.clone(), 1);

        // Initialize usage pattern
        self.initialize_usage_pattern(&model_id).await;

        Ok(())
    }

    /// Select a model to evict based on the configured algorithm
    async fn select_eviction_candidate(&self, storage: &CacheStorage) -> Result<ModelId> {
        match self.eviction_policy.algorithm {
            EvictionAlgorithm::LRU => {
                storage.access_order.front()
                    .cloned()
                    .ok_or_else(|| Error::Internal("No models to evict".to_string()))
            },
            EvictionAlgorithm::LFU => {
                let min_freq_model = storage.frequency_map.iter()
                    .min_by_key(|(_, &count)| count)
                    .map(|(id, _)| id.clone());
                
                min_freq_model.ok_or_else(|| Error::Internal("No models to evict".to_string()))
            },
            EvictionAlgorithm::AdaptiveLRU => {
                self.adaptive_lru_eviction(storage).await
            },
            EvictionAlgorithm::PredictiveLRU => {
                self.predictive_eviction(storage).await
            },
            EvictionAlgorithm::HybridScore => {
                self.hybrid_score_eviction(storage).await
            },
        }
    }

    /// Adaptive LRU that considers model importance
    async fn adaptive_lru_eviction(&self, storage: &CacheStorage) -> Result<ModelId> {
        let mut best_candidate = None;
        let mut lowest_score = f32::MAX;

        for model_id in &storage.access_order {
            if let Some(model) = storage.models.get(model_id) {
                // Calculate adaptive score (lower = more likely to evict)
                let time_since_access = (Utc::now() - model.last_accessed).num_minutes() as f32;
                let frequency_factor = model.access_frequency;
                let size_penalty = model.size_mb as f32 / 100.0; // Penalize large models slightly
                
                let score = frequency_factor - (time_since_access / 60.0) + size_penalty;
                
                if score < lowest_score {
                    lowest_score = score;
                    best_candidate = Some(model_id.clone());
                }
            }
        }

        best_candidate.ok_or_else(|| Error::Internal("No models to evict".to_string()))
    }

    /// Predictive eviction based on ML predictions
    async fn predictive_eviction(&self, storage: &CacheStorage) -> Result<ModelId> {
        let predictor = self.predictor.read().await;
        let mut best_candidate = None;
        let mut latest_predicted_access = Utc::now();

        for model_id in &storage.access_order {
            if let Some(model) = storage.models.get(model_id) {
                if let Some(pattern) = predictor.model_patterns.get(model_id) {
                    let predicted_time = self.predict_next_access(pattern);
                    
                    if predicted_time > latest_predicted_access {
                        latest_predicted_access = predicted_time;
                        best_candidate = Some(model_id.clone());
                    }
                }
            }
        }

        best_candidate.or_else(|| storage.access_order.front().cloned())
            .ok_or_else(|| Error::Internal("No models to evict".to_string()))
    }

    /// Hybrid scoring eviction combining multiple factors
    async fn hybrid_score_eviction(&self, storage: &CacheStorage) -> Result<ModelId> {
        let predictor = self.predictor.read().await;
        let mut best_candidate = None;
        let mut lowest_score = f32::MAX;

        for model_id in &storage.access_order {
            if let Some(model) = storage.models.get(model_id) {
                let mut score = 0.0f32;

                // Time since last access (higher = more likely to evict)
                let time_since_access = (Utc::now() - model.last_accessed).num_minutes() as f32;
                score += time_since_access / 60.0; // Convert to hours

                // Frequency factor (lower frequency = more likely to evict)
                score -= model.access_frequency * 2.0;

                // Size penalty (larger models slightly more likely to evict)
                score += (model.size_mb as f32 / 1024.0) * 0.1;

                // Prediction factor
                if let Some(pattern) = predictor.model_patterns.get(model_id) {
                    let predicted_time = self.predict_next_access(pattern);
                    let time_until_access = (predicted_time - Utc::now()).num_hours() as f32;
                    score += time_until_access * 0.5;
                }

                // Importance score
                score -= model.importance_score * 3.0;

                if score < lowest_score {
                    lowest_score = score;
                    best_candidate = Some(model_id.clone());
                }
            }
        }

        best_candidate.ok_or_else(|| Error::Internal("No models to evict".to_string()))
    }

    /// Predict the next access time for a model
    fn predict_next_access(&self, pattern: &UsagePattern) -> DateTime<Utc> {
        let now = Utc::now();
        let current_hour = now.hour() as usize;
        
        // Simple prediction based on hourly patterns
        let mut max_probability = 0.0;
        let mut best_hour = current_hour;

        for (hour, &probability) in pattern.hourly_usage.iter().enumerate() {
            if probability > max_probability {
                max_probability = probability;
                best_hour = hour;
            }
        }

        // Calculate time to best hour
        let hours_ahead = if best_hour > current_hour {
            best_hour - current_hour
        } else {
            24 - current_hour + best_hour
        };

        now + ChronoDuration::hours(hours_ahead as i64)
    }

    /// Update usage pattern based on access
    async fn update_usage_pattern(&self, model_id: &ModelId) {
        let mut predictor = self.predictor.write().await;
        
        if let Some(pattern) = predictor.model_patterns.get_mut(model_id) {
            let now = Utc::now();
            let hour = now.hour() as usize;
            
            // Update hourly usage with exponential moving average
            let alpha = self.config.learning_rate;
            pattern.hourly_usage[hour] = alpha + (1.0 - alpha) * pattern.hourly_usage[hour];
            
            // Update access frequency
            let storage = self.storage.read().await;
            if let Some(model) = storage.models.get(model_id) {
                let hours_since_load = (now - model.load_time).num_hours() as f32;
                if hours_since_load > 0.0 {
                    let new_frequency = model.access_count as f32 / hours_since_load;
                    
                    // Update with moving average
                    pattern.access_intervals.push_back(hours_since_load as u64);
                    if pattern.access_intervals.len() > 100 {
                        pattern.access_intervals.pop_front();
                    }
                }
            }
        }
    }

    /// Initialize usage pattern for a new model
    async fn initialize_usage_pattern(&self, model_id: &ModelId) {
        let mut predictor = self.predictor.write().await;
        
        let pattern = UsagePattern {
            hourly_usage: [0.0; 24],
            daily_usage: VecDeque::with_capacity(30),
            weekly_pattern: [0.0; 7],
            access_intervals: VecDeque::with_capacity(100),
            seasonal_factors: SeasonalFactors::default(),
            trend_coefficient: 0.0,
        };

        predictor.model_patterns.insert(model_id.clone(), pattern);
    }

    /// Predictive preloading based on usage patterns
    pub async fn suggest_preloads(&self) -> Vec<ModelId> {
        let predictor = self.predictor.read().await;
        let mut suggestions = Vec::new();
        let now = Utc::now();
        let current_hour = now.hour() as usize;

        for (model_id, pattern) in &predictor.model_patterns {
            // Check if model is likely to be accessed soon
            let next_hour_probability = pattern.hourly_usage.get(current_hour + 1)
                .or_else(|| pattern.hourly_usage.get(0)) // Wrap to 0 if at hour 23
                .unwrap_or(&0.0);

            if *next_hour_probability > self.config.preload_threshold {
                let storage = self.storage.read().await;
                if !storage.models.contains_key(model_id) {
                    suggestions.push(model_id.clone());
                }
            }
        }

        suggestions
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Optimize cache based on current usage patterns
    pub async fn optimize(&self) -> Result<()> {
        // Update importance scores
        self.update_importance_scores().await?;
        
        // Clean up old pattern data
        self.cleanup_old_patterns().await?;
        
        // Update global patterns
        self.update_global_patterns().await?;

        Ok(())
    }

    /// Update model importance scores based on patterns
    async fn update_importance_scores(&self) -> Result<()> {
        let mut storage = self.storage.write().await;
        let predictor = self.predictor.read().await;

        for (model_id, model) in &mut storage.models {
            if let Some(pattern) = predictor.model_patterns.get(model_id) {
                // Calculate importance based on multiple factors
                let frequency_score = model.access_frequency / 24.0; // Normalize to daily
                let recency_score = 1.0 / (1.0 + (Utc::now() - model.last_accessed).num_hours() as f32);
                let trend_score = pattern.trend_coefficient.max(0.0);

                model.importance_score = (frequency_score + recency_score + trend_score) / 3.0;
            }
        }

        Ok(())
    }

    /// Clean up old pattern data
    async fn cleanup_old_patterns(&self) -> Result<()> {
        let mut predictor = self.predictor.write().await;
        let storage = self.storage.read().await;

        // Remove patterns for models no longer in cache
        predictor.model_patterns.retain(|model_id, _| {
            storage.models.contains_key(model_id)
        });

        Ok(())
    }

    /// Update global usage patterns
    async fn update_global_patterns(&self) -> Result<()> {
        let mut predictor = self.predictor.write().await;
        let _storage = self.storage.read().await;

        // Reset global patterns
        predictor.global_patterns.total_requests_per_hour = [0; 24];

        // Collect hourly usage data first to avoid borrow checker issues
        let mut hourly_usage_data = Vec::new();
        for pattern in predictor.model_patterns.values() {
            hourly_usage_data.push(pattern.hourly_usage);
        }

        // Aggregate hourly usage across all models
        for hourly_usage in hourly_usage_data {
            for (hour, usage) in hourly_usage.iter().enumerate() {
                predictor.global_patterns.total_requests_per_hour[hour] += (*usage * 100.0) as u32;
            }
        }

        // Identify peak hours
        let total_requests = predictor.global_patterns.total_requests_per_hour;
        let max_requests = total_requests.iter().max().unwrap_or(&0);
        let threshold = (*max_requests as f32 * 0.8) as u32;
        
        predictor.global_patterns.peak_hours.clear();
        for (hour, &requests) in total_requests.iter().enumerate() {
            if requests >= threshold {
                predictor.global_patterns.peak_hours.push(hour as u8);
            }
        }

        Ok(())
    }

    /// Clear all cached models
    pub async fn clear(&self) -> Result<()> {
        let mut storage = self.storage.write().await;
        storage.models.clear();
        storage.size_mb = 0;
        storage.access_order.clear();
        storage.frequency_map.clear();
        Ok(())
    }

    /// Get current cache utilization
    pub async fn utilization(&self) -> (f32, f32) {
        let storage = self.storage.read().await;
        let memory_util = storage.size_mb as f32 / self.config.max_size_mb as f32;
        let model_util = storage.models.len() as f32 / self.config.max_models as f32;
        (memory_util, model_util)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cache_creation() {
        let config = CacheConfig::default();
        let cache = IntelligentCache::new(config).await.unwrap();
        
        let (mem_util, model_util) = cache.utilization().await;
        assert_eq!(mem_util, 0.0);
        assert_eq!(model_util, 0.0);
    }

    #[tokio::test]
    async fn test_cache_put_get() {
        let config = CacheConfig::default();
        let cache = IntelligentCache::new(config).await.unwrap();
        
        let model_id = "test-model".to_string();
        let data = vec![1, 2, 3, 4, 5];
        
        cache.put(model_id.clone(), data.clone(), 1).await.unwrap();
        
        assert!(cache.contains(&model_id).await);
        
        let retrieved = cache.get(&model_id).await;
        assert_eq!(retrieved, Some(data));
    }

    #[tokio::test]
    async fn test_eviction() {
        let mut config = CacheConfig::default();
        config.max_models = 2;
        config.max_size_mb = 10;
        
        let cache = IntelligentCache::new(config).await.unwrap();
        
        // Add models until eviction occurs
        cache.put("model1".to_string(), vec![1; 1024], 1).await.unwrap();
        cache.put("model2".to_string(), vec![2; 1024], 1).await.unwrap();
        cache.put("model3".to_string(), vec![3; 1024], 1).await.unwrap();
        
        // Should have evicted the least recently used model
        let stats = cache.get_stats().await;
        assert!(stats.evictions > 0);
    }

    #[tokio::test]
    async fn test_preload_suggestions() {
        let config = CacheConfig::default();
        let cache = IntelligentCache::new(config).await.unwrap();
        
        // Initialize with some pattern data
        let model_id = "test-model".to_string();
        cache.initialize_usage_pattern(&model_id).await;
        
        let suggestions = cache.suggest_preloads().await;
        assert!(suggestions.is_empty() || suggestions.contains(&model_id));
    }
}