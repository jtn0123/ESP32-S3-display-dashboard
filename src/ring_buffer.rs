// Efficient ring buffer implementation for performance tracking
use std::time::Duration;

/// Fixed-size ring buffer for storing frame times without allocations
pub struct RingBuffer<T, const N: usize> {
    buffer: [T; N],
    head: usize,
    len: usize,
}

impl<T: Default + Copy, const N: usize> RingBuffer<T, N> {
    pub fn new() -> Self {
        Self {
            buffer: [T::default(); N],
            head: 0,
            len: 0,
        }
    }
    
    /// Push a new value, overwriting the oldest if full
    pub fn push(&mut self, value: T) {
        self.buffer[self.head] = value;
        self.head = (self.head + 1) % N;
        if self.len < N {
            self.len += 1;
        }
    }
    
    /// Get the number of elements in the buffer
    pub fn len(&self) -> usize {
        self.len
    }
    
    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }
    
    /// Get an iterator over the elements (oldest to newest)
    pub fn iter(&self) -> RingBufferIter<T, N> {
        RingBufferIter {
            buffer: &self.buffer,
            head: self.head,
            len: self.len,
            index: 0,
        }
    }
    
    /// Get the last N elements (newest)
    pub fn last_n(&self, n: usize) -> impl Iterator<Item = &T> {
        let start = if self.len <= n {
            0
        } else {
            self.len - n
        };
        
        self.iter().skip(start)
    }
    
    /// Clear the buffer
    pub fn clear(&mut self) {
        self.head = 0;
        self.len = 0;
    }
}

/// Iterator for ring buffer
pub struct RingBufferIter<'a, T, const N: usize> {
    buffer: &'a [T; N],
    head: usize,
    len: usize,
    index: usize,
}

impl<'a, T, const N: usize> Iterator for RingBufferIter<'a, T, N> {
    type Item = &'a T;
    
    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.len {
            return None;
        }
        
        // Calculate actual position in buffer
        let start = if self.len < N {
            0
        } else {
            self.head
        };
        
        let pos = (start + self.index) % N;
        self.index += 1;
        
        Some(&self.buffer[pos])
    }
}

/// Specialized ring buffer for Duration values with built-in statistics
pub struct DurationRingBuffer<const N: usize> {
    buffer: RingBuffer<Duration, N>,
    sum: Duration,
    min: Duration,
    max: Duration,
}

impl<const N: usize> DurationRingBuffer<N> {
    pub fn new() -> Self {
        Self {
            buffer: RingBuffer::new(),
            sum: Duration::ZERO,
            min: Duration::MAX,
            max: Duration::ZERO,
        }
    }
    
    /// Push a new duration and update statistics incrementally
    pub fn push(&mut self, duration: Duration) {
        // If buffer is full, subtract the oldest value from sum
        if self.buffer.len() == N {
            let oldest_index = if self.buffer.len < N {
                0
            } else {
                self.buffer.head
            };
            self.sum = self.sum.saturating_sub(self.buffer.buffer[oldest_index]);
        }
        
        // Add new value
        self.buffer.push(duration);
        self.sum = self.sum.saturating_add(duration);
        
        // Update min/max
        if duration < self.min {
            self.min = duration;
        }
        if duration > self.max {
            self.max = duration;
        }
        
        // Recalculate min/max if needed (when buffer wraps)
        if self.buffer.len() == N {
            self.recalculate_min_max();
        }
    }
    
    /// Get average duration
    pub fn average(&self) -> Option<Duration> {
        if self.buffer.is_empty() {
            return None;
        }
        
        Some(self.sum / self.buffer.len() as u32)
    }
    
    /// Get average of last N samples
    pub fn average_last_n(&self, n: usize) -> Option<Duration> {
        let samples: Vec<Duration> = self.buffer.last_n(n).copied().collect();
        if samples.is_empty() {
            return None;
        }
        
        let sum: Duration = samples.iter().sum();
        Some(sum / samples.len() as u32)
    }
    
    /// Get minimum duration
    pub fn min(&self) -> Option<Duration> {
        if self.buffer.is_empty() {
            None
        } else {
            Some(self.min)
        }
    }
    
    /// Get maximum duration
    pub fn max(&self) -> Option<Duration> {
        if self.buffer.is_empty() {
            None
        } else {
            Some(self.max)
        }
    }
    
    /// Get number of samples
    pub fn len(&self) -> usize {
        self.buffer.len()
    }
    
    /// Clear all samples
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.sum = Duration::ZERO;
        self.min = Duration::MAX;
        self.max = Duration::ZERO;
    }
    
    /// Recalculate min/max when buffer wraps
    fn recalculate_min_max(&mut self) {
        self.min = Duration::MAX;
        self.max = Duration::ZERO;
        
        for &duration in self.buffer.iter() {
            if duration < self.min {
                self.min = duration;
            }
            if duration > self.max {
                self.max = duration;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ring_buffer_basic() {
        let mut buffer: RingBuffer<u32, 3> = RingBuffer::new();
        
        assert_eq!(buffer.len(), 0);
        
        buffer.push(1);
        buffer.push(2);
        buffer.push(3);
        
        assert_eq!(buffer.len(), 3);
        
        let values: Vec<u32> = buffer.iter().copied().collect();
        assert_eq!(values, vec![1, 2, 3]);
        
        // Push one more, should overwrite oldest
        buffer.push(4);
        
        let values: Vec<u32> = buffer.iter().copied().collect();
        assert_eq!(values, vec![2, 3, 4]);
    }
    
    #[test]
    fn test_duration_ring_buffer() {
        let mut buffer: DurationRingBuffer<3> = DurationRingBuffer::new();
        
        buffer.push(Duration::from_millis(10));
        buffer.push(Duration::from_millis(20));
        buffer.push(Duration::from_millis(30));
        
        assert_eq!(buffer.average(), Some(Duration::from_millis(20)));
        assert_eq!(buffer.min(), Some(Duration::from_millis(10)));
        assert_eq!(buffer.max(), Some(Duration::from_millis(30)));
        
        // Overwrite oldest
        buffer.push(Duration::from_millis(5));
        
        assert_eq!(buffer.average(), Some(Duration::from_millis(18))); // (20+30+5)/3
        assert_eq!(buffer.min(), Some(Duration::from_millis(5)));
    }
}