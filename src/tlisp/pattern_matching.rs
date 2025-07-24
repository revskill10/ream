//! Pattern Matching for TLISP
//! 
//! Implements comprehensive pattern matching with destructuring and guards.

use std::collections::HashMap;
use crate::tlisp::{Value, Expr};
use crate::tlisp::types::Type;


/// Pattern for matching
#[derive(Debug, Clone, PartialEq)]
pub enum Pattern {
    /// Wildcard pattern (matches anything)
    Wildcard,
    /// Variable pattern (binds to variable)
    Variable(String),
    /// Literal pattern
    Literal(Value),
    /// Constructor pattern
    Constructor(String, Vec<Pattern>),
    /// List pattern
    List(Vec<Pattern>),
    /// Cons pattern (head::tail)
    Cons(Box<Pattern>, Box<Pattern>),
    /// Record pattern
    Record(String, HashMap<String, Pattern>),
    /// Guard pattern (pattern with condition)
    Guard(Box<Pattern>, Expr<Type>),
    /// Or pattern (alternative patterns)
    Or(Vec<Pattern>),
    /// As pattern (pattern with alias)
    As(Box<Pattern>, String),
}

/// Pattern match result
#[derive(Debug, Clone)]
pub struct MatchResult {
    /// Whether pattern matched
    pub matched: bool,
    /// Variable bindings from match
    pub bindings: HashMap<String, Value>,
}

/// Pattern compiler
pub struct PatternCompiler {
    /// Variable counter for generated names
    var_counter: usize,
}

/// Pattern matcher
pub struct PatternMatcher;

impl Pattern {
    /// Create a wildcard pattern
    pub fn wildcard() -> Self {
        Pattern::Wildcard
    }

    /// Create a variable pattern
    pub fn variable(name: String) -> Self {
        Pattern::Variable(name)
    }

    /// Create a literal pattern
    pub fn literal(value: Value) -> Self {
        Pattern::Literal(value)
    }

    /// Create a constructor pattern
    pub fn constructor(name: String, args: Vec<Pattern>) -> Self {
        Pattern::Constructor(name, args)
    }

    /// Create a list pattern
    pub fn list(patterns: Vec<Pattern>) -> Self {
        Pattern::List(patterns)
    }

    /// Create a cons pattern
    pub fn cons(head: Pattern, tail: Pattern) -> Self {
        Pattern::Cons(Box::new(head), Box::new(tail))
    }

    /// Create a guard pattern
    pub fn guard(pattern: Pattern, condition: Expr<Type>) -> Self {
        Pattern::Guard(Box::new(pattern), condition)
    }

    /// Create an or pattern
    pub fn or(patterns: Vec<Pattern>) -> Self {
        Pattern::Or(patterns)
    }

    /// Create an as pattern
    pub fn as_pattern(pattern: Pattern, alias: String) -> Self {
        Pattern::As(Box::new(pattern), alias)
    }

    /// Check if pattern is exhaustive for a type
    pub fn is_exhaustive(&self, _type: &Type) -> bool {
        match self {
            Pattern::Wildcard => true,
            Pattern::Variable(_) => true,
            Pattern::Or(patterns) => {
                // Check if any pattern is exhaustive
                patterns.iter().any(|p| p.is_exhaustive(_type))
            }
            _ => false, // Conservative: most patterns are not exhaustive
        }
    }

    /// Get all variables bound by this pattern
    pub fn bound_variables(&self) -> Vec<String> {
        let mut vars = Vec::new();
        self.collect_variables(&mut vars);
        vars
    }

    /// Collect variables recursively
    fn collect_variables(&self, vars: &mut Vec<String>) {
        match self {
            Pattern::Variable(name) => vars.push(name.clone()),
            Pattern::Constructor(_, args) => {
                for arg in args {
                    arg.collect_variables(vars);
                }
            }
            Pattern::List(patterns) => {
                for pattern in patterns {
                    pattern.collect_variables(vars);
                }
            }
            Pattern::Cons(head, tail) => {
                head.collect_variables(vars);
                tail.collect_variables(vars);
            }
            Pattern::Record(_, fields) => {
                for pattern in fields.values() {
                    pattern.collect_variables(vars);
                }
            }
            Pattern::Guard(pattern, _) => {
                pattern.collect_variables(vars);
            }
            Pattern::Or(patterns) => {
                for pattern in patterns {
                    pattern.collect_variables(vars);
                }
            }
            Pattern::As(pattern, alias) => {
                pattern.collect_variables(vars);
                vars.push(alias.clone());
            }
            _ => {}
        }
    }
}

