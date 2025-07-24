//! Macro system for TLISP

use std::collections::HashMap;
use crate::tlisp::Expr;
use crate::error::{MacroError, TlispResult, TlispError};

/// Macro definition
#[derive(Debug, Clone)]
pub struct Macro {
    /// Macro name
    pub name: String,
    /// Parameter patterns
    pub patterns: Vec<String>,
    /// Macro body template
    pub body: Expr<()>,
    /// Hygiene information
    pub hygiene: HygieneInfo,
}

/// Hygiene information for macros
#[derive(Debug, Clone, Default)]
pub struct HygieneInfo {
    /// Captured variables
    pub captured: Vec<String>,
    /// Generated symbols
    pub generated: Vec<String>,
}

impl Macro {
    /// Create a new macro
    pub fn new(name: String, patterns: Vec<String>, body: Expr<()>) -> Self {
        Macro {
            name,
            patterns,
            body,
            hygiene: HygieneInfo::default(),
        }
    }
    
    /// Expand the macro with given arguments
    pub fn expand(&self, args: &[Expr<()>]) -> TlispResult<Expr<()>> {
        if args.len() != self.patterns.len() {
            return Err(MacroError::ArityMismatch {
                expected: self.patterns.len(),
                actual: args.len(),
            }.into());
        }
        
        // Create substitution map
        let mut substitutions = HashMap::new();
        for (pattern, arg) in self.patterns.iter().zip(args.iter()) {
            substitutions.insert(pattern.clone(), arg.clone());
        }
        
        // Apply substitutions to body
        self.substitute(&self.body, &substitutions)
    }
    
    /// Substitute patterns in expression
    fn substitute(&self, expr: &Expr<()>, substitutions: &HashMap<String, Expr<()>>) -> TlispResult<Expr<()>> {
        match expr {
            Expr::Symbol(name, _) => {
                if let Some(replacement) = substitutions.get(name) {
                    Ok(replacement.clone())
                } else {
                    Ok(expr.clone())
                }
            }
            Expr::List(items, _) => {
                let new_items: Result<Vec<Expr<()>>, _> = items.iter()
                    .map(|item| self.substitute(item, substitutions))
                    .collect();
                Ok(Expr::List(new_items?, ()))
            }
            Expr::Lambda(params, body, _) => {
                // Handle variable capture in lambda
                let new_body = Box::new(self.substitute(body, substitutions)?);
                Ok(Expr::Lambda(params.clone(), new_body, ()))
            }
            Expr::Application(func, args, _) => {
                let new_func = Box::new(self.substitute(func, substitutions)?);
                let new_args: Result<Vec<Expr<()>>, TlispError> = args.iter()
                    .map(|arg| self.substitute(arg, substitutions))
                    .collect();
                Ok(Expr::Application(new_func, new_args?, ()))
            }
            Expr::Let(bindings, body, _) => {
                let new_bindings: Result<Vec<(String, Expr<()>)>, TlispError> = bindings.iter()
                    .map(|(name, expr)| {
                        let new_expr = self.substitute(expr, substitutions)?;
                        Ok::<(String, Expr<()>), TlispError>((name.clone(), new_expr))
                    })
                    .collect();
                let new_body = Box::new(self.substitute(body, substitutions)?);
                Ok(Expr::Let(new_bindings?, new_body, ()))
            }
            Expr::If(cond, then_expr, else_expr, _) => {
                let new_cond = Box::new(self.substitute(cond, substitutions)?);
                let new_then = Box::new(self.substitute(then_expr, substitutions)?);
                let new_else = Box::new(self.substitute(else_expr, substitutions)?);
                Ok(Expr::If(new_cond, new_then, new_else, ()))
            }
            Expr::Quote(expr, _) => {
                let new_expr = Box::new(self.substitute(expr, substitutions)?);
                Ok(Expr::Quote(new_expr, ()))
            }
            _ => Ok(expr.clone()),
        }
    }
}

