pub mod node;
pub mod cursor;
pub mod algebra;

pub use node::{BTreeNode, BTreeNodeType};
pub use cursor::BTreeCursor;
pub use algebra::BTreeAlgebra;

use crate::error::{SqlError, SqlResult};
use crate::types::{Row, RowId, Value};
use std::sync::Arc;

/// B-Tree implementation following algebraic patterns
#[derive(Debug, Clone)]
pub struct BTree {
    root: Arc<BTreeNode>,
    order: usize, // Maximum number of keys per node
}

impl BTree {
    pub fn new(order: usize) -> Self {
        BTree {
            root: Arc::new(BTreeNode::new_leaf()),
            order,
        }
    }

    pub fn empty() -> Self {
        Self::new(4) // Default order
    }

    /// Insert a key-value pair into the B-Tree
    pub fn insert(&mut self, key: Value, row_id: RowId, row: Row) -> SqlResult<()> {
        let new_root = self.insert_recursive(&self.root, key, row_id, row)?;
        self.root = Arc::new(new_root);
        Ok(())
    }

    /// Search for a value by key
    pub fn search(&self, key: &Value) -> SqlResult<Option<(RowId, Row)>> {
        self.search_recursive(&self.root, key)
    }

    /// Delete a key from the B-Tree
    pub fn delete(&mut self, key: &Value) -> SqlResult<Option<(RowId, Row)>> {
        let (deleted, new_root) = self.delete_recursive(&self.root, key)?;
        self.root = Arc::new(new_root);
        Ok(deleted)
    }

    /// Create a cursor for iterating over the B-Tree
    pub fn cursor(&self) -> BTreeCursor {
        BTreeCursor::new(&self.root)
    }

    /// Get all key-value pairs in sorted order
    pub fn scan(&self) -> SqlResult<Vec<(Value, RowId, Row)>> {
        let mut results = Vec::new();
        self.scan_recursive(&self.root, &mut results)?;
        Ok(results)
    }

    /// Range scan between two keys
    pub fn range_scan(&self, start: &Value, end: &Value) -> SqlResult<Vec<(Value, RowId, Row)>> {
        let mut results = Vec::new();
        self.range_scan_recursive(&self.root, start, end, &mut results)?;
        Ok(results)
    }

    // Private helper methods
    fn insert_recursive(
        &self,
        node: &BTreeNode,
        key: Value,
        row_id: RowId,
        row: Row,
    ) -> SqlResult<BTreeNode> {
        match &node.node_type {
            BTreeNodeType::Leaf { keys, values } => {
                let mut new_keys = keys.clone();
                let mut new_values = values.clone();

                // Find insertion position
                let pos = new_keys
                    .binary_search(&key)
                    .unwrap_or_else(|pos| pos);

                // Insert key and value
                new_keys.insert(pos, key);
                new_values.insert(pos, (row_id, row));

                // Check if split is needed
                if new_keys.len() > self.order {
                    self.split_leaf(new_keys, new_values)
                } else {
                    Ok(BTreeNode {
                        node_type: BTreeNodeType::Leaf {
                            keys: new_keys,
                            values: new_values,
                        },
                    })
                }
            }
            BTreeNodeType::Internal { keys, children } => {
                // Find child to insert into
                let pos = keys.partition_point(|k| k < &key);
                let child = &children[pos];

                // Recursively insert into child
                let new_child = self.insert_recursive(child, key, row_id, row)?;

                // Update children
                let mut new_children = children.clone();
                new_children[pos] = Arc::new(new_child);

                // Check if child was split
                if new_children[pos].is_overfull(self.order) {
                    self.split_internal(keys.clone(), new_children, pos)
                } else {
                    Ok(BTreeNode {
                        node_type: BTreeNodeType::Internal {
                            keys: keys.clone(),
                            children: new_children,
                        },
                    })
                }
            }
        }
    }

    fn search_recursive(&self, node: &BTreeNode, key: &Value) -> SqlResult<Option<(RowId, Row)>> {
        match &node.node_type {
            BTreeNodeType::Leaf { keys, values } => {
                if let Ok(pos) = keys.binary_search(key) {
                    Ok(Some(values[pos].clone()))
                } else {
                    Ok(None)
                }
            }
            BTreeNodeType::Internal { keys, children } => {
                let pos = keys.partition_point(|k| k < key);
                self.search_recursive(&children[pos], key)
            }
        }
    }

