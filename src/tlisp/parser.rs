//! TLISP parser and lexer

use std::fmt;
use crate::tlisp::{Expr, Value};
use crate::tlisp::types::{Type, TypeTerm, Kind};
use crate::error::{ParseError, TlispResult};

/// Token types for TLISP
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    /// Left parenthesis
    LeftParen,
    /// Right parenthesis
    RightParen,
    /// Symbol/identifier
    Symbol(String),
    /// Integer literal
    Number(i64),
    /// Float literal
    Float(f64),
    /// String literal
    String(String),
    /// Boolean literal
    Bool(bool),
    /// Quote
    Quote,
    /// End of input
    Eof,

    // NEW: Dependent type syntax tokens
    /// Colon for type annotations (:)
    Colon,
    /// Arrow for function types (->)
    Arrow,
    /// Left brace for refinement types ({)
    LeftBrace,
    /// Right brace for refinement types (})
    RightBrace,
    /// Pipe for refinement predicates (|)
    Pipe,
    /// Lambda keyword for type lambdas
    Lambda,
    /// Forall keyword for quantified types
    Forall,
    /// Refinement keyword
    Refinement,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Token::LeftParen => write!(f, "("),
            Token::RightParen => write!(f, ")"),
            Token::Symbol(s) => write!(f, "{}", s),
            Token::Number(n) => write!(f, "{}", n),
            Token::Float(fl) => write!(f, "{}", fl),
            Token::String(s) => write!(f, "\"{}\"", s),
            Token::Bool(b) => write!(f, "{}", b),
            Token::Quote => write!(f, "'"),
            Token::Eof => write!(f, "<EOF>"),
            Token::Colon => write!(f, ":"),
            Token::Arrow => write!(f, "->"),
            Token::LeftBrace => write!(f, "{{"),
            Token::RightBrace => write!(f, "}}"),
            Token::Pipe => write!(f, "|"),
            Token::Lambda => write!(f, "lambda"),
            Token::Forall => write!(f, "forall"),
            Token::Refinement => write!(f, "refinement"),
        }
    }
}

/// Lexer for tokenizing TLISP source code
pub struct Lexer {
    /// Input source
    input: Vec<char>,
    /// Current position
    position: usize,
    /// Current line number
    line: usize,
    /// Current column
    column: usize,
}