impl MatchResult {
    /// Create a successful match
    pub fn success(bindings: HashMap<String, Value>) -> Self {
        MatchResult {
            matched: true,
            bindings,
        }
    }

    /// Create a failed match
    pub fn failure() -> Self {
        MatchResult {
            matched: false,
            bindings: HashMap::new(),
        }
    }

    /// Check if match was successful
    pub fn is_success(&self) -> bool {
        self.matched
    }

    /// Get bindings
    pub fn bindings(&self) -> &HashMap<String, Value> {
        &self.bindings
    }

    /// Merge with another match result
    pub fn merge(&self, other: &MatchResult) -> MatchResult {
        if !self.matched || !other.matched {
            return MatchResult::failure();
        }

        let mut bindings = self.bindings.clone();
        for (key, value) in &other.bindings {
            bindings.insert(key.clone(), value.clone());
        }

        MatchResult::success(bindings)
    }
}

impl PatternCompiler {
    /// Create a new pattern compiler
    pub fn new() -> Self {
        PatternCompiler { var_counter: 0 }
    }

    /// Generate a fresh variable name
    fn fresh_var(&mut self) -> String {
        let name = format!("_g{}", self.var_counter);
        self.var_counter += 1;
        name
    }

    /// Compile pattern to decision tree
    pub fn compile_pattern(&mut self, pattern: &Pattern) -> CompiledPattern {
        match pattern {
            Pattern::Wildcard => CompiledPattern::Wildcard,
            Pattern::Variable(name) => CompiledPattern::Variable(name.clone()),
            Pattern::Literal(value) => CompiledPattern::Literal(value.clone()),
            Pattern::Constructor(name, args) => {
                let compiled_args = args.iter()
                    .map(|arg| self.compile_pattern(arg))
                    .collect();
                CompiledPattern::Constructor(name.clone(), compiled_args)
            }
            Pattern::List(patterns) => {
                let compiled_patterns = patterns.iter()
                    .map(|p| self.compile_pattern(p))
                    .collect();
                CompiledPattern::List(compiled_patterns)
            }
            Pattern::Cons(head, tail) => {
                let compiled_head = Box::new(self.compile_pattern(head));
                let compiled_tail = Box::new(self.compile_pattern(tail));
                CompiledPattern::Cons(compiled_head, compiled_tail)
            }
            Pattern::Guard(pattern, condition) => {
                let compiled_pattern = Box::new(self.compile_pattern(pattern));
                CompiledPattern::Guard(compiled_pattern, condition.clone())
            }
            Pattern::Or(patterns) => {
                let compiled_patterns = patterns.iter()
                    .map(|p| self.compile_pattern(p))
                    .collect();
                CompiledPattern::Or(compiled_patterns)
            }
            Pattern::As(pattern, alias) => {
                let compiled_pattern = Box::new(self.compile_pattern(pattern));
                CompiledPattern::As(compiled_pattern, alias.clone())
            }
            Pattern::Record(name, fields) => {
                let compiled_fields = fields.iter()
                    .map(|(k, v)| (k.clone(), self.compile_pattern(v)))
                    .collect();
                CompiledPattern::Record(name.clone(), compiled_fields)
            }
        }
    }
}

/// Compiled pattern for efficient matching
#[derive(Debug, Clone)]
pub enum CompiledPattern {
    Wildcard,
    Variable(String),
    Literal(Value),
    Constructor(String, Vec<CompiledPattern>),
    List(Vec<CompiledPattern>),
    Cons(Box<CompiledPattern>, Box<CompiledPattern>),
    Record(String, HashMap<String, CompiledPattern>),
    Guard(Box<CompiledPattern>, Expr<Type>),
    Or(Vec<CompiledPattern>),
    As(Box<CompiledPattern>, String),
}

impl PatternMatcher {
    /// Match a value against a pattern
    pub fn match_pattern(pattern: &Pattern, value: &Value) -> MatchResult {
        Self::match_pattern_internal(pattern, value, &mut HashMap::new())
    }