    fn delete_recursive(
        &self,
        node: &BTreeNode,
        key: &Value,
    ) -> SqlResult<(Option<(RowId, Row)>, BTreeNode)> {
        match &node.node_type {
            BTreeNodeType::Leaf { keys, values } => {
                if let Ok(pos) = keys.binary_search(key) {
                    let mut new_keys = keys.clone();
                    let mut new_values = values.clone();
                    let deleted = new_values.remove(pos);
                    new_keys.remove(pos);

                    Ok((
                        Some(deleted),
                        BTreeNode {
                            node_type: BTreeNodeType::Leaf {
                                keys: new_keys,
                                values: new_values,
                            },
                        },
                    ))
                } else {
                    Ok((None, node.clone()))
                }
            }
            BTreeNodeType::Internal { keys, children } => {
                let pos = keys.partition_point(|k| k < key);
                let (deleted, new_child) = self.delete_recursive(&children[pos], key)?;

                let mut new_children = children.clone();
                new_children[pos] = Arc::new(new_child);

                Ok((
                    deleted,
                    BTreeNode {
                        node_type: BTreeNodeType::Internal {
                            keys: keys.clone(),
                            children: new_children,
                        },
                    },
                ))
            }
        }
    }

    fn scan_recursive(
        &self,
        node: &BTreeNode,
        results: &mut Vec<(Value, RowId, Row)>,
    ) -> SqlResult<()> {
        match &node.node_type {
            BTreeNodeType::Leaf { keys, values } => {
                for (key, (row_id, row)) in keys.iter().zip(values.iter()) {
                    results.push((key.clone(), *row_id, row.clone()));
                }
                Ok(())
            }
            BTreeNodeType::Internal { keys: _, children } => {
                for child in children {
                    self.scan_recursive(child, results)?;
                }
                Ok(())
            }
        }
    }

    fn range_scan_recursive(
        &self,
        node: &BTreeNode,
        start: &Value,
        end: &Value,
        results: &mut Vec<(Value, RowId, Row)>,
    ) -> SqlResult<()> {
        match &node.node_type {
            BTreeNodeType::Leaf { keys, values } => {
                for (key, (row_id, row)) in keys.iter().zip(values.iter()) {
                    if key >= start && key <= end {
                        results.push((key.clone(), *row_id, row.clone()));
                    }
                }
                Ok(())
            }
            BTreeNodeType::Internal { keys, children } => {
                for (i, child) in children.iter().enumerate() {
                    // Check if this child might contain values in range
                    let should_search = if i == 0 {
                        true // First child
                    } else if i == children.len() - 1 {
                        keys[i - 1] <= *end // Last child
                    } else {
                        keys[i - 1] <= *end && keys[i] >= *start
                    };

                    if should_search {
                        self.range_scan_recursive(child, start, end, results)?;
                    }
                }
                Ok(())
            }
        }
    }

    fn split_leaf(
        &self,
        keys: Vec<Value>,
        values: Vec<(RowId, Row)>,
    ) -> SqlResult<BTreeNode> {
        let mid = keys.len() / 2;
        let (left_keys, right_keys) = keys.split_at(mid);
        let (left_values, right_values) = values.split_at(mid);

        let left_child = Arc::new(BTreeNode {
            node_type: BTreeNodeType::Leaf {
                keys: left_keys.to_vec(),
                values: left_values.to_vec(),
            },
        });

        let right_child = Arc::new(BTreeNode {
            node_type: BTreeNodeType::Leaf {
                keys: right_keys.to_vec(),
                values: right_values.to_vec(),
            },
        });

        // Promote middle key to new internal node
        Ok(BTreeNode {
            node_type: BTreeNodeType::Internal {
                keys: vec![right_keys[0].clone()],
                children: vec![left_child, right_child],
            },
        })
    }

    fn split_internal(
        &self,
        keys: Vec<Value>,
        children: Vec<Arc<BTreeNode>>,
        _split_pos: usize,
    ) -> SqlResult<BTreeNode> {
        // For simplicity, just return the original node
        // In a full implementation, this would handle internal node splits
        Ok(BTreeNode {
            node_type: BTreeNodeType::Internal { keys, children },
        })
    }
}
