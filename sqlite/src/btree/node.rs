use crate::types::{Row, RowId, Value};
use std::sync::Arc;

/// B-Tree node following algebraic structure
#[derive(Debug, Clone)]
pub struct BTreeNode {
    pub node_type: BTreeNodeType,
}

/// B-Tree node types as algebraic data structure
#[derive(Debug, Clone)]
pub enum BTreeNodeType {
    Leaf {
        keys: Vec<Value>,
        values: Vec<(RowId, Row)>,
    },
    Internal {
        keys: Vec<Value>,
        children: Vec<Arc<BTreeNode>>,
    },
}

impl BTreeNode {
    /// Create a new empty leaf node
    pub fn new_leaf() -> Self {
        BTreeNode {
            node_type: BTreeNodeType::Leaf {
                keys: Vec::new(),
                values: Vec::new(),
            },
        }
    }

    /// Create a new internal node
    pub fn new_internal(keys: Vec<Value>, children: Vec<Arc<BTreeNode>>) -> Self {
        BTreeNode {
            node_type: BTreeNodeType::Internal { keys, children },
        }
    }

    /// Check if node is a leaf
    pub fn is_leaf(&self) -> bool {
        matches!(self.node_type, BTreeNodeType::Leaf { .. })
    }

    /// Check if node is internal
    pub fn is_internal(&self) -> bool {
        matches!(self.node_type, BTreeNodeType::Internal { .. })
    }

    /// Get number of keys in the node
    pub fn key_count(&self) -> usize {
        match &self.node_type {
            BTreeNodeType::Leaf { keys, .. } => keys.len(),
            BTreeNodeType::Internal { keys, .. } => keys.len(),
        }
    }

    /// Check if node is overfull (needs splitting)
    pub fn is_overfull(&self, order: usize) -> bool {
        self.key_count() > order
    }

    /// Check if node is underfull (needs merging)
    pub fn is_underfull(&self, order: usize) -> bool {
        self.key_count() < order / 2
    }

    /// Get keys from the node
    pub fn keys(&self) -> &Vec<Value> {
        match &self.node_type {
            BTreeNodeType::Leaf { keys, .. } => keys,
            BTreeNodeType::Internal { keys, .. } => keys,
        }
    }

    /// Get values from leaf node
    pub fn values(&self) -> Option<&Vec<(RowId, Row)>> {
        match &self.node_type {
            BTreeNodeType::Leaf { values, .. } => Some(values),
            BTreeNodeType::Internal { .. } => None,
        }
    }

    /// Get children from internal node
    pub fn children(&self) -> Option<&Vec<Arc<BTreeNode>>> {
        match &self.node_type {
            BTreeNodeType::Leaf { .. } => None,
            BTreeNodeType::Internal { children, .. } => Some(children),
        }
    }

    /// Find the position where a key should be inserted
    pub fn find_key_position(&self, key: &Value) -> usize {
        self.keys().partition_point(|k| k < key)
    }

    /// Find a key in the node
    pub fn find_key(&self, key: &Value) -> Option<usize> {
        self.keys().binary_search(key).ok()
    }

    /// Get minimum key in the subtree rooted at this node
    pub fn min_key(&self) -> Option<&Value> {
        match &self.node_type {
            BTreeNodeType::Leaf { keys, .. } => keys.first(),
            BTreeNodeType::Internal { children, .. } => {
                children.first().and_then(|child| child.min_key())
            }
        }
    }

    /// Get maximum key in the subtree rooted at this node
    pub fn max_key(&self) -> Option<&Value> {
        match &self.node_type {
            BTreeNodeType::Leaf { keys, .. } => keys.last(),
            BTreeNodeType::Internal { children, .. } => {
                children.last().and_then(|child| child.max_key())
            }
        }
    }

    /// Catamorphism: fold over the B-Tree structure
    pub fn fold<A, F>(&self, init: A, f: F) -> A
    where
        F: Fn(A, &Value, Option<&(RowId, Row)>) -> A + Clone,
    {
        match &self.node_type {
            BTreeNodeType::Leaf { keys, values } => {
                keys.iter()
                    .zip(values.iter())
                    .fold(init, |acc, (key, value)| f(acc, key, Some(value)))
            }
            BTreeNodeType::Internal { keys, children } => {
                let acc = keys
                    .iter()
                    .fold(init, |acc, key| f(acc, key, None));
                
                children
                    .iter()
                    .fold(acc, |acc, child| child.fold(acc, f.clone()))
            }
        }
    }

    /// Map over all keys in the node (functor operation)
    pub fn map_keys<F>(&self, f: F) -> Self
    where
        F: Fn(&Value) -> Value,
    {
        match &self.node_type {
            BTreeNodeType::Leaf { keys, values } => BTreeNode {
                node_type: BTreeNodeType::Leaf {
                    keys: keys.iter().map(&f).collect(),
                    values: values.clone(),
                },
            },
            BTreeNodeType::Internal { keys, children } => BTreeNode {
                node_type: BTreeNodeType::Internal {
                    keys: keys.iter().map(&f).collect(),
                    children: children.iter().map(|child| Arc::new(child.map_keys(&f))).collect(),
                },
            },
        }
    }

    /// Check structural invariants
    pub fn validate(&self, order: usize) -> bool {
        match &self.node_type {
            BTreeNodeType::Leaf { keys, values } => {
                // Check that keys and values have same length
                if keys.len() != values.len() {
                    return false;
                }
                
                // Check that keys are sorted
                for i in 1..keys.len() {
                    if keys[i - 1] >= keys[i] {
                        return false;
                    }
                }
                
                // Check order constraint
                keys.len() <= order
            }
            BTreeNodeType::Internal { keys, children } => {
                // Check that children count is keys count + 1
                if children.len() != keys.len() + 1 {
                    return false;
                }
                
                // Check that keys are sorted
                for i in 1..keys.len() {
                    if keys[i - 1] >= keys[i] {
                        return false;
                    }
                }
                
                // Check order constraint
                if keys.len() > order {
                    return false;
                }
                
                // Recursively validate children
                for child in children {
                    if !child.validate(order) {
                        return false;
                    }
                }
                
                // Check key ordering between children
                for i in 0..keys.len() {
                    if let (Some(left_max), Some(right_min)) = (
                        children[i].max_key(),
                        children[i + 1].min_key(),
                    ) {
                        if left_max >= &keys[i] || &keys[i] >= right_min {
                            return false;
                        }
                    }
                }
                
                true
            }
        }
    }

    /// Get height of the subtree rooted at this node
    pub fn height(&self) -> usize {
        match &self.node_type {
            BTreeNodeType::Leaf { .. } => 0,
            BTreeNodeType::Internal { children, .. } => {
                1 + children.iter().map(|child| child.height()).max().unwrap_or(0)
            }
        }
    }

    /// Count total number of keys in the subtree
    pub fn total_keys(&self) -> usize {
        match &self.node_type {
            BTreeNodeType::Leaf { keys, .. } => keys.len(),
            BTreeNodeType::Internal { keys, children } => {
                keys.len() + children.iter().map(|child| child.total_keys()).sum::<usize>()
            }
        }
    }
}