    /// Internal pattern matching with accumulating bindings
    fn match_pattern_internal(
        pattern: &Pattern,
        value: &Value,
        bindings: &mut HashMap<String, Value>,
    ) -> MatchResult {
        match pattern {
            Pattern::Wildcard => MatchResult::success(bindings.clone()),
            
            Pattern::Variable(name) => {
                bindings.insert(name.clone(), value.clone());
                MatchResult::success(bindings.clone())
            }
            
            Pattern::Literal(literal) => {
                if literal == value {
                    MatchResult::success(bindings.clone())
                } else {
                    MatchResult::failure()
                }
            }
            
            Pattern::Constructor(name, args) => {
                // Match constructor patterns (for algebraic data types)
                // This would need to be implemented based on the value representation
                // For now, just check if it's a matching symbol
                match value {
                    Value::Symbol(sym) if sym == name && args.is_empty() => {
                        MatchResult::success(bindings.clone())
                    }
                    Value::List(values) if !values.is_empty() => {
                        if let Value::Symbol(sym) = &values[0] {
                            if sym == name && values.len() - 1 == args.len() {
                                // Match constructor arguments
                                for (i, arg_pattern) in args.iter().enumerate() {
                                    let result = Self::match_pattern_internal(
                                        arg_pattern,
                                        &values[i + 1],
                                        bindings,
                                    );
                                    if !result.matched {
                                        return MatchResult::failure();
                                    }
                                }
                                MatchResult::success(bindings.clone())
                            } else {
                                MatchResult::failure()
                            }
                        } else {
                            MatchResult::failure()
                        }
                    }
                    _ => MatchResult::failure(),
                }
            }
            
            Pattern::List(patterns) => {
                match value {
                    Value::List(values) => {
                        if patterns.len() != values.len() {
                            return MatchResult::failure();
                        }
                        
                        for (pattern, value) in patterns.iter().zip(values.iter()) {
                            let result = Self::match_pattern_internal(pattern, value, bindings);
                            if !result.matched {
                                return MatchResult::failure();
                            }
                        }
                        MatchResult::success(bindings.clone())
                    }
                    _ => MatchResult::failure(),
                }
            }
            
            Pattern::Cons(head_pattern, tail_pattern) => {
                match value {
                    Value::List(values) if !values.is_empty() => {
                        let head = &values[0];
                        let tail = Value::List(values[1..].to_vec());
                        
                        let head_result = Self::match_pattern_internal(head_pattern, head, bindings);
                        if !head_result.matched {
                            return MatchResult::failure();
                        }
                        
                        Self::match_pattern_internal(tail_pattern, &tail, bindings)
                    }
                    _ => MatchResult::failure(),
                }
            }
            
            Pattern::Guard(pattern, _condition) => {
                // First match the pattern
                let pattern_result = Self::match_pattern_internal(pattern, value, bindings);
                if !pattern_result.matched {
                    return MatchResult::failure();
                }
                
                // TODO: Evaluate guard condition
                // For now, just return the pattern match result
                pattern_result
            }
            
            Pattern::Or(patterns) => {
                // Try each pattern until one matches
                for pattern in patterns {
                    let mut temp_bindings = bindings.clone();
                    let result = Self::match_pattern_internal(pattern, value, &mut temp_bindings);
                    if result.matched {
                        *bindings = temp_bindings;
                        return MatchResult::success(bindings.clone());
                    }
                }
                MatchResult::failure()
            }
            
            Pattern::As(pattern, alias) => {
                // Match the pattern and bind the whole value to alias
                let result = Self::match_pattern_internal(pattern, value, bindings);
                if result.matched {
                    bindings.insert(alias.clone(), value.clone());
                    MatchResult::success(bindings.clone())
                } else {
                    MatchResult::failure()
                }
            }
            
            Pattern::Record(_, _fields) => {
                // TODO: Implement record pattern matching
                // This would require a record value type
                MatchResult::failure()
            }
        }
    }

    /// Check if patterns are exhaustive for a type
    pub fn check_exhaustiveness(patterns: &[Pattern], _type: &Type) -> bool {
        // Simple exhaustiveness check
        patterns.iter().any(|p| p.is_exhaustive(_type))
    }
}

impl Default for PatternCompiler {
    fn default() -> Self {
        Self::new()
    }
}
