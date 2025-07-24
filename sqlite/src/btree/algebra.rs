use crate::btree::node::{BTreeNode, BTreeNodeType};
use crate::error::{SqlError, SqlResult};
use crate::types::{Row, RowId, Value};
use std::sync::Arc;

/// B-Tree algebra for compositional operations
pub struct BTreeAlgebra;

impl BTreeAlgebra {
    /// Insert operation (algebra morphism)
    pub fn insert(
        node: &BTreeNode,
        key: Value,
        row_id: RowId,
        row: Row,
        order: usize,
    ) -> SqlResult<BTreeNode> {
        match &node.node_type {
            BTreeNodeType::Leaf { keys, values } => {
                Self::insert_into_leaf(keys, values, key, row_id, row, order)
            }
            BTreeNodeType::Internal { keys, children } => {
                Self::insert_into_internal(keys, children, key, row_id, row, order)
            }
        }
    }

    /// Search operation (algebra homomorphism)
    pub fn search(node: &BTreeNode, key: &Value) -> SqlResult<Option<(RowId, Row)>> {
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
                Self::search(&children[pos], key)
            }
        }
    }

    /// Delete operation (algebra morphism)
    pub fn delete(
        node: &BTreeNode,
        key: &Value,
        order: usize,
    ) -> SqlResult<(Option<(RowId, Row)>, BTreeNode)> {
        match &node.node_type {
            BTreeNodeType::Leaf { keys, values } => {
                Self::delete_from_leaf(keys, values, key)
            }
            BTreeNodeType::Internal { keys, children } => {
                Self::delete_from_internal(keys, children, key, order)
            }
        }
    }

    /// Catamorphism: fold over B-Tree structure
    pub fn fold<A, F>(node: &BTreeNode, init: A, f: F) -> A
    where
        F: Fn(A, &Value, Option<&(RowId, Row)>) -> A + Clone,
    {
        node.fold(init, f)
    }

    /// Anamorphism: unfold B-Tree from seed
    pub fn unfold<S, F>(seed: S, f: F, order: usize) -> SqlResult<BTreeNode>
    where
        F: Fn(S) -> Option<(Value, RowId, Row, S)>,
    {
        let mut node = BTreeNode::new_leaf();
        let mut current_seed = seed;

        while let Some((key, row_id, row, next_seed)) = f(current_seed) {
            node = Self::insert(&node, key, row_id, row, order)?;
            current_seed = next_seed;
        }

        Ok(node)
    }

    /// Map operation (functor)
    pub fn map<F>(node: &BTreeNode, f: F) -> BTreeNode
    where
        F: Fn(&Value) -> Value + Clone,
    {
        node.map_keys(f)
    }

    /// Filter operation
    pub fn filter<P>(node: &BTreeNode, predicate: P) -> SqlResult<BTreeNode>
    where
        P: Fn(&Value, &(RowId, Row)) -> bool,
    {
        match &node.node_type {
            BTreeNodeType::Leaf { keys, values } => {
                let mut new_keys = Vec::new();
                let mut new_values = Vec::new();

                for (key, value) in keys.iter().zip(values.iter()) {
                    if predicate(key, value) {
                        new_keys.push(key.clone());
                        new_values.push(value.clone());
                    }
                }

                Ok(BTreeNode {
                    node_type: BTreeNodeType::Leaf {
                        keys: new_keys,
                        values: new_values,
                    },
                })
            }
            BTreeNodeType::Internal { keys, children } => {
                let filtered_children: SqlResult<Vec<_>> = children
                    .iter()
                    .map(|child| Self::filter(child, &predicate))
                    .collect();

                Ok(BTreeNode {
                    node_type: BTreeNodeType::Internal {
                        keys: keys.clone(),
                        children: filtered_children?.into_iter().map(Arc::new).collect(),
                    },
                })
            }
        }
    }

    /// Merge two B-Trees (monoid operation)
    pub fn merge(left: &BTreeNode, right: &BTreeNode, order: usize) -> SqlResult<BTreeNode> {
        // Collect all key-value pairs from both trees
        let mut all_pairs = Vec::new();
        
        Self::collect_all_pairs(left, &mut all_pairs);
        Self::collect_all_pairs(right, &mut all_pairs);
        
        // Sort by key
        all_pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        
        // Build new tree
        let mut result = BTreeNode::new_leaf();
        for (key, row_id, row) in all_pairs {
            result = Self::insert(&result, key, row_id, row, order)?;
        }
        
        Ok(result)
    }

    /// Split B-Tree at a given key
    pub fn split_at(
        node: &BTreeNode,
        split_key: &Value,
    ) -> SqlResult<(BTreeNode, BTreeNode)> {
        let mut left_pairs = Vec::new();
        let mut right_pairs = Vec::new();
        
        Self::collect_all_pairs(node, &mut left_pairs);
        
        // Partition pairs
        let mut temp_pairs = Vec::new();
        temp_pairs.append(&mut left_pairs);

        for pair in temp_pairs {
            if pair.0 < *split_key {
                left_pairs.push(pair);
            } else {
                right_pairs.push(pair);
            }
        }
        
        // Build trees
        let mut left_tree = BTreeNode::new_leaf();
        let mut right_tree = BTreeNode::new_leaf();
        
        for (key, row_id, row) in left_pairs {
            left_tree = Self::insert(&left_tree, key, row_id, row, 4)?;
        }
        
        for (key, row_id, row) in right_pairs {
            right_tree = Self::insert(&right_tree, key, row_id, row, 4)?;
        }
        
        Ok((left_tree, right_tree))
    }

    /// Range query (algebra morphism)
    pub fn range_query(
        node: &BTreeNode,
        start: &Value,
        end: &Value,
    ) -> SqlResult<Vec<(Value, RowId, Row)>> {
        let mut results = Vec::new();
        Self::range_query_recursive(node, start, end, &mut results)?;
        Ok(results)
    }

    // Private helper methods
    fn insert_into_leaf(
        keys: &[Value],
        values: &[(RowId, Row)],
        key: Value,
        row_id: RowId,
        row: Row,
        order: usize,
    ) -> SqlResult<BTreeNode> {
        let mut new_keys = keys.to_vec();
        let mut new_values = values.to_vec();

        let pos = new_keys.partition_point(|k| k < &key);
        new_keys.insert(pos, key);
        new_values.insert(pos, (row_id, row));

        if new_keys.len() > order {
            // Split leaf
            let mid = new_keys.len() / 2;
            let (left_keys, right_keys) = new_keys.split_at(mid);
            let (left_values, right_values) = new_values.split_at(mid);

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

            Ok(BTreeNode {
                node_type: BTreeNodeType::Internal {
                    keys: vec![right_keys[0].clone()],
                    children: vec![left_child, right_child],
                },
            })
        } else {
            Ok(BTreeNode {
                node_type: BTreeNodeType::Leaf {
                    keys: new_keys,
                    values: new_values,
                },
            })
        }
    }

    fn insert_into_internal(
        keys: &[Value],
        children: &[Arc<BTreeNode>],
        key: Value,
        row_id: RowId,
        row: Row,
        order: usize,
    ) -> SqlResult<BTreeNode> {
        let pos = keys.partition_point(|k| k < &key);
        let child = &children[pos];

        let new_child = Self::insert(child, key, row_id, row, order)?;
        let mut new_children = children.to_vec();
        new_children[pos] = Arc::new(new_child);

        Ok(BTreeNode {
            node_type: BTreeNodeType::Internal {
                keys: keys.to_vec(),
                children: new_children,
            },
        })
    }

    fn delete_from_leaf(
        keys: &[Value],
        values: &[(RowId, Row)],
        key: &Value,
    ) -> SqlResult<(Option<(RowId, Row)>, BTreeNode)> {
        if let Ok(pos) = keys.binary_search(key) {
            let mut new_keys = keys.to_vec();
            let mut new_values = values.to_vec();
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
            Ok((
                None,
                BTreeNode {
                    node_type: BTreeNodeType::Leaf {
                        keys: keys.to_vec(),
                        values: values.to_vec(),
                    },
                },
            ))
        }
    }

    fn delete_from_internal(
        keys: &[Value],
        children: &[Arc<BTreeNode>],
        key: &Value,
        order: usize,
    ) -> SqlResult<(Option<(RowId, Row)>, BTreeNode)> {
        let pos = keys.partition_point(|k| k < key);
        let (deleted, new_child) = Self::delete(&children[pos], key, order)?;

        let mut new_children = children.to_vec();
        new_children[pos] = Arc::new(new_child);

        Ok((
            deleted,
            BTreeNode {
                node_type: BTreeNodeType::Internal {
                    keys: keys.to_vec(),
                    children: new_children,
                },
            },
        ))
    }

    fn collect_all_pairs(node: &BTreeNode, pairs: &mut Vec<(Value, RowId, Row)>) {
        match &node.node_type {
            BTreeNodeType::Leaf { keys, values } => {
                for (key, (row_id, row)) in keys.iter().zip(values.iter()) {
                    pairs.push((key.clone(), *row_id, row.clone()));
                }
            }
            BTreeNodeType::Internal { children, .. } => {
                for child in children {
                    Self::collect_all_pairs(child, pairs);
                }
            }
        }
    }

    fn range_query_recursive(
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
                    let should_search = if i == 0 {
                        true
                    } else if i == children.len() - 1 {
                        keys[i - 1] <= *end
                    } else {
                        keys[i - 1] <= *end && keys[i] >= *start
                    };

                    if should_search {
                        Self::range_query_recursive(child, start, end, results)?;
                    }
                }
                Ok(())
            }
        }
    }
}
