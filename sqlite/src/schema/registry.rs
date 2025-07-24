use crate::error::{SqlError, SqlResult};
use crate::schema::{Schema, SchemaStatistics};
use std::collections::HashMap;

/// Schema registry for managing database schemas
#[derive(Debug, Clone)]
pub struct SchemaRegistry {
    schemas: HashMap<String, Schema>,
    version: u64,
}

impl SchemaRegistry {
    pub fn new() -> Self {
        SchemaRegistry {
            schemas: HashMap::new(),
            version: 1,
        }
    }

    pub fn add_schema(&mut self, name: String, schema: Schema) -> SqlResult<()> {
        if self.schemas.contains_key(&name) {
            return Err(SqlError::schema_error(format!("Schema '{}' already exists", name)));
        }
        
        self.schemas.insert(name, schema);
        self.version += 1;
        Ok(())
    }

    pub fn remove_schema(&mut self, name: &str) -> SqlResult<Schema> {
        let schema = self.schemas.remove(name)
            .ok_or_else(|| SqlError::schema_error(format!("Schema '{}' not found", name)))?;
        
        self.version += 1;
        Ok(schema)
    }

    pub fn get_schema(&self, name: &str) -> Option<&Schema> {
        self.schemas.get(name)
    }

    pub fn schema_names(&self) -> Vec<String> {
        self.schemas.keys().cloned().collect()
    }

    pub fn index_names(&self) -> Vec<String> {
        self.schemas.values()
            .flat_map(|schema| schema.index_names())
            .collect()
    }

    pub fn validate_all_schemas(&self) -> SqlResult<()> {
        for schema in self.schemas.values() {
            schema.validate()?;
        }
        Ok(())
    }

    pub fn get_statistics(&self) -> SchemaStatistics {
        SchemaStatistics {
            table_count: self.schemas.len(),
            index_count: self.index_names().len(),
            constraint_count: 0,
            total_columns: 0,
            version: self.version,
            size_estimate: 0,
        }
    }

    pub fn clear(&mut self) {
        self.schemas.clear();
        self.version += 1;
    }
}

impl Default for SchemaRegistry {
    fn default() -> Self {
        Self::new()
    }
}
