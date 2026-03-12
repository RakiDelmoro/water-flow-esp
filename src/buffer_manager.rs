use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// A single flow reading with boot-relative timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferedReading {
    /// Monotonic timestamp (milliseconds since boot)
    pub boot_timestamp_ms: u64,
    /// Number of pulses detected in this interval
    pub pulse: u32,
}

/// Thread-safe buffer for storing flow readings during WiFi/MQTT outage
pub struct PulseBuffer {
    readings: VecDeque<BufferedReading>,
    max_capacity: usize,
}

impl PulseBuffer {
    /// Create a new buffer with specified maximum capacity
    pub fn new(max_capacity: usize) -> Self {
        Self {
            readings: VecDeque::with_capacity(max_capacity),
            max_capacity,
        }
    }

    /// Add a reading to the buffer
    /// If buffer is full, removes the oldest reading to make space
    pub fn push(&mut self, reading: BufferedReading) {
        if self.readings.len() >= self.max_capacity {
            self.readings.pop_front();
        }
        self.readings.push_back(reading);
    }

    /// Drain all readings from the buffer, returning them in order (oldest to newest)
    /// After this call, the buffer is empty
    #[allow(dead_code)]
    pub fn drain(&mut self) -> Vec<BufferedReading> {
        self.readings.drain(..).collect()
    }

    /// Pop the oldest reading from the front of the buffer
    pub fn pop_front(&mut self) -> Option<BufferedReading> {
        self.readings.pop_front()
    }

    /// Push a reading to the front of the buffer
    /// Note: This does not respect capacity; consider capacity management at call site if needed
    pub fn push_front(&mut self, reading: BufferedReading) {
        self.readings.push_front(reading);
    }

    /// Get the number of buffered readings
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.readings.len()
    }

    /// Check if buffer is empty
    pub fn is_empty(&self) -> bool {
        self.readings.is_empty()
    }
}
