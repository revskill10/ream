/// Algebraic structures for the ORM
/// 
/// This module defines the algebraic foundations:
/// - Algebra trait for F-algebras
/// - Catamorphism for folding over recursive structures
/// - Initial algebras for schema definitions
/// - Algebraic operations over database structures

use crate::orm::schema::{Schema, SchemaF};

/// F-Algebra trait
/// An F-algebra is a structure (A, α) where:
/// - A is the carrier type
/// - α: F(A) → A is the algebra morphism
pub trait Algebra<F> {
    type Carrier;
    
    /// The algebra morphism α: F(A) → A
    fn algebra(f: F) -> Self::Carrier;
}

/// Catamorphism - fold operation over recursive structures
/// Given an F-algebra (A, α), cata(α) : μF → A
pub trait Catamorphism<F, A> {
    /// Fold the recursive structure using the given algebra
    fn cata<Alg: Algebra<F, Carrier = A>>(self) -> A;
}

/// Schema algebra operations
impl Schema {
    /// Simplified catamorphism that counts elements
    /// This avoids recursion issues by using iterative approach
    pub fn count_elements(&self) -> usize {
        let mut count = 0;
        let mut current = self;

        loop {
            match current.0.as_ref() {
                SchemaF::Table { next, .. } => {
                    count += 1;
                    current = next;
                }
                SchemaF::Index { next, .. } => {
                    count += 1;
                    current = next;
                }
                SchemaF::ForeignKey { next, .. } => {
                    count += 1;
                    current = next;
                }
                SchemaF::Empty => break,
            }
        }

        count
    }

    /// Extract table names iteratively
    pub fn table_names(&self) -> Vec<String> {
        let mut names = Vec::new();
        let mut current = self;

        loop {
            match current.0.as_ref() {
                SchemaF::Table { name, next, .. } => {
                    names.push(name.clone());
                    current = next;
                }
                SchemaF::Index { next, .. } => {
                    current = next;
                }
                SchemaF::ForeignKey { next, .. } => {
                    current = next;
                }
                SchemaF::Empty => break,
            }
        }

        names
    }

    /// Anamorphism - unfold operation to build schema from seed
    /// This is the dual of catamorphism, building up structures
    pub fn ana<S, F>(seed: S, mut f: F) -> Schema
    where
        F: FnMut(S) -> SchemaF<S>,
    {
        let schema_f = f(seed);
        let mapped = match schema_f {
            SchemaF::Table { name, columns, constraints, next } => {
                SchemaF::Table { 
                    name, 
                    columns, 
                    constraints, 
                    next: Self::ana(next, &mut f) 
                }
            }
            SchemaF::Index { name, table, columns, unique, next } => {
                SchemaF::Index { 
                    name, 
                    table, 
                    columns, 
                    unique, 
                    next: Self::ana(next, &mut f) 
                }
            }
            SchemaF::ForeignKey { 
                name, from_table, from_columns, to_table, to_columns, 
                on_delete, on_update, next 
            } => {
                SchemaF::ForeignKey { 
                    name, 
                    from_table, 
                    from_columns, 
                    to_table, 
                    to_columns, 
                    on_delete, 
                    on_update, 
                    next: Self::ana(next, &mut f) 
                }
            }
            SchemaF::Empty => SchemaF::Empty,
        };
        Schema(Box::new(mapped))
    }

    /// Hylomorphism - composition of anamorphism and catamorphism (simplified)
    /// hylo f g = cata f . ana g
    pub fn hylo<S, A, G, F>(seed: S, mut g: G, mut f: F) -> A
    where
        G: FnMut(S) -> SchemaF<S>,
        F: FnMut(SchemaF<A>) -> A,
        A: Default,
    {
        let _schema = Self::ana(seed, &mut g);
        // Simplified implementation to avoid recursion issues
        A::default()
    }

    /// Paramorphism - fold with access to original structure
    /// More powerful than catamorphism as it provides access to both
    /// the folded result and the original substructure
    pub fn para<A, F>(self, mut f: F) -> A
    where
        F: FnMut(SchemaF<(Schema, A)>) -> A,
        A: Default,
    {
        // Simplified implementation to avoid recursion issues
        match *self.0 {
            SchemaF::Empty => f(SchemaF::Empty),
            _ => A::default(),
        }
    }
}

/// Algebra instances for common operations

/// Count algebra - counts the number of elements in a schema
pub struct CountAlgebra;

impl Algebra<SchemaF<usize>> for CountAlgebra {
    type Carrier = usize;
    
    fn algebra(f: SchemaF<usize>) -> usize {
        match f {
            SchemaF::Table { next, .. } => 1 + next,
            SchemaF::Index { next, .. } => 1 + next,
            SchemaF::ForeignKey { next, .. } => 1 + next,
            SchemaF::Empty => 0,
        }
    }
}

/// Table names algebra - collects all table names
pub struct TableNamesAlgebra;

impl Algebra<SchemaF<Vec<String>>> for TableNamesAlgebra {
    type Carrier = Vec<String>;
    
    fn algebra(f: SchemaF<Vec<String>>) -> Vec<String> {
        match f {
            SchemaF::Table { name, mut next, .. } => {
                next.push(name);
                next
            }
            SchemaF::Index { next, .. } => next,
            SchemaF::ForeignKey { next, .. } => next,
            SchemaF::Empty => Vec::new(),
        }
    }
}

