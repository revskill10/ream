use crate::page_cache::{PageCacheTransition, PageData};
use crate::types::PageId;
use std::collections::HashMap;
use std::sync::Arc;

/// Coalgebraic page cache implementation
/// 
/// This implements the page cache as a coalgebra where:
/// - State: Current cache contents and metadata
/// - Observations: Cache operations (load, store, flush, evict)
/// - Transitions: State evolution based on operations
#[derive(Debug)]
pub struct PageCacheCoalgebra {
    cache: HashMap<PageId, Arc<PageData>>,
    capacity: usize,
    access_order: Vec<PageId>, // For LRU eviction
}

impl PageCacheCoalgebra {
    /// Create new page cache coalgebra with given capacity
    pub fn new(capacity: usize) -> Self {
        PageCacheCoalgebra {
            cache: HashMap::new(),
            capacity,
            access_order: Vec::new(),
        }
    }

    /// Process input and evolve state (coalgebraic observation)
    /// 
    /// This is the core coalgebraic operation that takes an input transition
    /// and produces an output along with the next state.
    pub fn process_input(&mut self, input: PageCacheTransition) -> (Option<Arc<PageData>>, ()) {
        match input {
            PageCacheTransition::Load(page_id) => {
                self.handle_load(page_id)
            }
            PageCacheTransition::Store(page_id, page_data) => {
                self.handle_store(page_id, page_data)
            }
            PageCacheTransition::Flush(page_id) => {
                self.handle_flush(page_id)
            }
            PageCacheTransition::Evict(page_id) => {
                self.handle_evict(page_id)
            }
            PageCacheTransition::FlushAll => {
                self.handle_flush_all()
            }
            PageCacheTransition::Clear => {
                self.handle_clear()
            }
        }
    }

    /// Get current cache state
    pub fn get_state(&self) -> PageCacheState {
        PageCacheState {
            cached_pages: self.cache.keys().cloned().collect(),
            cache_size: self.cache.len(),
            capacity: self.capacity,
            dirty_pages: self.count_dirty_pages(),
        }
    }

    /// Check if page is in cache
    pub fn contains(&self, page_id: &PageId) -> bool {
        self.cache.contains_key(page_id)
    }

    /// Get cache utilization (0.0 to 1.0)
    pub fn utilization(&self) -> f64 {
        self.cache.len() as f64 / self.capacity as f64
    }

    /// Get number of dirty pages
    pub fn count_dirty_pages(&self) -> usize {
        self.cache
            .values()
            .filter(|page| page.is_dirty)
            .count()
    }

    /// Get all dirty page IDs
    pub fn get_dirty_pages(&self) -> Vec<PageId> {
        self.cache
            .iter()
            .filter(|(_, page)| page.is_dirty)
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get least recently used page ID
    pub fn get_lru_page(&self) -> Option<PageId> {
        self.access_order.first().cloned()
    }

    /// Get most recently used page ID
    pub fn get_mru_page(&self) -> Option<PageId> {
        self.access_order.last().cloned()
    }

    // Private coalgebraic transition handlers
    fn handle_load(&mut self, page_id: PageId) -> (Option<Arc<PageData>>, ()) {
        if let Some(page_data) = self.cache.get(&page_id).cloned() {
            // Update access order (move to end)
            self.update_access_order(page_id);
            (Some(page_data), ())
        } else {
            (None, ())
        }
    }

    fn handle_store(&mut self, page_id: PageId, page_data: PageData) -> (Option<Arc<PageData>>, ()) {
        // Check if we need to evict pages to make room
        while self.cache.len() >= self.capacity {
            if let Some(lru_page) = self.get_lru_page() {
                self.evict_page(lru_page);
            } else {
                break;
            }
        }

        // Store the page
        let arc_data = Arc::new(page_data);
        self.cache.insert(page_id, Arc::clone(&arc_data));
        self.update_access_order(page_id);

        (Some(arc_data), ())
    }

    fn handle_flush(&mut self, page_id: PageId) -> (Option<Arc<PageData>>, ()) {
        if let Some(page_data) = self.cache.get(&page_id).cloned() {
            // Mark as clean (in a real implementation, this would write to disk)
            let mut clean_page = (*page_data).clone();
            clean_page.is_dirty = false;
            let clean_arc = Arc::new(clean_page);
            self.cache.insert(page_id, clean_arc.clone());
            (Some(clean_arc), ())
        } else {
            (None, ())
        }
    }

    fn handle_evict(&mut self, page_id: PageId) -> (Option<Arc<PageData>>, ()) {
        let evicted = self.cache.remove(&page_id);
        self.remove_from_access_order(page_id);
        (evicted, ())
    }

    fn handle_flush_all(&mut self) -> (Option<Arc<PageData>>, ()) {
        // Mark all pages as clean
        let page_ids: Vec<PageId> = self.cache.keys().cloned().collect();
        for page_id in page_ids {
            if let Some(page_data) = self.cache.get(&page_id).cloned() {
                let mut clean_page = (*page_data).clone();
                clean_page.is_dirty = false;
                self.cache.insert(page_id, Arc::new(clean_page));
            }
        }
        (None, ())
    }

    fn handle_clear(&mut self) -> (Option<Arc<PageData>>, ()) {
        self.cache.clear();
        self.access_order.clear();
        (None, ())
    }

    fn evict_page(&mut self, page_id: PageId) {
        self.cache.remove(&page_id);
        self.remove_from_access_order(page_id);
    }

    fn update_access_order(&mut self, page_id: PageId) {
        // Remove from current position
        self.access_order.retain(|&id| id != page_id);
        // Add to end (most recently used)
        self.access_order.push(page_id);
    }

    fn remove_from_access_order(&mut self, page_id: PageId) {
        self.access_order.retain(|&id| id != page_id);
    }
}

impl Clone for PageCacheCoalgebra {
    fn clone(&self) -> Self {
        PageCacheCoalgebra {
            cache: self.cache.clone(),
            capacity: self.capacity,
            access_order: self.access_order.clone(),
        }
    }
}

/// Page cache state for coalgebraic observations
#[derive(Debug, Clone)]
pub struct PageCacheState {
    pub cached_pages: Vec<PageId>,
    pub cache_size: usize,
    pub capacity: usize,
    pub dirty_pages: usize,
}

impl PageCacheState {
    /// Check if cache is full
    pub fn is_full(&self) -> bool {
        self.cache_size >= self.capacity
    }

