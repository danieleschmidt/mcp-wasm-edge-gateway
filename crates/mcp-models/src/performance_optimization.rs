//! Performance optimizations for model processing

/// Optimized processor for vector operations with CPU feature detection
pub struct PerformanceProcessor {
    use_avx2: bool,
    use_sse4: bool,
    vector_size: usize,
}

impl PerformanceProcessor {
    /// Create a new processor with automatic feature detection
    pub fn new() -> Self {
        Self {
            use_avx2: Self::is_avx2_available(),
            use_sse4: Self::is_sse4_available(),
            vector_size: if Self::is_avx2_available() { 8 } else { 4 },
        }
    }

    /// Check if AVX2 is available on the current CPU
    fn is_avx2_available() -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            std::arch::is_x86_feature_detected!("avx2")
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            false
        }
    }

    /// Check if SSE4.1 is available on the current CPU
    fn is_sse4_available() -> bool {
        #[cfg(target_arch = "x86_64")]
        {
            std::arch::is_x86_feature_detected!("sse4.1")
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            false
        }
    }

    /// Optimized vector dot product
    pub fn vector_dot_product(&self, a: &[f32], b: &[f32]) -> f32 {
        assert_eq!(a.len(), b.len(), "Vector lengths must match");
        
        // Use unrolled loops for better performance
        self.optimized_scalar_dot_product(a, b)
    }

    /// Optimized scalar implementation with loop unrolling
    fn optimized_scalar_dot_product(&self, a: &[f32], b: &[f32]) -> f32 {
        let mut sum = 0.0f32;
        let len = a.len();
        
        // Process 4 elements at a time (manual unrolling)
        let chunks = len / 4;
        for i in 0..chunks {
            let start = i * 4;
            sum += a[start] * b[start];
            sum += a[start + 1] * b[start + 1];
            sum += a[start + 2] * b[start + 2];
            sum += a[start + 3] * b[start + 3];
        }

        // Handle remaining elements
        let remainder_start = chunks * 4;
        for i in remainder_start..len {
            sum += a[i] * b[i];
        }

        sum
    }

    /// Optimized matrix-vector multiplication
    pub fn matrix_vector_multiply(&self, matrix: &[Vec<f32>], vector: &[f32]) -> Vec<f32> {
        let rows = matrix.len();
        let mut result = Vec::with_capacity(rows);

        for row in matrix.iter() {
            let dot_product = self.vector_dot_product(row, vector);
            result.push(dot_product);
        }

        result
    }

    /// Optimized element-wise vector operations
    pub fn vector_add(&self, a: &[f32], b: &[f32]) -> Vec<f32> {
        assert_eq!(a.len(), b.len());
        
        let mut result = Vec::with_capacity(a.len());
        let len = a.len();

        // Process 4 elements at a time
        let chunks = len / 4;
        for i in 0..chunks {
            let start = i * 4;
            result.push(a[start] + b[start]);
            result.push(a[start + 1] + b[start + 1]);
            result.push(a[start + 2] + b[start + 2]);
            result.push(a[start + 3] + b[start + 3]);
        }

        // Handle remaining elements
        let remainder_start = chunks * 4;
        for i in remainder_start..len {
            result.push(a[i] + b[i]);
        }

        result
    }

    /// ReLU activation function
    pub fn relu_activation(&self, input: &[f32]) -> Vec<f32> {
        input.iter().map(|&x| x.max(0.0)).collect()
    }

    /// Softmax activation function
    pub fn softmax_activation(&self, input: &[f32]) -> Vec<f32> {
        // Find maximum for numerical stability
        let max_val = input.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        
        // Compute exp(x - max) for each element
        let exp_values: Vec<f32> = input.iter().map(|&x| (x - max_val).exp()).collect();
        
        // Compute sum of exponentials
        let sum_exp: f32 = exp_values.iter().sum();
        
        // Normalize
        exp_values.iter().map(|&x| x / sum_exp).collect()
    }

    /// Get processor capabilities
    pub fn get_capabilities(&self) -> String {
        let mut caps = Vec::new();
        
        if self.use_avx2 {
            caps.push("AVX2");
        }
        if self.use_sse4 {
            caps.push("SSE4.1");
        }
        
        if caps.is_empty() {
            "Optimized-Scalar".to_string()
        } else {
            caps.join(", ")
        }
    }

    /// Benchmark operations
    pub fn benchmark(&self, size: usize) -> BenchmarkResults {
        use std::time::Instant;

        let a: Vec<f32> = (0..size).map(|i| i as f32).collect();
        let b: Vec<f32> = (0..size).map(|i| (i + 1) as f32).collect();

        // Benchmark dot product
        let start = Instant::now();
        let _dot_result = self.vector_dot_product(&a, &b);
        let dot_duration = start.elapsed();

        // Benchmark vector addition
        let start = Instant::now();
        let _add_result = self.vector_add(&a, &b);
        let add_duration = start.elapsed();

        BenchmarkResults {
            vector_size: size,
            dot_product_ns: dot_duration.as_nanos() as u64,
            vector_add_ns: add_duration.as_nanos() as u64,
            operations_per_second: (size as f64) / (dot_duration.as_secs_f64()),
            capabilities: self.get_capabilities(),
        }
    }
}

