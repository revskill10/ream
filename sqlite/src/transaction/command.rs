use crate::types::{Row, Value};
use serde::{Deserialize, Serialize};

/// SQL command types for transaction logging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SqlCommand {
    /// Insert a row into a table
    Insert {
        table: String,
        row: Row,
    },
    /// Update a row in a table
    Update {
        table: String,
        key: Value,
        old_row: Row,
        new_row: Row,
    },
    /// Delete a row from a table
    Delete {
        table: String,
        key: Value,
        old_row: Row,
    },
    /// Create a new table
    CreateTable {
        name: String,
        schema: String, // Simplified schema representation
    },
    /// Drop a table
    DropTable {
        name: String,
        schema: String, // For rollback
    },
    /// Create an index
    CreateIndex {
        name: String,
        table: String,
        columns: Vec<String>,
    },
    /// Drop an index
    DropIndex {
        name: String,
        table: String,
        columns: Vec<String>, // For rollback
    },
    /// Begin transaction marker
    Begin,
    /// Commit transaction marker
    Commit,
    /// Rollback transaction marker
    Rollback,
    /// Checkpoint marker
    Checkpoint,
}

impl SqlCommand {
    /// Check if this command is read-only
    pub fn is_read_only(&self) -> bool {
        matches!(self, SqlCommand::Begin)
    }

    /// Check if this command modifies data
    pub fn is_write_operation(&self) -> bool {
        matches!(
            self,
            SqlCommand::Insert { .. }
                | SqlCommand::Update { .. }
                | SqlCommand::Delete { .. }
                | SqlCommand::CreateTable { .. }
                | SqlCommand::DropTable { .. }
                | SqlCommand::CreateIndex { .. }
                | SqlCommand::DropIndex { .. }
        )
    }

    /// Get the table affected by this command
    pub fn affected_table(&self) -> Option<String> {
        match self {
            SqlCommand::Insert { table, .. }
            | SqlCommand::Update { table, .. }
            | SqlCommand::Delete { table, .. }
            | SqlCommand::CreateIndex { table, .. }
            | SqlCommand::DropIndex { table, .. } => Some(table.clone()),
            SqlCommand::CreateTable { name, .. } | SqlCommand::DropTable { name, .. } => {
                Some(name.clone())
            }
            _ => None,
        }
    }