    /// Check if cache is empty
    pub fn is_empty(&self) -> bool {
        self.cache_size == 0
    }

    /// Get cache utilization ratio
    pub fn utilization(&self) -> f64 {
        if self.capacity == 0 {
            0.0
        } else {
            self.cache_size as f64 / self.capacity as f64
        }
    }

    /// Get dirty page ratio
    pub fn dirty_ratio(&self) -> f64 {
        if self.cache_size == 0 {
            0.0
        } else {
            self.dirty_pages as f64 / self.cache_size as f64
        }
    }
}

/// Coalgebraic cache policy trait
pub trait CachePolicy {
    /// Determine which page to evict
    fn select_eviction_candidate(&self, state: &PageCacheState) -> Option<PageId>;
    
    /// Determine if a page should be prefetched
    fn should_prefetch(&self, page_id: PageId, state: &PageCacheState) -> bool;
    
    /// Determine if cache maintenance is needed
    fn needs_maintenance(&self, state: &PageCacheState) -> bool;
}

/// LRU (Least Recently Used) cache policy
pub struct LruPolicy;

impl CachePolicy for LruPolicy {
    fn select_eviction_candidate(&self, state: &PageCacheState) -> Option<PageId> {
        // In a real implementation, this would use access order information
        state.cached_pages.first().cloned()
    }
    
    fn should_prefetch(&self, _page_id: PageId, state: &PageCacheState) -> bool {
        // Prefetch if cache is not full
        !state.is_full()
    }
    
    fn needs_maintenance(&self, state: &PageCacheState) -> bool {
        // Maintenance needed if too many dirty pages
        state.dirty_ratio() > 0.5
    }
}

/// FIFO (First In, First Out) cache policy
pub struct FifoPolicy;

impl CachePolicy for FifoPolicy {
    fn select_eviction_candidate(&self, state: &PageCacheState) -> Option<PageId> {
        // Evict first page (oldest)
        state.cached_pages.first().cloned()
    }
    
    fn should_prefetch(&self, _page_id: PageId, state: &PageCacheState) -> bool {
        state.utilization() < 0.8
    }
    
    fn needs_maintenance(&self, state: &PageCacheState) -> bool {
        state.dirty_pages > 10
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page_cache::PageType;

    #[test]
    fn test_coalgebra_load_miss() {
        let mut coalgebra = PageCacheCoalgebra::new(10);
        let page_id = PageId(1);
        
        let (result, _) = coalgebra.process_input(PageCacheTransition::Load(page_id));
        assert!(result.is_none());
    }

    #[test]
    fn test_coalgebra_store_and_load() {
        let mut coalgebra = PageCacheCoalgebra::new(10);
        let page_id = PageId(1);
        let page_data = PageData::new(vec![1, 2, 3, 4], PageType::Data);
        
        // Store page
        let (result, _) = coalgebra.process_input(PageCacheTransition::Store(page_id, page_data.clone()));
        assert!(result.is_some());
        
        // Load page
        let (result, _) = coalgebra.process_input(PageCacheTransition::Load(page_id));
        assert!(result.is_some());
        assert_eq!(result.unwrap().data, page_data.data);
    }

    #[test]
    fn test_coalgebra_eviction() {
        let mut coalgebra = PageCacheCoalgebra::new(2);
        
        // Fill cache
        let page1 = PageData::new(vec![1], PageType::Data);
        let page2 = PageData::new(vec![2], PageType::Data);
        let page3 = PageData::new(vec![3], PageType::Data);
        
        coalgebra.process_input(PageCacheTransition::Store(PageId(1), page1));
        coalgebra.process_input(PageCacheTransition::Store(PageId(2), page2));
        
        // This should evict page 1
        coalgebra.process_input(PageCacheTransition::Store(PageId(3), page3));
        
        // Page 1 should be evicted
        let (result, _) = coalgebra.process_input(PageCacheTransition::Load(PageId(1)));
        assert!(result.is_none());
        
        // Page 3 should be present
        let (result, _) = coalgebra.process_input(PageCacheTransition::Load(PageId(3)));
        assert!(result.is_some());
    }

    #[test]
    fn test_coalgebra_state_observation() {
        let mut coalgebra = PageCacheCoalgebra::new(10);
        let page_data = PageData::new(vec![1, 2, 3, 4], PageType::Data);
        
        coalgebra.process_input(PageCacheTransition::Store(PageId(1), page_data));
        
        let state = coalgebra.get_state();
        assert_eq!(state.cache_size, 1);
        assert_eq!(state.capacity, 10);
        assert!(!state.is_full());
        assert!(!state.is_empty());
    }
}