impl Default for PerformanceProcessor {
    fn default() -> Self {
        Self::new()
    }
}

/// Benchmark results
#[derive(Debug, Clone)]
pub struct BenchmarkResults {
    pub vector_size: usize,
    pub dot_product_ns: u64,
    pub vector_add_ns: u64,
    pub operations_per_second: f64,
    pub capabilities: String,
}

/// Memory pool for efficient buffer management
pub struct MemoryPool<T> {
    buffers: Vec<Vec<T>>,
    buffer_size: usize,
    pool_size: usize,
}

impl<T: Clone + Default> MemoryPool<T> {
    /// Create a new memory pool
    pub fn new(buffer_size: usize, pool_size: usize) -> Self {
        let mut buffers = Vec::with_capacity(pool_size);
        for _ in 0..pool_size {
            let mut buffer = Vec::with_capacity(buffer_size);
            buffer.resize(buffer_size, T::default());
            buffers.push(buffer);
        }
        
        Self {
            buffers,
            buffer_size,
            pool_size,
        }
    }

    /// Get a buffer from the pool
    pub fn get_buffer(&mut self) -> Option<Vec<T>> {
        self.buffers.pop()
    }

    /// Return a buffer to the pool
    pub fn return_buffer(&mut self, mut buffer: Vec<T>) {
        if buffer.len() == self.buffer_size && self.buffers.len() < self.pool_size {
            buffer.clear();
            buffer.resize(self.buffer_size, T::default());
            self.buffers.push(buffer);
        }
    }

    /// Get current pool utilization
    pub fn utilization(&self) -> f32 {
        1.0 - (self.buffers.len() as f32 / self.pool_size as f32)
    }
}

/// Cache-friendly matrix representation
#[derive(Debug, Clone)]
pub struct OptimizedMatrix {
    data: Vec<f32>,
    rows: usize,
    cols: usize,
    row_major: bool,
}

impl OptimizedMatrix {
    /// Create a new matrix
    pub fn new(rows: usize, cols: usize, row_major: bool) -> Self {
        Self {
            data: vec![0.0; rows * cols],
            rows,
            cols,
            row_major,
        }
    }

    /// Create from existing data
    pub fn from_data(data: Vec<f32>, rows: usize, cols: usize, row_major: bool) -> Self {
        assert_eq!(data.len(), rows * cols);
        Self {
            data,
            rows,
            cols,
            row_major,
        }
    }

    /// Get element at position (row, col)
    pub fn get(&self, row: usize, col: usize) -> f32 {
        if self.row_major {
            self.data[row * self.cols + col]
        } else {
            self.data[col * self.rows + row]
        }
    }

    /// Set element at position (row, col)
    pub fn set(&mut self, row: usize, col: usize, value: f32) {
        if self.row_major {
            self.data[row * self.cols + col] = value;
        } else {
            self.data[col * self.rows + row] = value;
        }
    }

    /// Get a row as a slice (only for row-major matrices)
    pub fn get_row(&self, row: usize) -> Option<&[f32]> {
        if self.row_major {
            let start = row * self.cols;
            Some(&self.data[start..start + self.cols])
        } else {
            None
        }
    }

    /// Matrix-vector multiplication
    pub fn multiply_vector(&self, vector: &[f32], processor: &PerformanceProcessor) -> Vec<f32> {
        assert_eq!(vector.len(), self.cols);
        
        let mut result = Vec::with_capacity(self.rows);
        
        if self.row_major {
            for row in 0..self.rows {
                let row_data = self.get_row(row).unwrap();
                let dot_product = processor.vector_dot_product(row_data, vector);
                result.push(dot_product);
            }
        } else {
            result.resize(self.rows, 0.0);
            for col in 0..self.cols {
                let scalar = vector[col];
                for row in 0..self.rows {
                    result[row] += self.get(row, col) * scalar;
                }
            }
        }
        
        result
    }

    /// Get matrix dimensions
    pub fn dimensions(&self) -> (usize, usize) {
        (self.rows, self.cols)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_processor_creation() {
        let processor = PerformanceProcessor::new();
        println!("Capabilities: {}", processor.get_capabilities());
        assert!(processor.vector_size > 0);
    }

    #[test]
    fn test_dot_product() {
        let processor = PerformanceProcessor::new();
        let a = vec![1.0, 2.0, 3.0, 4.0];
        let b = vec![2.0, 3.0, 4.0, 5.0];
        
        let result = processor.vector_dot_product(&a, &b);
        let expected = 40.0;
        
        assert!((result - expected).abs() < 0.001);
    }

    #[test]
    fn test_vector_add() {
        let processor = PerformanceProcessor::new();
        let a = vec![1.0, 2.0, 3.0, 4.0];
        let b = vec![2.0, 3.0, 4.0, 5.0];
        
        let result = processor.vector_add(&a, &b);
        let expected = vec![3.0, 5.0, 7.0, 9.0];
        
        for (r, e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 0.001);
        }
    }

    #[test]
    fn test_memory_pool() {
        let mut pool = MemoryPool::<f32>::new(100, 5);
        
        let buffer = pool.get_buffer().unwrap();
        assert_eq!(buffer.len(), 100);
        
        let utilization = pool.utilization();
        assert!(utilization > 0.0);
        
        pool.return_buffer(buffer);
    }
}