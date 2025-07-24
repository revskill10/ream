use crate::btree::node::{BTreeNode, BTreeNodeType};
use crate::types::{Row, RowId, Value};
use std::sync::Arc;

/// B-Tree cursor for iteration following coalgebraic structure
#[derive(Debug)]
pub struct BTreeCursor<'a> {
    root: &'a BTreeNode,
    stack: Vec<CursorFrame<'a>>,
    current_position: Option<CursorPosition>,
}

/// Cursor frame for tracking position in tree traversal
#[derive(Debug, Clone)]
struct CursorFrame<'a> {
    node: &'a BTreeNode,
    key_index: usize,
    child_index: usize,
}

/// Current cursor position
#[derive(Debug, Clone)]
struct CursorPosition {
    key_index: usize,
    at_end: bool,
}

impl<'a> BTreeCursor<'a> {
    /// Create a new cursor positioned at the beginning
    pub fn new(root: &'a BTreeNode) -> Self {
        let mut cursor = BTreeCursor {
            root,
            stack: Vec::new(),
            current_position: None,
        };
        cursor.move_to_first();
        cursor
    }

    /// Move cursor to the first element
    pub fn move_to_first(&mut self) {
        self.stack.clear();
        self.current_position = None;
        self.find_leftmost_leaf();
    }

    /// Move cursor to the last element
    pub fn move_to_last(&mut self) {
        self.stack.clear();
        self.current_position = None;
        self.find_rightmost_leaf();
    }

    /// Move cursor to a specific key
    pub fn seek(&mut self, key: &Value) {
        self.stack.clear();
        self.current_position = None;
        self.seek_key(self.root, key);
    }

    /// Get current key-value pair (coalgebraic observation)
    pub fn current(&self) -> Option<(&Value, &RowId, &Row)> {
        if let Some(position) = &self.current_position {
            if position.at_end {
                return None;
            }

            if let Some(frame) = self.stack.last() {
                if let BTreeNodeType::Leaf { keys, values } = &frame.node.node_type {
                    if position.key_index < keys.len() {
                        let key = &keys[position.key_index];
                        let (row_id, row) = &values[position.key_index];
                        return Some((key, row_id, row));
                    }
                }
            }
        }
        None
    }

    /// Move to next element (coalgebraic advancement)
    pub fn next(&mut self) -> Option<(&Value, &RowId, &Row)> {
        if let Some(position) = &mut self.current_position {
            if position.at_end {
                return None;
            }

            // Try to advance within current leaf
            if let Some(frame) = self.stack.last() {
                if let BTreeNodeType::Leaf { keys, .. } = &frame.node.node_type {
                    if position.key_index + 1 < keys.len() {
                        position.key_index += 1;
                        return self.current();
                    }
                }
            }

            // Need to move to next leaf
            self.advance_to_next_leaf();
        }

        self.current()
    }

    /// Move to previous element
    pub fn prev(&mut self) -> Option<(&Value, &RowId, &Row)> {
        if let Some(position) = &mut self.current_position {
            if position.at_end {
                // Move to last valid position
                self.move_to_last();
                return self.current();
            }

            // Try to move back within current leaf
            if position.key_index > 0 {
                position.key_index -= 1;
                return self.current();
            }

            // Need to move to previous leaf
            self.advance_to_prev_leaf();
        }

        self.current()
    }

    /// Check if cursor is at end
    pub fn is_at_end(&self) -> bool {
        self.current_position
            .as_ref()
            .map(|pos| pos.at_end)
            .unwrap_or(true)
    }

    /// Check if cursor is valid
    pub fn is_valid(&self) -> bool {
        !self.is_at_end() && self.current().is_some()
    }

    /// Collect all remaining elements from cursor position
    pub fn collect_remaining(&mut self) -> Vec<(Value, RowId, Row)> {
        let mut results = Vec::new();
        while let Some((key, row_id, row)) = self.current() {
            results.push((key.clone(), *row_id, row.clone()));
            self.next();
        }
        results
    }

    /// Count remaining elements without consuming them
    pub fn count_remaining(&self) -> usize {
        let mut count = 0;
        let mut temp_cursor = BTreeCursor::new(self.root);
        
        // Position temp cursor at same position
        if let Some(current) = self.current() {
            temp_cursor.seek(current.0);
        }
        
        while temp_cursor.current().is_some() {
            count += 1;
            temp_cursor.next();
        }
        
        count
    }

    // Private helper methods
    fn find_leftmost_leaf(&mut self) {
        let mut current = self.root;
        
        loop {
            match &current.node_type {
                BTreeNodeType::Leaf { keys, .. } => {
                    self.stack.push(CursorFrame {
                        node: current,
                        key_index: 0,
                        child_index: 0,
                    });
                    
                    self.current_position = Some(CursorPosition {
                        key_index: 0,
                        at_end: keys.is_empty(),
                    });
                    break;
                }
                BTreeNodeType::Internal { children, .. } => {
                    self.stack.push(CursorFrame {
                        node: current,
                        key_index: 0,
                        child_index: 0,
                    });
                    
                    current = &children[0];
                }
            }
        }
    }

