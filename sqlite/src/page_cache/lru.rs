use std::collections::{HashMap, VecDeque};
use std::hash::Hash;

/// LRU (Least Recently Used) cache implementation
/// 
/// This provides a generic LRU cache that can be used as part of the
/// coalgebraic page cache system.
#[derive(Debug)]
pub struct LruCache<K, V> {
    capacity: usize,
    cache: HashMap<K, V>,
    order: VecDeque<K>,
}

impl<K, V> LruCache<K, V>
where
    K: Clone + Eq + Hash,
{
    /// Create new LRU cache with given capacity
    pub fn new(capacity: usize) -> Self {
        LruCache {
            capacity,
            cache: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    /// Get value by key, updating access order
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if self.cache.contains_key(key) {
            self.update_access_order(key);
            self.cache.get(key)
        } else {
            None
        }
    }

    /// Get mutable value by key, updating access order
    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        if self.cache.contains_key(key) {
            self.update_access_order(key);
            self.cache.get_mut(key)
        } else {
            None
        }
    }

    /// Insert key-value pair, evicting LRU item if necessary
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        // If key already exists, update value and access order
        if self.cache.contains_key(&key) {
            self.update_access_order(&key);
            return self.cache.insert(key, value);
        }

        // If at capacity, evict LRU item
        if self.cache.len() >= self.capacity {
            self.evict_lru();
        }

        // Insert new item
        self.order.push_back(key.clone());
        self.cache.insert(key, value)
    }

    /// Remove key-value pair
    pub fn remove(&mut self, key: &K) -> Option<V> {
        if let Some(value) = self.cache.remove(key) {
            self.order.retain(|k| k != key);
            Some(value)
        } else {
            None
        }
    }

    /// Check if key exists
    pub fn contains(&self, key: &K) -> bool {
        self.cache.contains_key(key)
    }

    /// Get current size
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    /// Get capacity
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Check if cache is full
    pub fn is_full(&self) -> bool {
        self.cache.len() >= self.capacity
    }

    /// Get utilization ratio (0.0 to 1.0)
    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 {
            0.0
        } else {
            self.cache.len() as f64 / self.capacity as f64
        }
    }

    /// Clear all entries
    pub fn clear(&mut self) {
        self.cache.clear();
        self.order.clear();
    }

    /// Get least recently used key
    pub fn lru_key(&self) -> Option<&K> {
        self.order.front()
    }

    /// Get most recently used key
    pub fn mru_key(&self) -> Option<&K> {
        self.order.back()
    }

    /// Get all keys in access order (LRU to MRU)
    pub fn keys_by_access_order(&self) -> Vec<&K> {
        self.order.iter().collect()
    }

    /// Get all keys in reverse access order (MRU to LRU)
    pub fn keys_by_reverse_access_order(&self) -> Vec<&K> {
        self.order.iter().rev().collect()
    }

    /// Peek at value without updating access order
    pub fn peek(&self, key: &K) -> Option<&V> {
        self.cache.get(key)
    }

    /// Resize cache capacity
    pub fn resize(&mut self, new_capacity: usize) {
        self.capacity = new_capacity;
        
        // Evict items if new capacity is smaller
        while self.cache.len() > self.capacity {
            self.evict_lru();
        }
    }

    /// Get statistics about cache usage
    pub fn stats(&self) -> LruCacheStats {
        LruCacheStats {
            capacity: self.capacity,
            size: self.cache.len(),
            utilization: self.utilization(),
            is_full: self.is_full(),
        }
    }

    /// Iterate over all key-value pairs (no particular order)
    pub fn iter(&self) -> impl Iterator<Item = (&K, &V)> {
        self.cache.iter()
    }

    /// Iterate over key-value pairs in access order (LRU to MRU)
    pub fn iter_by_access_order(&self) -> impl Iterator<Item = (&K, Option<&V>)> {
        self.order.iter().map(move |k| (k, self.cache.get(k)))
    }

    // Private helper methods
    fn update_access_order(&mut self, key: &K) {
        // Remove from current position
        self.order.retain(|k| k != key);
        // Add to back (most recently used)
        self.order.push_back(key.clone());
    }

    fn evict_lru(&mut self) -> Option<(K, V)> {
        if let Some(lru_key) = self.order.pop_front() {
            if let Some(value) = self.cache.remove(&lru_key) {
                Some((lru_key, value))
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl<K, V> Clone for LruCache<K, V>
where
    K: Clone + Eq + Hash,
    V: Clone,
{
    fn clone(&self) -> Self {
        LruCache {
            capacity: self.capacity,
            cache: self.cache.clone(),
            order: self.order.clone(),
        }
    }
}

/// LRU cache statistics
#[derive(Debug, Clone)]
pub struct LruCacheStats {
    pub capacity: usize,
    pub size: usize,
    pub utilization: f64,
    pub is_full: bool,
}

/// LRU cache with hit/miss tracking
#[derive(Debug)]
pub struct LruCacheWithStats<K, V> {
    cache: LruCache<K, V>,
    hits: u64,
    misses: u64,
}

impl<K, V> LruCacheWithStats<K, V>
where
    K: Clone + Eq + Hash,
{
    /// Create new LRU cache with statistics tracking
    pub fn new(capacity: usize) -> Self {
        LruCacheWithStats {
            cache: LruCache::new(capacity),
            hits: 0,
            misses: 0,
        }
    }

    /// Get value by key, tracking hit/miss
    pub fn get(&mut self, key: &K) -> Option<&V> {
        if let Some(value) = self.cache.get(key) {
            self.hits += 1;
            Some(value)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Insert key-value pair
    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        self.cache.insert(key, value)
    }

    /// Remove key-value pair
    pub fn remove(&mut self, key: &K) -> Option<V> {
        self.cache.remove(key)
    }

    /// Get hit rate (0.0 to 1.0)
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }

    /// Get miss rate (0.0 to 1.0)
    pub fn miss_rate(&self) -> f64 {
        1.0 - self.hit_rate()
    }

    /// Get total accesses
    pub fn total_accesses(&self) -> u64 {
        self.hits + self.misses
    }

    /// Get hit count
    pub fn hits(&self) -> u64 {
        self.hits
    }

    /// Get miss count
    pub fn misses(&self) -> u64 {
        self.misses
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.hits = 0;
        self.misses = 0;
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> LruCacheStats {
        self.cache.stats()
    }

    /// Get detailed statistics
    pub fn detailed_stats(&self) -> LruCacheDetailedStats {
        LruCacheDetailedStats {
            cache_stats: self.cache.stats(),
            hits: self.hits,
            misses: self.misses,
            hit_rate: self.hit_rate(),
            total_accesses: self.total_accesses(),
        }
    }

    /// Delegate other methods to inner cache
    pub fn len(&self) -> usize {
        self.cache.len()
    }

    pub fn is_empty(&self) -> bool {
        self.cache.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.cache.capacity()
    }

    pub fn clear(&mut self) {
        self.cache.clear();
        self.reset_stats();
    }

    pub fn contains(&self, key: &K) -> bool {
        self.cache.contains(key)
    }

    pub fn peek(&self, key: &K) -> Option<&V> {
        self.cache.peek(key)
    }
}

/// Detailed LRU cache statistics
#[derive(Debug, Clone)]
pub struct LruCacheDetailedStats {
    pub cache_stats: LruCacheStats,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
    pub total_accesses: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lru_basic_operations() {
        let mut cache = LruCache::new(3);
        
        // Insert items
        cache.insert(1, "one");
        cache.insert(2, "two");
        cache.insert(3, "three");
        
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.get(&1), Some(&"one"));
        assert_eq!(cache.get(&2), Some(&"two"));
        assert_eq!(cache.get(&3), Some(&"three"));
    }

    #[test]
    fn test_lru_eviction() {
        let mut cache = LruCache::new(2);
        
        cache.insert(1, "one");
        cache.insert(2, "two");
        
        // This should evict key 1
        cache.insert(3, "three");
        
        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.get(&2), Some(&"two"));
        assert_eq!(cache.get(&3), Some(&"three"));
    }

    #[test]
    fn test_lru_access_order() {
        let mut cache = LruCache::new(3);
        
        cache.insert(1, "one");
        cache.insert(2, "two");
        cache.insert(3, "three");
        
        // Access key 1 to make it MRU
        cache.get(&1);
        
        // Insert new item, should evict key 2 (now LRU)
        cache.insert(4, "four");
        
        assert_eq!(cache.get(&1), Some(&"one"));
        assert_eq!(cache.get(&2), None);
        assert_eq!(cache.get(&3), Some(&"three"));
        assert_eq!(cache.get(&4), Some(&"four"));
    }

    #[test]
    fn test_lru_with_stats() {
        let mut cache = LruCacheWithStats::new(2);
        
        cache.insert(1, "one");
        cache.insert(2, "two");
        
        // Hit
        cache.get(&1);
        // Miss
        cache.get(&3);
        
        assert_eq!(cache.hits(), 1);
        assert_eq!(cache.misses(), 1);
        assert_eq!(cache.hit_rate(), 0.5);
    }

    #[test]
    fn test_lru_resize() {
        let mut cache = LruCache::new(3);
        
        cache.insert(1, "one");
        cache.insert(2, "two");
        cache.insert(3, "three");
        
        // Resize to smaller capacity
        cache.resize(2);
        
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.capacity(), 2);
        
        // Should have evicted LRU item (key 1)
        assert_eq!(cache.get(&1), None);
        assert_eq!(cache.get(&2), Some(&"two"));
        assert_eq!(cache.get(&3), Some(&"three"));
    }
}
