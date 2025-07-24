use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Page types in the database
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageType {
    /// Data page containing table rows
    Data,
    /// Index page containing B-Tree nodes
    Index,
    /// Overflow page for large data
    Overflow,
    /// Free list page tracking available pages
    FreeList,
    /// Lock page for concurrency control
    Lock,
    /// Metadata page containing schema information
    Metadata,
    /// WAL (Write-Ahead Log) page
    Wal,
}

impl PageType {
    /// Get the type identifier as a byte
    pub fn as_byte(&self) -> u8 {
        match self {
            PageType::Data => 0x01,
            PageType::Index => 0x02,
            PageType::Overflow => 0x03,
            PageType::FreeList => 0x04,
            PageType::Lock => 0x05,
            PageType::Metadata => 0x06,
            PageType::Wal => 0x07,
        }
    }

    /// Create from byte identifier
    pub fn from_byte(byte: u8) -> Option<Self> {
        match byte {
            0x01 => Some(PageType::Data),
            0x02 => Some(PageType::Index),
            0x03 => Some(PageType::Overflow),
            0x04 => Some(PageType::FreeList),
            0x05 => Some(PageType::Lock),
            0x06 => Some(PageType::Metadata),
            0x07 => Some(PageType::Wal),
            _ => None,
        }
    }

    /// Check if page type is cacheable
    pub fn is_cacheable(&self) -> bool {
        match self {
            PageType::Data | PageType::Index | PageType::Metadata => true,
            PageType::Overflow | PageType::FreeList | PageType::Lock | PageType::Wal => false,
        }
    }

    /// Check if page type supports compression
    pub fn supports_compression(&self) -> bool {
        match self {
            PageType::Data | PageType::Index => true,
            _ => false,
        }
    }
}

/// Page data structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageData {
    /// Raw page data
    pub data: Vec<u8>,
    /// Whether the page has been modified
    pub is_dirty: bool,
    /// Type of page
    pub page_type: PageType,
    /// Checksum for integrity verification
    pub checksum: u32,
}

impl PageData {
    /// Create new page data
    pub fn new(data: Vec<u8>, page_type: PageType) -> Self {
        let checksum = Self::calculate_checksum(&data);
        PageData {
            data,
            is_dirty: false,
            page_type,
            checksum,
        }
    }

    /// Create new dirty page data
    pub fn new_dirty(data: Vec<u8>, page_type: PageType) -> Self {
        let checksum = Self::calculate_checksum(&data);
        PageData {
            data,
            is_dirty: true,
            page_type,
            checksum,
        }
    }

    /// Create empty page of given size
    pub fn empty(size: usize, page_type: PageType) -> Self {
        Self::new(vec![0; size], page_type)
    }

    /// Get page size
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Mark page as dirty
    pub fn mark_dirty(&mut self) {
        self.is_dirty = true;
        self.update_checksum();
    }

    /// Mark page as clean
    pub fn mark_clean(&mut self) {
        self.is_dirty = false;
    }

    /// Update checksum
    pub fn update_checksum(&mut self) {
        self.checksum = Self::calculate_checksum(&self.data);
    }

    /// Verify checksum integrity
    pub fn verify_checksum(&self) -> bool {
        self.checksum == Self::calculate_checksum(&self.data)
    }

    /// Calculate checksum for data
    fn calculate_checksum(data: &[u8]) -> u32 {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        
        // Use first 4 bytes as checksum
        u32::from_be_bytes([result[0], result[1], result[2], result[3]])
    }

    /// Write data at offset
    pub fn write_at(&mut self, offset: usize, data: &[u8]) -> Result<(), PageError> {
        if offset + data.len() > self.data.len() {
            return Err(PageError::OutOfBounds {
                offset,
                length: data.len(),
                page_size: self.data.len(),
            });
        }

        self.data[offset..offset + data.len()].copy_from_slice(data);
        self.mark_dirty();
        Ok(())
    }

    /// Read data from offset
    pub fn read_at(&self, offset: usize, length: usize) -> Result<&[u8], PageError> {
        if offset + length > self.data.len() {
            return Err(PageError::OutOfBounds {
                offset,
                length,
                page_size: self.data.len(),
            });
        }

        Ok(&self.data[offset..offset + length])
    }

    /// Write u32 at offset (little endian)
    pub fn write_u32_at(&mut self, offset: usize, value: u32) -> Result<(), PageError> {
        self.write_at(offset, &value.to_le_bytes())
    }