impl Lexer {
    /// Create a new lexer
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }
    
    /// Tokenize the input
    pub fn tokenize(&mut self) -> TlispResult<Vec<Token>> {
        let mut tokens = Vec::new();
        
        while !self.is_at_end() {
            self.skip_whitespace();
            
            if self.is_at_end() {
                break;
            }
            
            let token = self.next_token()?;
            tokens.push(token);
        }
        
        tokens.push(Token::Eof);
        Ok(tokens)
    }
    
    /// Get the next token
    fn next_token(&mut self) -> TlispResult<Token> {
        let ch = self.advance();

        match ch {
            // Handle whitespace characters
            ' ' | '\r' | '\t' => {
                // Skip this whitespace and get the next token
                self.next_token()
            }
            '\n' => {
                // Handle newline
                self.line += 1;
                self.column = 1;
                self.next_token()
            }
            '(' => Ok(Token::LeftParen),
            ')' => Ok(Token::RightParen),
            '\'' => Ok(Token::Quote),
            '"' => self.string_literal(),
            ':' => {
                // Check if this is part of a symbol (look back to see if we're continuing a symbol)
                // For now, treat standalone colons as Colon tokens
                // Module function names like "http-server:start" will be handled differently
                Ok(Token::Colon)
            }
            '{' => Ok(Token::LeftBrace),
            '}' => Ok(Token::RightBrace),
            '|' => Ok(Token::Pipe),
            '#' => self.hash_literal(),
            '-' if self.peek() == '>' => {
                self.advance(); // consume '>'
                Ok(Token::Arrow)
            }
            ';' => {
                // Comment - skip to end of line
                while self.peek() != '\n' && self.peek() != '\r' && !self.is_at_end() {
                    self.advance();
                }
                // Skip the line ending (handle both \r\n and \n)
                if self.peek() == '\r' {
                    self.advance(); // Skip \r
                }
                if self.peek() == '\n' {
                    self.line += 1;
                    self.column = 1;
                    self.advance(); // Skip \n
                }
                self.next_token()
            }
            _ if ch.is_ascii_digit() || (ch == '-' && self.peek().is_ascii_digit()) => {
                self.number_literal(ch)
            }
            _ if ch.is_alphabetic() || "+-*/<>=!?".contains(ch) => {
                self.symbol_or_keyword(ch)
            }
            _ => Err(ParseError::UnexpectedToken {
                position: self.position - 1,
                token: ch.to_string(),
            }.into()),
        }
    }
    
    /// Parse a string literal
    fn string_literal(&mut self) -> TlispResult<Token> {
        let mut value = String::new();
        
        while self.peek() != '"' && !self.is_at_end() {
            let ch = self.advance();
            if ch == '\\' {
                // Handle escape sequences
                let escaped = self.advance();
                match escaped {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    '\\' => value.push('\\'),
                    '"' => value.push('"'),
                    _ => {
                        value.push('\\');
                        value.push(escaped);
                    }
                }
            } else {
                value.push(ch);
            }
        }
        
        if self.is_at_end() {
            return Err(ParseError::UnterminatedList(self.position).into());
        }
        
        // Consume closing quote
        self.advance();
        
        Ok(Token::String(value))
    }
    
    /// Parse a number literal
    fn number_literal(&mut self, first_char: char) -> TlispResult<Token> {
        let mut value = String::new();
        value.push(first_char);
        
        while self.peek().is_ascii_digit() {
            value.push(self.advance());
        }
        
        // Check for float
        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            value.push(self.advance()); // consume '.'
            
            while self.peek().is_ascii_digit() {
                value.push(self.advance());
            }
            
            let float_val = value.parse::<f64>()
                .map_err(|_| ParseError::InvalidNumber(value.clone()))?;
            
            Ok(Token::Float(float_val))
        } else {
            let int_val = value.parse::<i64>()
                .map_err(|_| ParseError::InvalidNumber(value.clone()))?;
            
            Ok(Token::Number(int_val))
        }
    }
    
    /// Parse a hash literal (#t, #f, etc.)
    fn hash_literal(&mut self) -> TlispResult<Token> {
        let ch = self.advance();
        match ch {
            't' => Ok(Token::Bool(true)),
            'f' => Ok(Token::Bool(false)),
            _ => Err(ParseError::UnexpectedToken {
                position: self.position - 1,
                token: format!("#{}", ch),
            }.into()),
        }
    }

    /// Parse a symbol or keyword
    fn symbol_or_keyword(&mut self, first_char: char) -> TlispResult<Token> {
        let mut value = String::new();
        value.push(first_char);

        while !self.is_at_end() {
            let ch = self.peek();
            if ch.is_alphanumeric() || "+-*/<>=!?_-".contains(ch) {
                value.push(self.advance());
            } else if ch == ':' {
                // Allow colon in symbols for module function names like "http-server:start"
                value.push(self.advance());
            } else {
                break;
            }
        }

        // Check for boolean literals and keywords
        match value.as_str() {
            "true" => Ok(Token::Bool(true)),
            "false" => Ok(Token::Bool(false)),
            "lambda" => Ok(Token::Lambda),
            "forall" => Ok(Token::Forall),
            "refinement" => Ok(Token::Refinement),
            _ => Ok(Token::Symbol(value)),
        }
    }
    
    /// Skip whitespace characters
    fn skip_whitespace(&mut self) {
        while !self.is_at_end() {
            match self.peek() {
                ' ' | '\r' | '\t' => {
                    self.advance();
                }
                '\n' => {
                    self.line += 1;
                    self.column = 1;
                    self.advance();
                }
                _ => break,
            }
        }
    }
    
    /// Check if at end of input
    fn is_at_end(&self) -> bool {
        self.position >= self.input.len()
    }
    
    /// Advance to next character
    fn advance(&mut self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            let ch = self.input[self.position];
            self.position += 1;
            self.column += 1;
            ch
        }
    }
    
    /// Peek at current character
    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.input[self.position]
        }
    }
    
    /// Peek at next character
    fn peek_next(&self) -> char {
        if self.position + 1 >= self.input.len() {
            '\0'
        } else {
            self.input[self.position + 1]
        }
    }
}

/// Parser for TLISP expressions
pub struct Parser {
    /// Current token position
    current: usize,
    /// Tokens to parse
    tokens: Vec<Token>,
}

impl Parser {
    /// Create a new parser
    pub fn new() -> Self {
        Parser {
            current: 0,
            tokens: Vec::new(),
        }
    }
    
    /// Tokenize source code
    pub fn tokenize(&self, source: &str) -> TlispResult<Vec<Token>> {
        let mut lexer = Lexer::new(source);
        lexer.tokenize()
    }
    
    /// Parse tokens into an expression
    pub fn parse(&mut self, tokens: &[Token]) -> TlispResult<Expr<()>> {
        self.tokens = tokens.to_vec();
        self.current = 0;

        if self.is_at_end() {
            return Err(ParseError::UnexpectedEof.into());
        }

        self.expression()
    }

    /// Parse tokens into multiple expressions
    pub fn parse_multiple(&mut self, tokens: &[Token]) -> TlispResult<Vec<Expr<()>>> {
        self.tokens = tokens.to_vec();
        self.current = 0;

        let mut expressions = Vec::new();

        while !self.is_at_end() {
            expressions.push(self.expression()?);
        }

        if expressions.is_empty() {
            return Err(ParseError::UnexpectedEof.into());
        }

        Ok(expressions)
    }

