//! Bytecode registry for cross-language compilation

use std::collections::HashMap;
use crate::bytecode::{BytecodeProgram, LanguageCompiler};
use crate::error::{BytecodeError, BytecodeResult};

/// Universal bytecode registry for multiple languages
pub struct BytecodeRegistry {
    /// Registered language compilers
    compilers: HashMap<String, Box<dyn LanguageCompiler<AST = String>>>,
    /// Bytecode cache
    cache: HashMap<String, BytecodeProgram>,
    /// Language bridges for cross-compilation
    bridges: HashMap<String, Box<dyn LanguageBridge>>,
}

/// Language bridge for cross-compilation
pub trait LanguageBridge {
    /// Source language
    fn source_language(&self) -> &str;
    /// Target language
    fn target_language(&self) -> &str;
    /// Transform source to target
    fn transform(&self, source: &str) -> BytecodeResult<String>;
}

impl BytecodeRegistry {
    /// Create a new registry
    pub fn new() -> Self {
        BytecodeRegistry {
            compilers: HashMap::new(),
            cache: HashMap::new(),
            bridges: HashMap::new(),
        }
    }
    
    /// Register a language compiler
    pub fn register_compiler<C>(&mut self, language: String, compiler: C)
    where
        C: LanguageCompiler<AST = String> + 'static,
    {
        self.compilers.insert(language, Box::new(compiler));
    }
    
    /// Register a language bridge
    pub fn register_bridge<B>(&mut self, bridge: B)
    where
        B: LanguageBridge + 'static,
    {
        let key = format!("{}:{}", bridge.source_language(), bridge.target_language());
        self.bridges.insert(key, Box::new(bridge));
    }
    
    /// Compile source code to bytecode
    pub fn compile(&mut self, language: &str, source: &str) -> BytecodeResult<BytecodeProgram> {
        let cache_key = format!("{}:{}", language, source);
        
        // Check cache first
        if let Some(cached) = self.cache.get(&cache_key) {
            return Ok(cached.clone());
        }
        
        // Get compiler
        let compiler = self.compilers.get(language)
            .ok_or_else(|| BytecodeError::UnknownLanguage(language.to_string()))?;
        
        // Compile
        let program = compiler.compile_to_bytecode(source.to_string())?;
        
        // Cache result
        self.cache.insert(cache_key, program.clone());
        
        Ok(program)
    }
    
    /// Cross-compile from one language to another
    pub fn cross_compile(&mut self, from_lang: &str, to_lang: &str, source: &str) -> BytecodeResult<BytecodeProgram> {
        let bridge_key = format!("{}:{}", from_lang, to_lang);
        
        // Get bridge
        let bridge = self.bridges.get(&bridge_key)
            .ok_or_else(|| BytecodeError::NoBridge(from_lang.to_string()))?;
        
        // Transform source
        let transformed = bridge.transform(source)?;
        
        // Compile with target language
        self.compile(to_lang, &transformed)
    }
    
    /// Get supported languages
    pub fn supported_languages(&self) -> Vec<String> {
        self.compilers.keys().cloned().collect()
    }
    
    /// Clear cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
    
    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.cache.len(), self.compilers.len())
    }
}

impl Default for BytecodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple string-based compiler for testing
pub struct SimpleCompiler {
    language: String,
}

impl SimpleCompiler {
    pub fn new(language: String) -> Self {
        SimpleCompiler { language }
    }
}

impl LanguageCompiler for SimpleCompiler {
    type AST = String;
    
    fn compile_to_bytecode(&self, ast: Self::AST) -> BytecodeResult<BytecodeProgram> {
        let mut program = BytecodeProgram::new(format!("{}_program", self.language));
        
        // Very simple compilation - just parse numbers and create constants
        for line in ast.lines() {
            if let Ok(num) = line.trim().parse::<i64>() {
                let const_id = program.add_constant(crate::bytecode::Value::Int(num));
                program.add_instruction(crate::bytecode::Bytecode::Const(const_id, crate::types::EffectGrade::Pure));
            }
        }
        
        // Add return
        program.add_instruction(crate::bytecode::Bytecode::Ret(crate::types::EffectGrade::Pure));
        
        Ok(program)
    }
    
    fn get_type_info(&self, _ast: &Self::AST) -> crate::bytecode::TypeInfo {
        crate::bytecode::TypeInfo::Int
    }
}

/// Simple language bridge for testing
pub struct SimpleBridge {
    from: String,
    to: String,
}

impl SimpleBridge {
    pub fn new(from: String, to: String) -> Self {
        SimpleBridge { from, to }
    }
}

impl LanguageBridge for SimpleBridge {
    fn source_language(&self) -> &str {
        &self.from
    }
    
    fn target_language(&self) -> &str {
        &self.to
    }
    
    fn transform(&self, source: &str) -> BytecodeResult<String> {
        // Simple transformation - just pass through
        Ok(source.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_basic() {
        let mut registry = BytecodeRegistry::new();
        
        // Register a simple compiler
        registry.register_compiler("simple".to_string(), SimpleCompiler::new("simple".to_string()));
        
        // Compile some code
        let program = registry.compile("simple", "42\n24").unwrap();
        
        assert_eq!(program.constants.len(), 2);
        assert!(program.instructions.len() > 0);
    }
    
    #[test]
    fn test_cross_compilation() {
        let mut registry = BytecodeRegistry::new();
        
        // Register compilers
        registry.register_compiler("lang1".to_string(), SimpleCompiler::new("lang1".to_string()));
        registry.register_compiler("lang2".to_string(), SimpleCompiler::new("lang2".to_string()));
        
        // Register bridge
        registry.register_bridge(SimpleBridge::new("lang1".to_string(), "lang2".to_string()));
        
        // Cross-compile
        let program = registry.cross_compile("lang1", "lang2", "42").unwrap();
        
        assert!(program.constants.len() > 0);
    }
    
    #[test]
    fn test_caching() {
        let mut registry = BytecodeRegistry::new();
        registry.register_compiler("simple".to_string(), SimpleCompiler::new("simple".to_string()));
        
        // First compilation
        let _program1 = registry.compile("simple", "42").unwrap();
        
        // Second compilation (should use cache)
        let _program2 = registry.compile("simple", "42").unwrap();
        
        let (cache_size, _) = registry.cache_stats();
        assert_eq!(cache_size, 1);
    }
}