/// Macro registry for managing macros
pub struct MacroRegistry {
    /// Registered macros
    macros: HashMap<String, Macro>,
    /// Expansion depth limit
    max_depth: usize,
}

impl MacroRegistry {
    /// Create a new macro registry
    pub fn new() -> Self {
        let mut registry = MacroRegistry {
            macros: HashMap::new(),
            max_depth: 100,
        };
        
        // Add built-in macros
        registry.add_builtins();
        registry
    }
    
    /// Register a macro
    pub fn register(&mut self, macro_def: Macro) {
        self.macros.insert(macro_def.name.clone(), macro_def);
    }
    
    /// Expand macros in an expression
    pub fn expand(&self, expr: &Expr<()>) -> TlispResult<Expr<()>> {
        self.expand_with_depth(expr, 0)
    }
    
    /// Expand with depth tracking
    fn expand_with_depth(&self, expr: &Expr<()>, depth: usize) -> TlispResult<Expr<()>> {
        if depth >= self.max_depth {
            return Err(MacroError::RecursiveExpansion("Maximum expansion depth exceeded".to_string()).into());
        }
        
        match expr {
            Expr::Application(func, args, _) => {
                if let Expr::Symbol(name, _) = func.as_ref() {
                    if let Some(macro_def) = self.macros.get(name) {
                        // Expand macro
                        let expanded = macro_def.expand(args)?;
                        // Recursively expand the result
                        return self.expand_with_depth(&expanded, depth + 1);
                    }
                }
                
                // Not a macro, expand subexpressions
                let new_func = Box::new(self.expand_with_depth(func, depth)?);
                let new_args: Result<Vec<Expr<()>>, _> = args.iter()
                    .map(|arg| self.expand_with_depth(arg, depth))
                    .collect();
                Ok(Expr::Application(new_func, new_args?, ()))
            }
            Expr::List(items, _) => {
                let new_items: Result<Vec<Expr<()>>, _> = items.iter()
                    .map(|item| self.expand_with_depth(item, depth))
                    .collect();
                Ok(Expr::List(new_items?, ()))
            }
            Expr::Lambda(params, body, _) => {
                let new_body = Box::new(self.expand_with_depth(body, depth)?);
                Ok(Expr::Lambda(params.clone(), new_body, ()))
            }
            Expr::Let(bindings, body, _) => {
                let new_bindings: Result<Vec<(String, Expr<()>)>, _> = bindings.iter()
                    .map(|(name, expr)| {
                        let new_expr = self.expand_with_depth(expr, depth)?;
                        Ok::<(String, Expr<()>), TlispError>((name.clone(), new_expr))
                    })
                    .collect();
                let new_body = Box::new(self.expand_with_depth(body, depth)?);
                Ok(Expr::Let(new_bindings?, new_body, ()))
            }
            Expr::If(cond, then_expr, else_expr, _) => {
                let new_cond = Box::new(self.expand_with_depth(cond, depth)?);
                let new_then = Box::new(self.expand_with_depth(then_expr, depth)?);
                let new_else = Box::new(self.expand_with_depth(else_expr, depth)?);
                Ok(Expr::If(new_cond, new_then, new_else, ()))
            }
            _ => Ok(expr.clone()),
        }
    }
    
    /// Check if a symbol is a macro
    pub fn is_macro(&self, name: &str) -> bool {
        self.macros.contains_key(name)
    }
    
    /// Get macro by name
    pub fn get_macro(&self, name: &str) -> Option<&Macro> {
        self.macros.get(name)
    }
    
    /// Remove a macro
    pub fn remove(&mut self, name: &str) -> Option<Macro> {
        self.macros.remove(name)
    }
    
    /// Get all macro names
    pub fn macro_names(&self) -> Vec<String> {
        self.macros.keys().cloned().collect()
    }
    
    /// Set maximum expansion depth
    pub fn set_max_depth(&mut self, depth: usize) {
        self.max_depth = depth;
    }
    