    /// Read u32 from offset (little endian)
    pub fn read_u32_at(&self, offset: usize) -> Result<u32, PageError> {
        let bytes = self.read_at(offset, 4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    /// Write u64 at offset (little endian)
    pub fn write_u64_at(&mut self, offset: usize, value: u64) -> Result<(), PageError> {
        self.write_at(offset, &value.to_le_bytes())
    }

    /// Read u64 from offset (little endian)
    pub fn read_u64_at(&self, offset: usize) -> Result<u64, PageError> {
        let bytes = self.read_at(offset, 8)?;
        Ok(u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3],
            bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    /// Compress page data (if supported)
    pub fn compress(&mut self) -> Result<(), PageError> {
        if !self.page_type.supports_compression() {
            return Err(PageError::CompressionNotSupported(self.page_type));
        }

        // Simple compression simulation (in real implementation, use actual compression)
        // For now, just mark as compressed in metadata
        Ok(())
    }

    /// Decompress page data
    pub fn decompress(&mut self) -> Result<(), PageError> {
        // Simple decompression simulation
        Ok(())
    }

    /// Get free space in page
    pub fn free_space(&self) -> usize {
        // This would be calculated based on page structure
        // For now, return a simple estimate
        self.data.len().saturating_sub(self.used_space())
    }

    /// Get used space in page
    pub fn used_space(&self) -> usize {
        // This would be calculated based on page structure
        // For now, return a simple estimate based on non-zero bytes
        self.data.iter().filter(|&&b| b != 0).count()
    }

    /// Check if page has enough free space for data
    pub fn has_space_for(&self, size: usize) -> bool {
        self.free_space() >= size
    }

    /// Defragment page (compact free space)
    pub fn defragment(&mut self) -> Result<(), PageError> {
        // This would implement page defragmentation
        // For now, just mark as dirty
        self.mark_dirty();
        Ok(())
    }
}

impl PartialEq for PageData {
    fn eq(&self, other: &Self) -> bool {
        self.data == other.data && self.page_type == other.page_type
    }
}

/// Page-related errors
#[derive(Debug, Clone, thiserror::Error)]
pub enum PageError {
    #[error("Out of bounds access: offset {offset}, length {length}, page size {page_size}")]
    OutOfBounds {
        offset: usize,
        length: usize,
        page_size: usize,
    },
    
    #[error("Checksum mismatch")]
    ChecksumMismatch,
    
    #[error("Compression not supported for page type: {0:?}")]
    CompressionNotSupported(PageType),
    
    #[error("Page is corrupted")]
    Corrupted,
    
    #[error("Insufficient space: need {needed}, available {available}")]
    InsufficientSpace { needed: usize, available: usize },
}

/// High-level page interface
#[derive(Debug, Clone)]
pub struct Page {
    pub data: PageData,
    pub id: crate::types::PageId,
}

impl Page {
    /// Create new page
    pub fn new(id: crate::types::PageId, data: PageData) -> Self {
        Page { id, data }
    }

    /// Create empty page
    pub fn empty(id: crate::types::PageId, size: usize, page_type: PageType) -> Self {
        Page {
            id,
            data: PageData::empty(size, page_type),
        }
    }

    /// Get page ID
    pub fn id(&self) -> crate::types::PageId {
        self.id
    }

    /// Get page type
    pub fn page_type(&self) -> PageType {
        self.data.page_type
    }

    /// Check if page is dirty
    pub fn is_dirty(&self) -> bool {
        self.data.is_dirty
    }

    /// Get page size
    pub fn size(&self) -> usize {
        self.data.size()
    }

    /// Verify page integrity
    pub fn verify(&self) -> Result<(), PageError> {
        if !self.data.verify_checksum() {
            return Err(PageError::ChecksumMismatch);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_data_creation() {
        let data = vec![1, 2, 3, 4];
        let page = PageData::new(data.clone(), PageType::Data);
        
        assert_eq!(page.data, data);
        assert_eq!(page.page_type, PageType::Data);
        assert!(!page.is_dirty);
        assert!(page.verify_checksum());
    }

    #[test]
    fn test_page_data_write_read() {
        let mut page = PageData::empty(100, PageType::Data);
        
        // Write data
        page.write_at(10, &[1, 2, 3, 4]).unwrap();
        assert!(page.is_dirty);
        
        // Read data
        let read_data = page.read_at(10, 4).unwrap();
        assert_eq!(read_data, &[1, 2, 3, 4]);
    }

    #[test]
    fn test_page_data_u32_operations() {
        let mut page = PageData::empty(100, PageType::Data);
        
        // Write u32
        page.write_u32_at(0, 0x12345678).unwrap();
        
        // Read u32
        let value = page.read_u32_at(0).unwrap();
        assert_eq!(value, 0x12345678);
    }

    #[test]
    fn test_page_type_conversion() {
        assert_eq!(PageType::Data.as_byte(), 0x01);
        assert_eq!(PageType::from_byte(0x01), Some(PageType::Data));
        assert_eq!(PageType::from_byte(0xFF), None);
    }

    #[test]
    fn test_page_bounds_checking() {
        let page = PageData::empty(10, PageType::Data);
        
        // Should fail - out of bounds
        let result = page.read_at(8, 5);
        assert!(result.is_err());
        
        // Should succeed
        let result = page.read_at(8, 2);
        assert!(result.is_ok());
    }
}