    /// Get the operation type as a string
    pub fn operation_type(&self) -> &'static str {
        match self {
            SqlCommand::Insert { .. } => "INSERT",
            SqlCommand::Update { .. } => "UPDATE",
            SqlCommand::Delete { .. } => "DELETE",
            SqlCommand::CreateTable { .. } => "CREATE_TABLE",
            SqlCommand::DropTable { .. } => "DROP_TABLE",
            SqlCommand::CreateIndex { .. } => "CREATE_INDEX",
            SqlCommand::DropIndex { .. } => "DROP_INDEX",
            SqlCommand::Begin => "BEGIN",
            SqlCommand::Commit => "COMMIT",
            SqlCommand::Rollback => "ROLLBACK",
            SqlCommand::Checkpoint => "CHECKPOINT",
        }
    }

    /// Get the inverse command for rollback purposes
    pub fn inverse(&self) -> Option<SqlCommand> {
        match self {
            SqlCommand::Insert { table, row } => Some(SqlCommand::Delete {
                table: table.clone(),
                key: row.values.get(0).cloned().unwrap_or(Value::Null), // Simplified
                old_row: row.clone(),
            }),
            SqlCommand::Update {
                table,
                key,
                old_row,
                new_row: _,
            } => Some(SqlCommand::Update {
                table: table.clone(),
                key: key.clone(),
                old_row: old_row.clone(),
                new_row: old_row.clone(),
            }),
            SqlCommand::Delete {
                table, old_row, ..
            } => Some(SqlCommand::Insert {
                table: table.clone(),
                row: old_row.clone(),
            }),
            SqlCommand::CreateTable { name, .. } => Some(SqlCommand::DropTable {
                name: name.clone(),
                schema: "".to_string(), // Would need actual schema
            }),
            SqlCommand::DropTable { name, schema } => Some(SqlCommand::CreateTable {
                name: name.clone(),
                schema: schema.clone(),
            }),
            SqlCommand::CreateIndex {
                name,
                table,
                columns,
            } => Some(SqlCommand::DropIndex {
                name: name.clone(),
                table: table.clone(),
                columns: columns.clone(),
            }),
            SqlCommand::DropIndex {
                name,
                table,
                columns,
            } => Some(SqlCommand::CreateIndex {
                name: name.clone(),
                table: table.clone(),
                columns: columns.clone(),
            }),
            _ => None,
        }
    }

    /// Estimate the size of this command in bytes
    pub fn estimated_size(&self) -> usize {
        match self {
            SqlCommand::Insert { table, row } => {
                table.len() + row.values.iter().map(|v| self.value_size(v)).sum::<usize>()
            }
            SqlCommand::Update {
                table,
                key,
                old_row,
                new_row,
            } => {
                table.len()
                    + self.value_size(key)
                    + old_row.values.iter().map(|v| self.value_size(v)).sum::<usize>()
                    + new_row.values.iter().map(|v| self.value_size(v)).sum::<usize>()
            }
            SqlCommand::Delete {
                table, key, old_row, ..
            } => {
                table.len()
                    + self.value_size(key)
                    + old_row.values.iter().map(|v| self.value_size(v)).sum::<usize>()
            }
            SqlCommand::CreateTable { name, schema } => name.len() + schema.len(),
            SqlCommand::DropTable { name, schema } => name.len() + schema.len(),
            SqlCommand::CreateIndex {
                name,
                table,
                columns,
            } => {
                name.len()
                    + table.len()
                    + columns.iter().map(|c| c.len()).sum::<usize>()
            }
            SqlCommand::DropIndex {
                name,
                table,
                columns,
            } => {
                name.len()
                    + table.len()
                    + columns.iter().map(|c| c.len()).sum::<usize>()
            }
            _ => 8, // Small fixed size for control commands
        }
    }

    /// Get command priority for ordering
    pub fn priority(&self) -> u8 {
        match self {
            SqlCommand::Begin => 0,
            SqlCommand::CreateTable { .. } | SqlCommand::CreateIndex { .. } => 1,
            SqlCommand::Insert { .. } => 2,
            SqlCommand::Update { .. } => 3,
            SqlCommand::Delete { .. } => 4,
            SqlCommand::DropIndex { .. } | SqlCommand::DropTable { .. } => 5,
            SqlCommand::Commit | SqlCommand::Rollback => 6,
            SqlCommand::Checkpoint => 7,
        }
    }

    /// Check if this command can be batched with others
    pub fn is_batchable(&self) -> bool {
        matches!(
            self,
            SqlCommand::Insert { .. } | SqlCommand::Update { .. } | SqlCommand::Delete { .. }
        )
    }

    /// Get a human-readable description of the command
    pub fn description(&self) -> String {
        match self {
            SqlCommand::Insert { table, .. } => format!("Insert into {}", table),
            SqlCommand::Update { table, .. } => format!("Update {}", table),
            SqlCommand::Delete { table, .. } => format!("Delete from {}", table),
            SqlCommand::CreateTable { name, .. } => format!("Create table {}", name),
            SqlCommand::DropTable { name, .. } => format!("Drop table {}", name),
            SqlCommand::CreateIndex { name, table, .. } => {
                format!("Create index {} on {}", name, table)
            }
            SqlCommand::DropIndex { name, table, .. } => {
                format!("Drop index {} on {}", name, table)
            }
            SqlCommand::Begin => "Begin transaction".to_string(),
            SqlCommand::Commit => "Commit transaction".to_string(),
            SqlCommand::Rollback => "Rollback transaction".to_string(),
            SqlCommand::Checkpoint => "Checkpoint".to_string(),
        }
    }

    // Helper method to estimate value size
    fn value_size(&self, value: &Value) -> usize {
        match value {
            Value::Null => 1,
            Value::Boolean(_) => 1,
            Value::Integer(_) => 8,
            Value::Real(_) => 8,
            Value::Text(s) => s.len(),
            Value::Blob(b) => b.len(),
        }
    }
}