    fn find_rightmost_leaf(&mut self) {
        let mut current = self.root;
        
        loop {
            match &current.node_type {
                BTreeNodeType::Leaf { keys, .. } => {
                    let last_index = if keys.is_empty() { 0 } else { keys.len() - 1 };
                    
                    self.stack.push(CursorFrame {
                        node: current,
                        key_index: last_index,
                        child_index: 0,
                    });
                    
                    self.current_position = Some(CursorPosition {
                        key_index: last_index,
                        at_end: keys.is_empty(),
                    });
                    break;
                }
                BTreeNodeType::Internal { children, .. } => {
                    let last_child_index = children.len() - 1;
                    
                    self.stack.push(CursorFrame {
                        node: current,
                        key_index: 0,
                        child_index: last_child_index,
                    });
                    
                    current = &children[last_child_index];
                }
            }
        }
    }

    fn seek_key(&mut self, node: &'a BTreeNode, key: &Value) {
        match &node.node_type {
            BTreeNodeType::Leaf { keys, .. } => {
                let pos = keys.partition_point(|k| k < key);
                
                self.stack.push(CursorFrame {
                    node,
                    key_index: pos,
                    child_index: 0,
                });
                
                self.current_position = Some(CursorPosition {
                    key_index: pos,
                    at_end: pos >= keys.len(),
                });
            }
            BTreeNodeType::Internal { keys, children } => {
                let pos = keys.partition_point(|k| k < key);
                
                self.stack.push(CursorFrame {
                    node,
                    key_index: pos,
                    child_index: pos,
                });
                
                self.seek_key(&children[pos], key);
            }
        }
    }

    fn advance_to_next_leaf(&mut self) {
        // Pop current leaf
        self.stack.pop();
        
        // Find next leaf by going up the stack
        while let Some(frame) = self.stack.last_mut() {
            if let BTreeNodeType::Internal { children, .. } = &frame.node.node_type {
                if frame.child_index + 1 < children.len() {
                    // Move to next child
                    frame.child_index += 1;
                    let next_child = &children[frame.child_index];
                    
                    // Go down to leftmost leaf of this child
                    self.descend_to_leftmost_leaf(next_child);
                    return;
                }
            }
            
            // No more children at this level, go up
            self.stack.pop();
        }
        
        // Reached end of tree
        self.current_position = Some(CursorPosition {
            key_index: 0,
            at_end: true,
        });
    }

    fn advance_to_prev_leaf(&mut self) {
        // Pop current leaf
        self.stack.pop();
        
        // Find previous leaf by going up the stack
        while let Some(frame) = self.stack.last_mut() {
            if let BTreeNodeType::Internal { children, .. } = &frame.node.node_type {
                if frame.child_index > 0 {
                    // Move to previous child
                    frame.child_index -= 1;
                    let prev_child = &children[frame.child_index];
                    
                    // Go down to rightmost leaf of this child
                    self.descend_to_rightmost_leaf(prev_child);
                    return;
                }
            }
            
            // No more children at this level, go up
            self.stack.pop();
        }
        
        // Reached beginning of tree
        self.current_position = Some(CursorPosition {
            key_index: 0,
            at_end: true,
        });
    }

    fn descend_to_leftmost_leaf(&mut self, node: &'a BTreeNode) {
        let mut current = node;
        
        loop {
            match &current.node_type {
                BTreeNodeType::Leaf { keys, .. } => {
                    self.stack.push(CursorFrame {
                        node: current,
                        key_index: 0,
                        child_index: 0,
                    });
                    
                    self.current_position = Some(CursorPosition {
                        key_index: 0,
                        at_end: keys.is_empty(),
                    });
                    break;
                }
                BTreeNodeType::Internal { children, .. } => {
                    self.stack.push(CursorFrame {
                        node: current,
                        key_index: 0,
                        child_index: 0,
                    });
                    
                    current = &children[0];
                }
            }
        }
    }

    fn descend_to_rightmost_leaf(&mut self, node: &'a BTreeNode) {
        let mut current = node;
        
        loop {
            match &current.node_type {
                BTreeNodeType::Leaf { keys, .. } => {
                    let last_index = if keys.is_empty() { 0 } else { keys.len() - 1 };
                    
                    self.stack.push(CursorFrame {
                        node: current,
                        key_index: last_index,
                        child_index: 0,
                    });
                    
                    self.current_position = Some(CursorPosition {
                        key_index: last_index,
                        at_end: keys.is_empty(),
                    });
                    break;
                }
                BTreeNodeType::Internal { children, .. } => {
                    let last_child_index = children.len() - 1;
                    
                    self.stack.push(CursorFrame {
                        node: current,
                        key_index: 0,
                        child_index: last_child_index,
                    });
                    
                    current = &children[last_child_index];
                }
            }
        }
    }
}

/// Iterator implementation for BTreeCursor
impl<'a> Iterator for BTreeCursor<'a> {
    type Item = (Value, RowId, Row);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some((key, row_id, row)) = self.current() {
            let result = (key.clone(), *row_id, row.clone());
            self.next();
            Some(result)
        } else {
            None
        }
    }
}