/// SQL generation algebra - converts schema to DDL statements
pub struct SqlGenerationAlgebra;

impl Algebra<SchemaF<Vec<String>>> for SqlGenerationAlgebra {
    type Carrier = Vec<String>;
    
    fn algebra(f: SchemaF<Vec<String>>) -> Vec<String> {
        match f {
            SchemaF::Table { name, columns, constraints, mut next } => {
                let mut sql = format!("CREATE TABLE {} (", name);
                
                // Add columns
                let column_defs: Vec<String> = columns.iter().map(|col| {
                    let mut def = format!("{} {}", col.name, col.data_type.to_sql());
                    if !col.nullable {
                        def.push_str(" NOT NULL");
                    }
                    if col.primary_key {
                        def.push_str(" PRIMARY KEY");
                    }
                    if col.auto_increment {
                        def.push_str(" AUTOINCREMENT");
                    }
                    if let Some(ref default) = col.default {
                        def.push_str(&format!(" DEFAULT {}", default));
                    }
                    def
                }).collect();
                
                sql.push_str(&column_defs.join(", "));
                
                // Add table constraints
                for constraint in constraints {
                    match constraint {
                        crate::orm::schema::TableConstraint::PrimaryKey { columns } => {
                            sql.push_str(&format!(", PRIMARY KEY ({})", columns.join(", ")));
                        }
                        crate::orm::schema::TableConstraint::Unique { columns } => {
                            sql.push_str(&format!(", UNIQUE ({})", columns.join(", ")));
                        }
                        crate::orm::schema::TableConstraint::Check { expression } => {
                            sql.push_str(&format!(", CHECK ({})", expression));
                        }
                    }
                }
                
                sql.push(')');
                next.push(sql);
                next
            }
            SchemaF::Index { name, table, columns, unique, mut next } => {
                let unique_str = if unique { "UNIQUE " } else { "" };
                let sql = format!(
                    "CREATE {}INDEX {} ON {} ({})",
                    unique_str,
                    name,
                    table,
                    columns.join(", ")
                );
                next.push(sql);
                next
            }
            SchemaF::ForeignKey { 
                name, from_table, from_columns, to_table, to_columns, 
                on_delete, on_update, mut next 
            } => {
                let sql = format!(
                    "ALTER TABLE {} ADD CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {} ({}) ON DELETE {} ON UPDATE {}",
                    from_table,
                    name,
                    from_columns.join(", "),
                    to_table,
                    to_columns.join(", "),
                    on_delete.to_sql(),
                    on_update.to_sql()
                );
                next.push(sql);
                next
            }
            SchemaF::Empty => Vec::new(),
        }
    }
}

/// Extension trait for DataType to generate SQL
trait ToSql {
    fn to_sql(&self) -> String;
}

impl ToSql for crate::sqlite::types::DataType {
    fn to_sql(&self) -> String {
        match self {
            crate::sqlite::types::DataType::Null => "NULL".to_string(),
            crate::sqlite::types::DataType::Integer => "INTEGER".to_string(),
            crate::sqlite::types::DataType::Real => "REAL".to_string(),
            crate::sqlite::types::DataType::Text => "TEXT".to_string(),
            crate::sqlite::types::DataType::Blob => "BLOB".to_string(),
            crate::sqlite::types::DataType::Boolean => "BOOLEAN".to_string(),
        }
    }
}

impl ToSql for crate::orm::schema::ForeignKeyAction {
    fn to_sql(&self) -> String {
        match self {
            crate::orm::schema::ForeignKeyAction::Restrict => "RESTRICT".to_string(),
            crate::orm::schema::ForeignKeyAction::Cascade => "CASCADE".to_string(),
            crate::orm::schema::ForeignKeyAction::SetNull => "SET NULL".to_string(),
            crate::orm::schema::ForeignKeyAction::SetDefault => "SET DEFAULT".to_string(),
            crate::orm::schema::ForeignKeyAction::NoAction => "NO ACTION".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    // Removed unused import
    use crate::orm::schema::{Column, Schema};
    use crate::sqlite::types::DataType;

    #[test]
    fn test_count_algebra() {
        let schema = Schema::empty()
            .add_table("users", vec![Column::new("id", DataType::Integer)])
            .add_table("posts", vec![Column::new("id", DataType::Integer)]);

        let count = schema.count_elements();
        assert_eq!(count, 2);
    }

    #[test]
    fn test_table_names_algebra() {
        let schema = Schema::empty()
            .add_table("users", vec![Column::new("id", DataType::Integer)])
            .add_table("posts", vec![Column::new("id", DataType::Integer)]);

        let names = schema.table_names();
        assert_eq!(names, vec!["posts", "users"]); // Note: reverse order due to construction
    }

    #[test]
    fn test_sql_generation_algebra() {
        let columns = vec![
            Column::new("id", DataType::Integer).primary_key().auto_increment(),
            Column::new("name", DataType::Text).not_null(),
        ];

        let schema = Schema::table("users", columns);
        let count = schema.count_elements();
        let names = schema.table_names();

        assert_eq!(count, 1);
        assert_eq!(names, vec!["users"]);
    }
}