/// Command batch for efficient processing
#[derive(Debug, Clone)]
pub struct CommandBatch {
    pub commands: Vec<SqlCommand>,
    pub batch_id: uuid::Uuid,
    pub created_at: std::time::SystemTime,
}

impl CommandBatch {
    /// Create a new command batch
    pub fn new() -> Self {
        CommandBatch {
            commands: Vec::new(),
            batch_id: uuid::Uuid::new_v4(),
            created_at: std::time::SystemTime::now(),
        }
    }

    /// Add a command to the batch
    pub fn add_command(&mut self, command: SqlCommand) {
        self.commands.push(command);
    }

    /// Check if the batch can accept more commands
    pub fn can_add_command(&self, command: &SqlCommand) -> bool {
        // Check if command is batchable
        if !command.is_batchable() {
            return false;
        }

        // Check batch size limits
        if self.commands.len() >= 1000 {
            return false;
        }

        // Check if all commands in batch affect the same table
        if let Some(table) = command.affected_table() {
            if let Some(first_command) = self.commands.first() {
                if let Some(first_table) = first_command.affected_table() {
                    return table == first_table;
                }
            }
        }

        true
    }

    /// Get total estimated size of the batch
    pub fn estimated_size(&self) -> usize {
        self.commands.iter().map(|cmd| cmd.estimated_size()).sum()
    }

    /// Get the primary table affected by this batch
    pub fn primary_table(&self) -> Option<String> {
        self.commands.first().and_then(|cmd| cmd.affected_table())
    }

    /// Check if the batch is empty
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Get command count
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Sort commands by priority
    pub fn sort_by_priority(&mut self) {
        self.commands.sort_by_key(|cmd| cmd.priority());
    }

    /// Split batch into smaller batches if needed
    pub fn split_if_needed(&self, max_size: usize) -> Vec<CommandBatch> {
        if self.commands.len() <= max_size {
            return vec![self.clone()];
        }

        let mut batches = Vec::new();
        for chunk in self.commands.chunks(max_size) {
            let mut batch = CommandBatch::new();
            batch.commands = chunk.to_vec();
            batches.push(batch);
        }

        batches
    }
}

impl Default for CommandBatch {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_properties() {
        let insert_cmd = SqlCommand::Insert {
            table: "users".to_string(),
            row: Row::new(vec![Value::Integer(1), Value::Text("Alice".to_string())]),
        };

        assert!(!insert_cmd.is_read_only());
        assert!(insert_cmd.is_write_operation());
        assert_eq!(insert_cmd.affected_table(), Some("users".to_string()));
        assert_eq!(insert_cmd.operation_type(), "INSERT");
        assert!(insert_cmd.is_batchable());
    }

    #[test]
    fn test_command_inverse() {
        let insert_cmd = SqlCommand::Insert {
            table: "users".to_string(),
            row: Row::new(vec![Value::Integer(1), Value::Text("Alice".to_string())]),
        };

        let inverse = insert_cmd.inverse().unwrap();
        match inverse {
            SqlCommand::Delete { table, .. } => {
                assert_eq!(table, "users");
            }
            _ => panic!("Expected DELETE command"),
        }
    }

    #[test]
    fn test_command_batch() {
        let mut batch = CommandBatch::new();
        
        let cmd1 = SqlCommand::Insert {
            table: "users".to_string(),
            row: Row::new(vec![Value::Integer(1)]),
        };
        
        let cmd2 = SqlCommand::Insert {
            table: "users".to_string(),
            row: Row::new(vec![Value::Integer(2)]),
        };

        assert!(batch.can_add_command(&cmd1));
        batch.add_command(cmd1);
        
        assert!(batch.can_add_command(&cmd2));
        batch.add_command(cmd2);
        
        assert_eq!(batch.len(), 2);
        assert_eq!(batch.primary_table(), Some("users".to_string()));
    }

    #[test]
    fn test_command_size_estimation() {
        let cmd = SqlCommand::Insert {
            table: "users".to_string(),
            row: Row::new(vec![
                Value::Integer(1),
                Value::Text("Alice".to_string()),
            ]),
        };

        let size = cmd.estimated_size();
        assert!(size > 0);
        // "users" (5) + integer (8) + "Alice" (5) = at least 18 bytes
        assert!(size >= 18);
    }
}