    /// Parse type annotation from string
    pub fn parse_type_from_string(&mut self, source: &str) -> TlispResult<Type> {
        let tokens = self.tokenize(source)?;
        self.tokens = tokens;
        self.current = 0;

        if self.is_at_end() {
            return Err(ParseError::UnexpectedEof.into());
        }

        self.parse_type_annotation()
    }

    /// Parse dependent function type from string
    pub fn parse_dependent_function_from_string(&mut self, source: &str) -> TlispResult<Type> {
        let tokens = self.tokenize(source)?;
        self.tokens = tokens;
        self.current = 0;

        if self.is_at_end() {
            return Err(ParseError::UnexpectedEof.into());
        }

        self.parse_dependent_function_type()
    }
    
    /// Parse an expression
    fn expression(&mut self) -> TlispResult<Expr<()>> {
        match &self.peek() {
            Token::LeftParen => self.list_expression(),
            Token::Quote => self.quote_expression(),
            Token::Symbol(name) => {
                let name = name.clone();
                self.advance();
                Ok(Expr::Symbol(name, ()))
            }
            Token::Number(n) => {
                let n = *n;
                self.advance();
                Ok(Expr::Number(n, ()))
            }
            Token::Float(f) => {
                let f = *f;
                self.advance();
                Ok(Expr::Float(f, ()))
            }
            Token::Bool(b) => {
                let b = *b;
                self.advance();
                Ok(Expr::Bool(b, ()))
            }
            Token::String(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expr::String(s, ()))
            }
            Token::RightParen => {
                Err(ParseError::UnexpectedToken {
                    position: self.current,
                    token: ")".to_string(),
                }.into())
            }
            Token::Eof => Err(ParseError::UnexpectedEof.into()),

            // Lambda can be used in expressions as a symbol
            Token::Lambda => {
                self.advance();
                Ok(Expr::Symbol("lambda".to_string(), ()))
            }

            // Other type syntax tokens - not valid in expressions
            Token::Colon | Token::Arrow | Token::LeftBrace | Token::RightBrace |
            Token::Pipe | Token::Forall | Token::Refinement => {
                Err(ParseError::UnexpectedToken {
                    position: self.current,
                    token: format!("{}", self.peek()),
                }.into())
            }
        }
    }
    
    /// Parse a list expression
    fn list_expression(&mut self) -> TlispResult<Expr<()>> {
        self.advance(); // consume '('
        
        let mut elements = Vec::new();
        
        while !self.check(&Token::RightParen) && !self.is_at_end() {
            elements.push(self.expression()?);
        }
        
        if self.is_at_end() {
            return Err(ParseError::UnterminatedList(self.current).into());
        }
        
        self.advance(); // consume ')'
        
        // Check for special forms
        if !elements.is_empty() {
            if let Expr::Symbol(name, _) = &elements[0] {
                match name.as_str() {
                    "lambda" => return self.parse_lambda(elements),
                    "let" => return self.parse_let(elements),
                    "if" => return self.parse_if(elements),
                    "cond" => return self.parse_cond(elements),
                    "set!" => return self.parse_set(elements),
                    "quote" => return self.parse_quote_form(elements),
                    "define" => return self.parse_define(elements),
                    _ => {}
                }
            }
        }
        
        // Regular list or function application
        if elements.is_empty() {
            Ok(Expr::List(elements, ()))
        } else {
            // In Lisp, (f x y) is always a function application, not a list
            // Lists are created with quote or list function
            let func = Box::new(elements[0].clone());
            let args = elements[1..].to_vec();
            Ok(Expr::Application(func, args, ()))
        }
    }
    
    /// Parse a quote expression
    fn quote_expression(&mut self) -> TlispResult<Expr<()>> {
        self.advance(); // consume quote
        let expr = Box::new(self.expression()?);
        Ok(Expr::Quote(expr, ()))
    }
    
    /// Parse lambda expression
    fn parse_lambda(&self, elements: Vec<Expr<()>>) -> TlispResult<Expr<()>> {
        if elements.len() < 3 {
            return Err(ParseError::InvalidSymbol("lambda requires parameter list and body".to_string()).into());
        }
        
        // Extract parameter list
        let params = match &elements[1] {
            Expr::List(param_exprs, _) => {
                let mut params = Vec::new();
                for param in param_exprs {
                    if let Expr::Symbol(name, _) = param {
                        params.push(name.clone());
                    } else {
                        return Err(ParseError::InvalidSymbol("lambda parameters must be symbols".to_string()).into());
                    }
                }
                params
            }
            // Handle the case where a single parameter is parsed as an application
            Expr::Application(func, args, _) => {
                if args.is_empty() {
                    if let Expr::Symbol(name, _) = func.as_ref() {
                        vec![name.clone()]
                    } else {
                        return Err(ParseError::InvalidSymbol("lambda parameters must be symbols".to_string()).into());
                    }
                } else {
                    // Multiple parameters in application form
                    let mut params = Vec::new();
                    if let Expr::Symbol(name, _) = func.as_ref() {
                        params.push(name.clone());
                    } else {
                        return Err(ParseError::InvalidSymbol("lambda parameters must be symbols".to_string()).into());
                    }
                    for arg in args {
                        if let Expr::Symbol(name, _) = arg {
                            params.push(name.clone());
                        } else {
                            return Err(ParseError::InvalidSymbol("lambda parameters must be symbols".to_string()).into());
                        }
                    }
                    params
                }
            }
            // Handle single parameter without parentheses
            Expr::Symbol(name, _) => {
                vec![name.clone()]
            }
            _ => return Err(ParseError::InvalidSymbol("lambda requires parameter list".to_string()).into()),
        };
        
        // Body is the rest of the expressions
        let body = if elements.len() == 3 {
            Box::new(elements[2].clone())
        } else {
            // Multiple expressions - wrap in implicit begin
            Box::new(Expr::List(elements[2..].to_vec(), ()))
        };
        
        Ok(Expr::Lambda(params, body, ()))
    }

    /// Parse define expression
    fn parse_define(&self, elements: Vec<Expr<()>>) -> TlispResult<Expr<()>> {
        if elements.len() < 3 {
            return Err(ParseError::InvalidSymbol("define requires name and value".to_string()).into());
        }

        match &elements[1] {
            // Simple variable definition: (define name value)
            Expr::Symbol(name, _) => {
                if elements.len() != 3 {
                    return Err(ParseError::InvalidSymbol("define requires name and value".to_string()).into());
                }
                let value = Box::new(elements[2].clone());
                Ok(Expr::Define(name.clone(), value, ()))
            }
            // Function definition: (define (name args...) body...)
            Expr::List(_, _) | Expr::Application(_, _, _) => {
                let (name, params) = match &elements[1] {
                    Expr::List(func_spec, _) => {
                        if func_spec.is_empty() {
                            return Err(ParseError::InvalidSymbol("define function requires name".to_string()).into());
                        }

                        let name = match &func_spec[0] {
                            Expr::Symbol(name, _) => name.clone(),
                            _ => return Err(ParseError::InvalidSymbol("define function requires symbol name".to_string()).into()),
                        };

                        // Extract parameters (skip the function name)
                        let params: Vec<String> = func_spec[1..].iter()
                            .map(|param| match param {
                                Expr::Symbol(name, _) => Ok(name.clone()),
                                _ => Err(ParseError::InvalidSymbol("function parameters must be symbols".to_string())),
                            })
                            .collect::<Result<Vec<_>, _>>()?;

                        (name, params)
                    }
                    Expr::Application(func_box, args, _) => {
                        let name = match func_box.as_ref() {
                            Expr::Symbol(name, _) => name.clone(),
                            _ => return Err(ParseError::InvalidSymbol("define function requires symbol name".to_string()).into()),
                        };

                        // Extract parameters from args
                        let params: Vec<String> = args.iter()
                            .map(|param| match param {
                                Expr::Symbol(name, _) => Ok(name.clone()),
                                _ => Err(ParseError::InvalidSymbol("function parameters must be symbols".to_string())),
                            })
                            .collect::<Result<Vec<_>, _>>()?;

                        (name, params)
                    }
                    _ => return Err(ParseError::InvalidSymbol("define requires symbol name or function specification".to_string()).into()),
                };

                // Body is everything after the function specification
                let body_exprs: Vec<Expr<()>> = elements[2..].to_vec();

                // If there's only one body expression, use it directly
                // Otherwise, wrap in a begin expression (represented as an application)
                let body = if body_exprs.len() == 1 {
                    Box::new(body_exprs[0].clone())
                } else {
                    // Create a begin expression: (begin expr1 expr2 ...)
                    let begin_func = Box::new(Expr::Symbol("begin".to_string(), ()));
                    Box::new(Expr::Application(begin_func, body_exprs, ()))
                };

                // Create lambda expression: (lambda (params...) body)
                let lambda = Expr::Lambda(params, body, ());

                Ok(Expr::Define(name, Box::new(lambda), ()))
            }
            _ => Err(ParseError::InvalidSymbol("define requires symbol name or function specification".to_string()).into()),
        }
    }

    /// Parse let expression
    fn parse_let(&self, elements: Vec<Expr<()>>) -> TlispResult<Expr<()>> {
        if elements.len() < 3 {
            return Err(ParseError::InvalidSymbol("let requires bindings and body".to_string()).into());
        }
        
        // Extract bindings - handle both List and Application forms
        let binding_exprs = match &elements[1] {
            Expr::List(items, _) => items.clone(),
            Expr::Application(func, args, _) => {
                // Convert Application back to a list: [func, ...args]
                let mut items = vec![(**func).clone()];
                items.extend(args.clone());
                items
            }
            _ => return Err(ParseError::InvalidSymbol("let requires binding list".to_string()).into()),
        };

        let mut bindings = Vec::new();
        for binding in &binding_exprs {
            // Handle both List and Application forms for individual bindings
            let pair = match binding {
                Expr::List(items, _) => items.clone(),
                Expr::Application(func, args, _) => {
                    // Convert Application back to a list: [func, ...args]
                    let mut items = vec![(**func).clone()];
                    items.extend(args.clone());
                    items
                }
                _ => return Err(ParseError::InvalidSymbol("let bindings must be lists".to_string()).into()),
            };

            if pair.len() == 2 {
                if let Expr::Symbol(name, _) = &pair[0] {
                    bindings.push((name.clone(), pair[1].clone()));
                } else {
                    return Err(ParseError::InvalidSymbol("let binding name must be symbol".to_string()).into());
                }
            } else {
                return Err(ParseError::InvalidSymbol("let binding must be (name value)".to_string()).into());
            }
        }
        
        // Body is the rest of the expressions
        let body = if elements.len() == 3 {
            Box::new(elements[2].clone())
        } else {
            // Multiple expressions - wrap in implicit begin
            let begin_func = Box::new(Expr::Symbol("begin".to_string(), ()));
            let statements = elements[2..].to_vec();
            Box::new(Expr::Application(begin_func, statements, ()))
        };

        Ok(Expr::Let(bindings, body, ()))
    }
    
    /// Parse if expression
    fn parse_if(&self, elements: Vec<Expr<()>>) -> TlispResult<Expr<()>> {
        if elements.len() != 4 {
            return Err(ParseError::InvalidSymbol("if requires condition, then, and else".to_string()).into());
        }
        
        let condition = Box::new(elements[1].clone());
        let then_expr = Box::new(elements[2].clone());
        let else_expr = Box::new(elements[3].clone());
        
        Ok(Expr::If(condition, then_expr, else_expr, ()))
    }
    
    /// Parse quote form
    fn parse_quote_form(&self, elements: Vec<Expr<()>>) -> TlispResult<Expr<()>> {
        if elements.len() != 2 {
            return Err(ParseError::InvalidSymbol("quote requires one argument".to_string()).into());
        }

        let expr = Box::new(elements[1].clone());
        Ok(Expr::Quote(expr, ()))
    }

    // ===== TYPE PARSING METHODS =====

    /// Parse type annotation
    pub fn parse_type_annotation(&mut self) -> TlispResult<Type> {
        let token = self.peek().clone();
        match token {
            Token::Symbol(name) => {
                self.advance();
                match name.as_str() {
                    "Int" => Ok(Type::Int),
                    "Float" => Ok(Type::Float),
                    "Bool" => Ok(Type::Bool),
                    "String" => Ok(Type::String),
                    "Symbol" => Ok(Type::Symbol),
                    "Unit" => Ok(Type::Unit),
                    "Pid" => Ok(Type::Pid),
                    _ => Ok(Type::TypeVar(name)),
                }
            }

            Token::LeftParen => {
                self.advance(); // consume '('

                // Check if the first token is an arrow (function type)
                if self.check(&Token::Arrow) {
                    self.advance(); // consume '->'
                    self.parse_function_type()
                } else {
                    let first = self.parse_type_annotation()?;

                    match first {
                        Type::TypeVar(name) if name == "->" => {
                            // Function type: (-> T1 T2 ... Tn)
                            self.parse_function_type()
                        }

                    Type::TypeVar(name) if name == "lambda" => {
                        // Type lambda: (lambda (x) T)
                        self.parse_type_lambda()
                    }

                    Type::TypeVar(name) if name == "forall" => {
                        // Quantified type: (forall (x y) T)
                        self.parse_quantified_type()
                    }

                    Type::TypeVar(name) if name == "refinement" => {
                        // Refinement type: (refinement T (lambda (x) P))
                        self.parse_refinement_type()
                    }

                        constructor => {
                            // Type application: (F arg1 arg2 ...)
                            self.parse_type_application(constructor)
                        }
                    }
                }
            }

            _ => Err(ParseError::UnexpectedToken {
                position: self.current,
                token: format!("{}", self.peek()),
            }.into()),
        }
    }

    /// Parse function type
    fn parse_function_type(&mut self) -> TlispResult<Type> {
        let mut param_types = Vec::new();

        // Parse parameter types
        while !self.is_at_end() && !self.check(&Token::RightParen) {
            param_types.push(self.parse_type_annotation()?);
        }

        if param_types.is_empty() {
            return Err(ParseError::InvalidSymbol("Function type requires at least return type".to_string()).into());
        }

        // Last type is return type
        let return_type = param_types.pop().unwrap();

        self.expect_token(&Token::RightParen)?;

        Ok(Type::Function(param_types, Box::new(return_type)))
    }

    /// Parse type lambda: (lambda (x) T)
    fn parse_type_lambda(&mut self) -> TlispResult<Type> {
        self.expect_token(&Token::LeftParen)?;

        // Parse parameter list
        let mut params = Vec::new();
        while !self.is_at_end() && !self.check(&Token::RightParen) {
            match self.peek() {
                Token::Symbol(name) => {
                    let name = name.clone();
                    self.advance();
                    params.push(name);
                }
                _ => break,
            }
        }

        self.expect_token(&Token::RightParen)?;

        // Parse body
        let body = self.parse_type_annotation()?;

        self.expect_token(&Token::RightParen)?;

        // Create nested type lambda (right-associative)
        let result = params.into_iter().rev().fold(body, |acc, param| {
            Type::TypeLambda {
                param,
                param_kind: Kind::Type, // Default kind, will be inferred later
                body: Box::new(acc),
            }
        });

        Ok(result)
    }

    /// Parse quantified type (placeholder for now)
    fn parse_quantified_type(&mut self) -> TlispResult<Type> {
        // For now, just consume tokens until closing paren and return a type variable
        let mut depth = 1;
        while depth > 0 && !self.is_at_end() {
            match self.peek() {
                Token::LeftParen => depth += 1,
                Token::RightParen => depth -= 1,
                _ => {}
            }
            self.advance();
        }
        Ok(Type::TypeVar("Quantified".to_string()))
    }

    /// Parse refinement type: (refinement T (lambda (x) P))
    fn parse_refinement_type(&mut self) -> TlispResult<Type> {
        let base_type = self.parse_type_annotation()?;

        self.expect_token(&Token::LeftParen)?;

        // Expect lambda keyword
        match self.peek() {
            Token::Lambda => {
                self.advance();
            }
            _ => return Err(ParseError::UnexpectedToken {
                position: self.current,
                token: "Expected 'lambda' in refinement type".to_string(),
            }.into()),
        }

        self.expect_token(&Token::LeftParen)?;

        // Parse variable name
        let var = match self.peek() {
            Token::Symbol(name) => {
                let name = name.clone();
                self.advance();
                name
            }
            _ => return Err(ParseError::UnexpectedToken {
                position: self.current,
                token: "Expected variable name in refinement".to_string(),
            }.into()),
        };

        self.expect_token(&Token::RightParen)?;

        // Parse predicate
        let predicate = self.parse_type_term()?;

        self.expect_token(&Token::RightParen)?;
        self.expect_token(&Token::RightParen)?;

        Ok(Type::Refinement {
            var,
            base_type: Box::new(base_type),
            predicate: Box::new(predicate),
        })
    }

    /// Parse type application
    fn parse_type_application(&mut self, constructor: Type) -> TlispResult<Type> {
        let mut args = Vec::new();

        // For now, just parse basic type applications like List(Int)
        while !self.is_at_end() && !self.check(&Token::RightParen) {
            // Parse type term (simplified for now)
            match self.peek() {
                Token::Symbol(name) => {
                    let name = name.clone();
                    self.advance();
                    args.push(TypeTerm::Var(name));
                }
                Token::Number(n) => {
                    let n = *n;
                    self.advance();
                    args.push(TypeTerm::Literal(Value::Int(n)));
                }
                _ => break,
            }
        }

        self.expect_token(&Token::RightParen)?;

        Ok(Type::TypeApp {
            constructor: Box::new(constructor),
            args,
        })
    }

    /// Parse dependent function type: (x: T) -> U
    pub fn parse_dependent_function_type(&mut self) -> TlispResult<Type> {
        self.expect_token(&Token::LeftParen)?;

        // Parse parameter name and type
        let param_name = match self.peek() {
            Token::Symbol(name) => {
                let name = name.clone();
                self.advance();
                name
            }
            _ => return Err(ParseError::UnexpectedToken {
                position: self.current,
                token: "Expected parameter name".to_string(),
            }.into()),
        };

        self.expect_token(&Token::Colon)?;
        let param_type = self.parse_type_annotation()?;

        self.expect_token(&Token::RightParen)?;
        self.expect_token(&Token::Arrow)?;

        // Parse return type (may reference param_name)
        let return_type = self.parse_type_annotation()?;

        Ok(Type::DepFunction {
            param_name,
            param_type: Box::new(param_type),
            return_type: Box::new(return_type),
        })
    }

    /// Parse type term (for use in dependent types)
    pub fn parse_type_term(&mut self) -> TlispResult<TypeTerm> {
        let token = self.peek().clone();
        match token {
            Token::Symbol(name) => {
                self.advance();
                Ok(TypeTerm::Var(name))
            }

            Token::Number(n) => {
                self.advance();
                Ok(TypeTerm::Literal(Value::Int(n)))
            }

            Token::LeftParen => {
                self.advance();

                let func = self.parse_type_term()?;
                let mut args = Vec::new();

                while !self.is_at_end() && !self.check(&Token::RightParen) {
                    args.push(self.parse_type_term()?);
                }

                self.expect_token(&Token::RightParen)?;

                Ok(TypeTerm::App(Box::new(func), args))
            }

            _ => Err(ParseError::UnexpectedToken {
                position: self.current,
                token: format!("Expected type term, got {}", self.peek()),
            }.into()),
        }
    }

    /// Expect a specific token
    fn expect_token(&mut self, expected: &Token) -> TlispResult<()> {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(expected) {
            self.advance();
            Ok(())
        } else {
            Err(ParseError::UnexpectedToken {
                position: self.current,
                token: format!("Expected {:?}, got {:?}", expected, self.peek()),
            }.into())
        }
    }

    /// Parse cond expression
    fn parse_cond(&self, elements: Vec<Expr<()>>) -> TlispResult<Expr<()>> {
        if elements.len() < 2 {
            return Err(ParseError::InvalidSymbol("cond requires at least one clause".to_string()).into());
        }



        // Convert cond to nested if expressions
        let mut result = None;

        // Process clauses in reverse order
        for clause_expr in elements[1..].iter().rev() {

            // Extract clause items - handle both List and Application forms
            let clause_items = match clause_expr {
                Expr::List(items, _) => items.clone(),
                Expr::Application(func, args, _) => {
                    // Convert Application back to a list: [func, ...args]
                    let mut items = vec![(**func).clone()];
                    items.extend(args.clone());
                    items
                }
                _ => {
                    return Err(ParseError::InvalidSymbol("cond clause must be a list".to_string()).into());
                }
            };

            if clause_items.len() < 2 {
                return Err(ParseError::InvalidSymbol("cond clause must have condition and result".to_string()).into());
            }

            let condition = Box::new(clause_items[0].clone());

            // Handle multi-statement clauses by wrapping in begin
            let then_expr = if clause_items.len() == 2 {
                Box::new(clause_items[1].clone())
            } else {
                // Multiple statements - wrap in begin
                let begin_func = Box::new(Expr::Symbol("begin".to_string(), ()));
                let statements = clause_items[1..].to_vec();
                Box::new(Expr::Application(begin_func, statements, ()))
            };

            // Check for 'else' clause
            if let Expr::Symbol(name, _) = &clause_items[0] {
                if name == "else" {
                    result = Some(clause_items[1].clone());
                    continue;
                }
            }

            let else_expr = Box::new(result.unwrap_or(Expr::Symbol("nil".to_string(), ())));
            result = Some(Expr::If(condition, then_expr, else_expr, ()));
        }

        Ok(result.unwrap_or(Expr::Symbol("nil".to_string(), ())))
    }

    /// Parse set! expression
    fn parse_set(&self, elements: Vec<Expr<()>>) -> TlispResult<Expr<()>> {
        if elements.len() != 3 {
            return Err(ParseError::InvalidSymbol("set! requires variable and value".to_string()).into());
        }

        let name = match &elements[1] {
            Expr::Symbol(name, _) => name.clone(),
            _ => return Err(ParseError::InvalidSymbol("set! requires a symbol".to_string()).into()),
        };

        let value = Box::new(elements[2].clone());
        Ok(Expr::Set(name, value, ()))
    }
    
    /// Check if current token matches
    fn check(&self, token: &Token) -> bool {
        if self.is_at_end() {
            false
        } else {
            std::mem::discriminant(self.peek()) == std::mem::discriminant(token)
        }
    }
    
    /// Advance to next token
    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }
    
    /// Check if at end of tokens
    fn is_at_end(&self) -> bool {
        matches!(self.peek(), Token::Eof)
    }
    
    /// Peek at current token
    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }
    
    /// Get previous token
    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_basic() {
        let mut lexer = Lexer::new("(+ 1 2)");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens.len(), 6); // (, +, 1, 2, ), EOF
        assert_eq!(tokens[0], Token::LeftParen);
        assert_eq!(tokens[1], Token::Symbol("+".to_string()));
        assert_eq!(tokens[2], Token::Number(1));
        assert_eq!(tokens[3], Token::Number(2));
        assert_eq!(tokens[4], Token::RightParen);
        assert_eq!(tokens[5], Token::Eof);
    }
    
    #[test]
    fn test_lexer_string() {
        let mut lexer = Lexer::new("\"hello world\"");
        let tokens = lexer.tokenize().unwrap();
        
        assert_eq!(tokens[0], Token::String("hello world".to_string()));
    }
    
    #[test]
    fn test_parser_basic() {
        let mut parser = Parser::new();
        let tokens = parser.tokenize("42").unwrap();
        let expr = parser.parse(&tokens).unwrap();
        
        assert_eq!(expr, Expr::Number(42, ()));
    }
    
    #[test]
    fn test_parser_list() {
        let mut parser = Parser::new();
        let tokens = parser.tokenize("(+ 1 2)").unwrap();
        let expr = parser.parse(&tokens).unwrap();
        
        match expr {
            Expr::Application(func, args, _) => {
                assert_eq!(*func, Expr::Symbol("+".to_string(), ()));
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected application"),
        }
    }
    
    #[test]
    fn test_parser_lambda() {
        let mut parser = Parser::new();
        let tokens = parser.tokenize("(lambda (x) (+ x 1))").unwrap();
        let expr = parser.parse(&tokens).unwrap();

        match expr {
            Expr::Lambda(params, _body, _) => {
                assert_eq!(params, vec!["x".to_string()]);
            }
            _ => panic!("Expected lambda"),
        }
    }

    #[test]
    fn test_new_tokens() {
        let parser = Parser::new();
        let tokens = parser.tokenize(": -> { } |").unwrap();

        assert_eq!(tokens[0], Token::Colon);
        assert_eq!(tokens[1], Token::Arrow);
        assert_eq!(tokens[2], Token::LeftBrace);
        assert_eq!(tokens[3], Token::RightBrace);
        assert_eq!(tokens[4], Token::Pipe);
    }

    #[test]
    fn test_keyword_tokens() {
        let parser = Parser::new();
        let tokens = parser.tokenize("lambda forall refinement").unwrap();

        assert_eq!(tokens[0], Token::Lambda);
        assert_eq!(tokens[1], Token::Forall);
        assert_eq!(tokens[2], Token::Refinement);
    }

    #[test]
    fn test_basic_type_parsing() {
        let mut parser = Parser::new();

        // Test basic types
        assert_eq!(parser.parse_type_from_string("Int").unwrap(), Type::Int);
        assert_eq!(parser.parse_type_from_string("Bool").unwrap(), Type::Bool);
        assert_eq!(parser.parse_type_from_string("String").unwrap(), Type::String);
        assert_eq!(parser.parse_type_from_string("Float").unwrap(), Type::Float);
        assert_eq!(parser.parse_type_from_string("Unit").unwrap(), Type::Unit);
        assert_eq!(parser.parse_type_from_string("Pid").unwrap(), Type::Pid);
    }

    #[test]
    fn test_type_variable_parsing() {
        let mut parser = Parser::new();

        // Test type variables
        assert_eq!(parser.parse_type_from_string("T").unwrap(), Type::TypeVar("T".to_string()));
        assert_eq!(parser.parse_type_from_string("MyType").unwrap(), Type::TypeVar("MyType".to_string()));
        assert_eq!(parser.parse_type_from_string("a").unwrap(), Type::TypeVar("a".to_string()));
    }

    #[test]
    fn test_function_type_parsing() {
        let mut parser = Parser::new();

        // Test simple function type: (-> Int Bool)
        let func_type = parser.parse_type_from_string("(-> Int Bool)").unwrap();
        match func_type {
            Type::Function(params, ret) => {
                assert_eq!(params, vec![Type::Int]);
                assert_eq!(*ret, Type::Bool);
            }
            _ => panic!("Expected function type"),
        }

        // Test multi-parameter function: (-> Int String Bool)
        let func_type = parser.parse_type_from_string("(-> Int String Bool)").unwrap();
        match func_type {
            Type::Function(params, ret) => {
                assert_eq!(params, vec![Type::Int, Type::String]);
                assert_eq!(*ret, Type::Bool);
            }
            _ => panic!("Expected function type"),
        }
    }

    #[test]
    fn test_dependent_function_parsing() {
        let mut parser = Parser::new();

        // Test dependent function: (n: Int) -> String
        let dep_func = parser.parse_dependent_function_from_string("(n: Int) -> String").unwrap();
        match dep_func {
            Type::DepFunction { param_name, param_type, return_type } => {
                assert_eq!(param_name, "n");
                assert_eq!(*param_type, Type::Int);
                assert_eq!(*return_type, Type::String);
            }
            _ => panic!("Expected dependent function type"),
        }
    }

    #[test]
    fn test_type_application_parsing() {
        let mut parser = Parser::new();

        // Test type application: (List Int)
        let type_app = parser.parse_type_from_string("(List Int)").unwrap();
        match type_app {
            Type::TypeApp { constructor, args } => {
                assert_eq!(*constructor, Type::TypeVar("List".to_string()));
                assert_eq!(args.len(), 1);
                match &args[0] {
                    TypeTerm::Var(name) => assert_eq!(name, "Int"),
                    _ => panic!("Expected type variable"),
                }
            }
            _ => panic!("Expected type application"),
        }
    }
}
