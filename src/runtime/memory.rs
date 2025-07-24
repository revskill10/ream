//! Memory management with region algebra and garbage collection

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;
use bumpalo::Bump;
use crate::types::Pid;
use crate::error::{RuntimeError, RuntimeResult};

/// Memory region identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RegionId(usize);

impl RegionId {
    fn new() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(1);
        RegionId(COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

/// Memory region with bump allocation
pub struct MemoryRegion {
    id: RegionId,
    bump: Bump,
    allocated: AtomicUsize,
    owner: Option<Pid>,
    created_at: Instant,
}

impl MemoryRegion {
    fn new(owner: Option<Pid>) -> Self {
        MemoryRegion {
            id: RegionId::new(),
            bump: Bump::new(),
            allocated: AtomicUsize::new(0),
            owner,
            created_at: Instant::now(),
        }
    }
    
    /// Allocate memory in this region
    pub fn alloc<T>(&self, value: T) -> &T {
        let allocated = self.bump.alloc(value);
        self.allocated.fetch_add(std::mem::size_of::<T>(), Ordering::SeqCst);
        allocated
    }
    
    /// Allocate a slice in this region
    pub fn alloc_slice<T: Clone>(&self, slice: &[T]) -> &[T] {
        let allocated = self.bump.alloc_slice_clone(slice);
        self.allocated.fetch_add(
            std::mem::size_of::<T>() * slice.len(), 
            Ordering::SeqCst
        );
        allocated
    }
    
    /// Get total allocated bytes
    pub fn allocated_bytes(&self) -> usize {
        self.allocated.load(Ordering::SeqCst)
    }
    
    /// Get region ID
    pub fn id(&self) -> RegionId {
        self.id
    }
    
    /// Get owner process
    pub fn owner(&self) -> Option<Pid> {
        self.owner
    }
    
    /// Reset the region (deallocate all)
    pub fn reset(&mut self) {
        self.bump.reset();
        self.allocated.store(0, Ordering::SeqCst);
    }
}

/// Garbage collection statistics
#[derive(Debug, Default, Clone)]
pub struct GcStats {
    pub collections: u64,
    pub total_time: std::time::Duration,
    pub bytes_collected: usize,
    pub regions_collected: usize,
}

/// Memory manager with generational garbage collection
pub struct MemoryManager {
    /// Young generation regions (frequently allocated)
    young_regions: HashMap<RegionId, MemoryRegion>,
    
    /// Old generation regions (long-lived)
    old_regions: HashMap<RegionId, MemoryRegion>,
    
    /// Process-owned regions
    process_regions: HashMap<Pid, Vec<RegionId>>,
    
    /// Global shared region
    global_region: MemoryRegion,
    
    /// GC statistics
    gc_stats: GcStats,
    
    /// Total allocated bytes
    total_allocated: AtomicUsize,
    
    /// GC threshold
    gc_threshold: usize,
    
    /// Generation promotion threshold
    promotion_threshold: std::time::Duration,
}

impl MemoryManager {
    /// Create a new memory manager
    pub fn new() -> Self {
        MemoryManager {
            young_regions: HashMap::new(),
            old_regions: HashMap::new(),
            process_regions: HashMap::new(),
            global_region: MemoryRegion::new(None),
            gc_stats: GcStats::default(),
            total_allocated: AtomicUsize::new(0),
            gc_threshold: 64 * 1024 * 1024, // 64MB
            promotion_threshold: std::time::Duration::from_secs(60), // 1 minute
        }
    }
    
    /// Allocate a new region for a process
    pub fn allocate_region(&mut self, owner: Pid) -> RegionId {
        let region = MemoryRegion::new(Some(owner));
        let id = region.id();
        
        self.young_regions.insert(id, region);
        self.process_regions.entry(owner).or_insert_with(Vec::new).push(id);
        
        id
    }
    
    /// Get a region by ID
    pub fn get_region(&self, id: RegionId) -> Option<&MemoryRegion> {
        self.young_regions.get(&id)
            .or_else(|| self.old_regions.get(&id))
    }
    
    /// Get a mutable region by ID
    pub fn get_region_mut(&mut self, id: RegionId) -> Option<&mut MemoryRegion> {
        if self.young_regions.contains_key(&id) {
            self.young_regions.get_mut(&id)
        } else {
            self.old_regions.get_mut(&id)
        }
    }
    
    /// Get the global region
    pub fn global_region(&self) -> &MemoryRegion {
        &self.global_region
    }
    
    /// Get mutable global region
    pub fn global_region_mut(&mut self) -> &mut MemoryRegion {
        &mut self.global_region
    }
    
    /// Get regions owned by a process
    pub fn process_regions(&self, pid: Pid) -> Vec<RegionId> {
        self.process_regions.get(&pid).cloned().unwrap_or_default()
    }
    
    /// Deallocate all regions owned by a process
    pub fn deallocate_process_regions(&mut self, pid: Pid) -> RuntimeResult<()> {
        if let Some(region_ids) = self.process_regions.remove(&pid) {
            for id in region_ids {
                self.young_regions.remove(&id);
                self.old_regions.remove(&id);
            }
        }
        Ok(())
    }
    
    /// Run garbage collection
    pub fn collect(&mut self) -> GcStats {
        let start = Instant::now();
        let mut bytes_collected = 0;
        let mut regions_collected = 0;
        
        // Promote old young regions to old generation
        let mut to_promote = Vec::new();
        for (id, region) in &self.young_regions {
            if region.created_at.elapsed() > self.promotion_threshold {
                to_promote.push(*id);
            }
        }
        
        for id in to_promote {
            if let Some(region) = self.young_regions.remove(&id) {
                self.old_regions.insert(id, region);
            }
        }
        
        // Collect empty regions
        let mut to_remove = Vec::new();
        for (id, region) in &self.young_regions {
            if region.allocated_bytes() == 0 {
                to_remove.push(*id);
            }
        }
        
        for id in to_remove {
            if let Some(region) = self.young_regions.remove(&id) {
                bytes_collected += region.allocated_bytes();
                regions_collected += 1;
                
                // Remove from process regions
                if let Some(owner) = region.owner() {
                    if let Some(regions) = self.process_regions.get_mut(&owner) {
                        regions.retain(|&r| r != id);
                    }
                }
            }
        }
        
        let collection_time = start.elapsed();
        
        // Update statistics
        self.gc_stats.collections += 1;
        self.gc_stats.total_time += collection_time;
        self.gc_stats.bytes_collected += bytes_collected;
        self.gc_stats.regions_collected += regions_collected;
        
        self.gc_stats.clone()
    }
    
    /// Get total allocated bytes across all regions
    pub fn total_allocated(&self) -> usize {
        let mut total = self.global_region.allocated_bytes();
        
        for region in self.young_regions.values() {
            total += region.allocated_bytes();
        }
        
        for region in self.old_regions.values() {
            total += region.allocated_bytes();
        }
        
        total
    }
    
    /// Check if GC should run
    pub fn should_collect(&self) -> bool {
        self.total_allocated() > self.gc_threshold
    }
    
    /// Get GC statistics
    pub fn gc_stats(&self) -> &GcStats {
        &self.gc_stats
    }
    
    /// Set GC threshold
    pub fn set_gc_threshold(&mut self, threshold: usize) {
        self.gc_threshold = threshold;
    }
    
    /// Get memory usage by generation
    pub fn memory_usage(&self) -> (usize, usize, usize) {
        let young: usize = self.young_regions.values()
            .map(|r| r.allocated_bytes())
            .sum();
        
        let old: usize = self.old_regions.values()
            .map(|r| r.allocated_bytes())
            .sum();
        
        let global = self.global_region.allocated_bytes();
        
        (young, old, global)
    }
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Garbage collector interface
pub struct GarbageCollector {
    memory_manager: MemoryManager,
}

impl GarbageCollector {
    pub fn new() -> Self {
        GarbageCollector {
            memory_manager: MemoryManager::new(),
        }
    }
    
    /// Allocate memory for a value
    pub fn allocate<T>(&mut self, value: T, owner: Option<Pid>) -> RuntimeResult<&T> {
        let region_id = if let Some(pid) = owner {
            // Find or create a region for this process
            let regions = self.memory_manager.process_regions(pid);
            if let Some(&id) = regions.last() {
                id
            } else {
                self.memory_manager.allocate_region(pid)
            }
        } else {
            // Use global region
            self.memory_manager.global_region().id()
        };
        
        if let Some(region) = self.memory_manager.get_region(region_id) {
            Ok(region.alloc(value))
        } else {
            Err(RuntimeError::Memory("Region not found".to_string()))
        }
    }
    
    /// Run garbage collection
    pub fn collect(&mut self) -> GcStats {
        self.memory_manager.collect()
    }
    
    /// Get total allocated memory
    pub fn total_allocated(&self) -> usize {
        self.memory_manager.total_allocated()
    }
    
    /// Clean up process memory
    pub fn cleanup_process(&mut self, pid: Pid) -> RuntimeResult<()> {
        self.memory_manager.deallocate_process_regions(pid)
    }
}

impl Default for GarbageCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_region() {
        let region = MemoryRegion::new(None);
        
        let value = region.alloc(42i32);
        assert_eq!(*value, 42);
        assert!(region.allocated_bytes() >= std::mem::size_of::<i32>());
    }
    
    #[test]
    fn test_memory_manager() {
        let mut manager = MemoryManager::new();
        let pid = Pid::new();
        
        let region_id = manager.allocate_region(pid);
        assert!(manager.get_region(region_id).is_some());
        
        let regions = manager.process_regions(pid);
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0], region_id);
    }
    
    #[test]
    fn test_garbage_collection() {
        let mut manager = MemoryManager::new();
        let pid = Pid::new();
        
        // Allocate some regions
        for _ in 0..5 {
            manager.allocate_region(pid);
        }
        
        let initial_count = manager.young_regions.len();
        let stats = manager.collect();
        
        assert!(stats.collections > 0);
        // Some regions might be promoted or collected
        assert!(manager.young_regions.len() <= initial_count);
    }
}