    /// Add built-in macros
    fn add_builtins(&mut self) {
        // when macro: (when condition body) -> (if condition body null)
        let when_macro = Macro::new(
            "when".to_string(),
            vec!["condition".to_string(), "body".to_string()],
            Expr::If(
                Box::new(Expr::Symbol("condition".to_string(), ())),
                Box::new(Expr::Symbol("body".to_string(), ())),
                Box::new(Expr::Symbol("null".to_string(), ())),
                (),
            ),
        );
        self.register(when_macro);
        
        // unless macro: (unless condition body) -> (if condition null body)
        let unless_macro = Macro::new(
            "unless".to_string(),
            vec!["condition".to_string(), "body".to_string()],
            Expr::If(
                Box::new(Expr::Symbol("condition".to_string(), ())),
                Box::new(Expr::Symbol("null".to_string(), ())),
                Box::new(Expr::Symbol("body".to_string(), ())),
                (),
            ),
        );
        self.register(unless_macro);
        
        // and macro: (and a b) -> (if a b false)
        let and_macro = Macro::new(
            "and".to_string(),
            vec!["a".to_string(), "b".to_string()],
            Expr::If(
                Box::new(Expr::Symbol("a".to_string(), ())),
                Box::new(Expr::Symbol("b".to_string(), ())),
                Box::new(Expr::Bool(false, ())),
                (),
            ),
        );
        self.register(and_macro);
        
        // or macro: (or a b) -> (if a a b)
        let or_macro = Macro::new(
            "or".to_string(),
            vec!["a".to_string(), "b".to_string()],
            Expr::If(
                Box::new(Expr::Symbol("a".to_string(), ())),
                Box::new(Expr::Symbol("a".to_string(), ())),
                Box::new(Expr::Symbol("b".to_string(), ())),
                (),
            ),
        );
        self.register(or_macro);
    }
}

impl Default for MacroRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Macro builder for creating macros programmatically
pub struct MacroBuilder {
    name: String,
    patterns: Vec<String>,
    body: Option<Expr<()>>,
}

impl MacroBuilder {
    /// Create a new macro builder
    pub fn new(name: String) -> Self {
        MacroBuilder {
            name,
            patterns: Vec::new(),
            body: None,
        }
    }
    
    /// Add a parameter pattern
    pub fn param(mut self, pattern: String) -> Self {
        self.patterns.push(pattern);
        self
    }
    
    /// Add multiple parameter patterns
    pub fn params(mut self, patterns: Vec<String>) -> Self {
        self.patterns.extend(patterns);
        self
    }
    
    /// Set the macro body
    pub fn body(mut self, body: Expr<()>) -> Self {
        self.body = Some(body);
        self
    }
    
    /// Build the macro
    pub fn build(self) -> TlispResult<Macro> {
        let body = self.body.ok_or_else(|| {
            MacroError::InvalidDefinition("Macro body is required".to_string())
        })?;
        
        Ok(Macro::new(self.name, self.patterns, body))
    }
}

/// Syntax transformer for advanced macro features
pub struct SyntaxTransformer {
    /// Transformation rules
    rules: Vec<TransformRule>,
}

/// Transformation rule
#[derive(Debug, Clone)]
pub struct TransformRule {
    /// Pattern to match
    pub pattern: Expr<()>,
    /// Template to generate
    pub template: Expr<()>,
}

impl SyntaxTransformer {
    /// Create a new syntax transformer
    pub fn new() -> Self {
        SyntaxTransformer {
            rules: Vec::new(),
        }
    }
    
    /// Add a transformation rule
    pub fn add_rule(&mut self, pattern: Expr<()>, template: Expr<()>) {
        self.rules.push(TransformRule { pattern, template });
    }
    
    /// Apply transformations to an expression
    pub fn transform(&self, expr: &Expr<()>) -> TlispResult<Expr<()>> {
        for rule in &self.rules {
            if let Some(bindings) = self.match_pattern(&rule.pattern, expr) {
                return self.apply_template(&rule.template, &bindings);
            }
        }
        
        // No rule matched, return original expression
        Ok(expr.clone())
    }
    
    /// Match a pattern against an expression
    fn match_pattern(&self, pattern: &Expr<()>, expr: &Expr<()>) -> Option<HashMap<String, Expr<()>>> {
        let mut bindings = HashMap::new();
        
        if self.match_expr(pattern, expr, &mut bindings) {
            Some(bindings)
        } else {
            None
        }
    }
    
    /// Match expressions recursively
    fn match_expr(&self, pattern: &Expr<()>, expr: &Expr<()>, bindings: &mut HashMap<String, Expr<()>>) -> bool {
        match (pattern, expr) {
            (Expr::Symbol(name, _), _) if name.starts_with('?') => {
                // Pattern variable
                bindings.insert(name.clone(), expr.clone());
                true
            }
            (Expr::Symbol(a, _), Expr::Symbol(b, _)) => a == b,
            (Expr::Number(a, _), Expr::Number(b, _)) => a == b,
            (Expr::Bool(a, _), Expr::Bool(b, _)) => a == b,
            (Expr::String(a, _), Expr::String(b, _)) => a == b,
            (Expr::List(a_items, _), Expr::List(b_items, _)) => {
                if a_items.len() != b_items.len() {
                    return false;
                }
                
                for (a_item, b_item) in a_items.iter().zip(b_items.iter()) {
                    if !self.match_expr(a_item, b_item, bindings) {
                        return false;
                    }
                }
                
                true
            }
            _ => false,
        }
    }
    
    /// Apply template with bindings
    fn apply_template(&self, template: &Expr<()>, bindings: &HashMap<String, Expr<()>>) -> TlispResult<Expr<()>> {
        match template {
            Expr::Symbol(name, _) => {
                if let Some(replacement) = bindings.get(name) {
                    Ok(replacement.clone())
                } else {
                    Ok(template.clone())
                }
            }
            Expr::List(items, _) => {
                let new_items: Result<Vec<Expr<()>>, _> = items.iter()
                    .map(|item| self.apply_template(item, bindings))
                    .collect();
                Ok(Expr::List(new_items?, ()))
            }
            _ => Ok(template.clone()),
        }
    }
}

impl Default for SyntaxTransformer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macro_creation() {
        let macro_def = Macro::new(
            "test".to_string(),
            vec!["x".to_string()],
            Expr::Symbol("x".to_string(), ()),
        );
        
        assert_eq!(macro_def.name, "test");
        assert_eq!(macro_def.patterns.len(), 1);
    }
    
    #[test]
    fn test_macro_expansion() {
        let macro_def = Macro::new(
            "when".to_string(),
            vec!["condition".to_string(), "body".to_string()],
            Expr::If(
                Box::new(Expr::Symbol("condition".to_string(), ())),
                Box::new(Expr::Symbol("body".to_string(), ())),
                Box::new(Expr::Symbol("null".to_string(), ())),
                (),
            ),
        );
        
        let args = vec![
            Expr::Bool(true, ()),
            Expr::Number(42, ()),
        ];
        
        let expanded = macro_def.expand(&args).unwrap();
        
        match expanded {
            Expr::If(cond, then_expr, else_expr, _) => {
                assert_eq!(*cond, Expr::Bool(true, ()));
                assert_eq!(*then_expr, Expr::Number(42, ()));
                assert_eq!(*else_expr, Expr::Symbol("null".to_string(), ()));
            }
            _ => panic!("Expected if expression"),
        }
    }
    
    #[test]
    fn test_macro_registry() {
        let registry = MacroRegistry::new();
        
        // Should have built-in macros
        assert!(registry.is_macro("when"));
        assert!(registry.is_macro("unless"));
        assert!(registry.is_macro("and"));
        assert!(registry.is_macro("or"));
    }
    
    #[test]
    fn test_macro_builder() {
        let macro_def = MacroBuilder::new("test".to_string())
            .param("x".to_string())
            .body(Expr::Symbol("x".to_string(), ()))
            .build()
            .unwrap();
        
        assert_eq!(macro_def.name, "test");
        assert_eq!(macro_def.patterns, vec!["x".to_string()]);
    }
}
