use super::span::Span;

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    // Type names
    BoolType,
    I32Type,
    I64Type,
    U32Type,
    U64Type,
    UsizeType,
    F32Type,
    F64Type,
    // Symbols
    Arrow,
    Assign,
    LBrace,
    RBrace,
    LParen,
    RParen,
    LSquare,
    RSquare,
    Bar,
    Dot,
    Comma,
    Colon,
    Semicolon,
    // Keywords
    Fn,
    If, Else,
    // Operators
    Add, Sub, Mul, Div,
    Lt, Le, Gt, Ge,
    Eq, Ne, Not,
    // Literals
    BoolValue(bool),
    NatValue(i32),
    I32Value(i32),
    I64Value(i64),
    U32Value(u32),
    U64Value(u64),
    UsizeValue(usize),
    RealValue(f32),
    F32Value(f32),
    F64Value(f64),
    /// `@` is used as a prefix for primitive function calls
    Prf(String),
    Identifier(String),
    /// Error: natural number specifier on a real numbered value
    /// Example: `42.0i32`, `3.14usize`
    NotANaturalNumber(String),
    /// Error: unexpected token during lexing
    UnexpectedCharacter(char),
}

#[derive(Clone)]
pub struct Lexer<'src> {
    /// The input program as a string.
    src: &'src str,
    /// Index of the current character in the source string.
    current: usize,
    /// Line number of the current character.
    line: usize,
    /// Column number of the current character.
    col: usize,
}

impl<'src> Lexer<'src> {
    pub fn new(source: &'src str) -> Self {
        Self { src: source, current: 0, line: 1, col: 1 }
    }

    /// Get the next character without consuming it.
    fn peek_char(&self) -> Option<char> {
        self.src.chars().nth(self.current)
    }

    /// Get the next character without consuming it.
    fn peek_next_char(&self) -> Option<char> {
        self.src.chars().nth(self.current + 1)
    }

    /// Get the next character and consume it.
    fn next_char(&mut self) -> Option<char> {
        if let Some(c) = self.src.chars().nth(self.current) {
            self.current += 1;
            self.col += 1;
            Some(c)
        } else {
            None
        }
    }

    /// Check whether the next character is equal to the expected character.
    /// Consumes the next character iff it matches.
    fn match_char(&mut self, expected: char) -> bool {
        if self.peek_char().is_some_and(|c| c == expected) {
            self.current += 1;
            self.col += 1;
            true
        } else {
            false
        }
    }

    /// Check whether the next character is equal to the expected string.
    /// Consumes that many characters iff it matches.
    fn match_str(&mut self, expected: &str) -> bool {
        if self.src[self.current..].starts_with(expected) {
            self.current += expected.len();
            self.col += expected.len();
            true
        } else {
            false
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek_char() {
            match c {
                // Whitespace
                ' ' | '\t' | '\r' => {
                    self.current += 1;
                    self.col += 1;
                },
                // Single-line comment
                '/' if self.peek_next_char() == Some('/') => {
                    self.current += 2;
                    self.col += 2;
                    while self.peek_char() != Some('\n') && self.peek_char().is_some() {
                        self.current += 1;
                        self.col += 1;
                    }
                },
                // Newline
                '\n' => {
                    self.current += 1;
                    self.line += 1;
                    self.col = 1;
                }
                // Done
                _ => break,
            }
        }
    }
}

impl<'source> Iterator for Lexer<'source> {
    type Item = (Token, Span);

    fn next(&mut self) -> Option<Self::Item> {
        use Token::*;

        self.skip_whitespace();

        let start_idx = self.current;
        let start_col = self.col;

        // Keywords
        let token = if self.match_str("true") {
            BoolValue(true)
        } else if self.match_str("false") {
            BoolValue(false)
        } else if self.match_str("bool") {
            BoolType
        } else if self.match_str("i32") {
            I32Type
        } else if self.match_str("i64") {
            I64Type
        } else if self.match_str("u32") {
            U32Type
        } else if self.match_str("u64") {
            U64Type
        } else if self.match_str("usize") {
            UsizeType
        } else if self.match_str("f32") {
            F32Type
        } else if self.match_str("f64") {
            F64Type
        } else if self.match_str("fn") {
            Fn
        } else if self.match_str("if") {
            If
        } else if self.match_str("else") {
            Else
        } else {
            match self.next_char()? {
                // Symbols
                '-' if self.match_char('>') => Arrow,
                '{' => LBrace,
                '}' => RBrace,
                '(' => LParen,
                ')' => RParen,
                '[' => LSquare,
                ']' => RSquare,
                '|' => Bar,
                '.' => Dot,
                ',' => Comma,
                ':' => Colon,
                ';' => Semicolon,
                // Operators
                '+' => Add,
                '-' => Sub,
                '*' => Mul,
                '/' => Div,
                '<' if self.match_char('=') => Le,
                '<' => Lt,
                '>' if self.match_char('=') => Ge,
                '>' => Gt,
                '=' if self.match_char('=') => Eq,
                '!' if self.match_char('=') => Ne,
                '!' => Not,
                '=' => Assign,
                // Primitive function call
                '@' => {
                    while self.peek_char().is_some_and(|c| c.is_ascii_alphanumeric() || c == '_') {
                        self.current += 1;
                        self.col += 1;
                    }

                    let end_idx = self.current;
                    Prf(self.src[start_idx + 1..end_idx].to_string())
                },
                // Literals
                c if c.is_ascii_digit() => {
                    while self.peek_char().is_some_and(|c| c.is_ascii_digit()) {
                        self.current += 1;
                        self.col += 1;
                    }

                    let mut is_real = false;
                    if self.match_char('.') {
                        while self.peek_char().is_some_and(|c| c.is_ascii_digit()) {
                            self.current += 1;
                            self.col += 1;
                        }
                        is_real = true
                    };

                    let end_idx = self.current;
                    let s = &self.src[start_idx..end_idx];

                    if self.match_str("i32") {
                        if is_real {
                            NotANaturalNumber(s.to_string())
                        } else {
                            I32Value(s.parse().unwrap())
                        }
                    } else if self.match_str("i64") {
                        if is_real {
                            NotANaturalNumber(s.to_string())
                        } else {
                            I64Value(s.parse().unwrap())
                        }
                    } else if self.match_str("u32") {
                        if is_real {
                            NotANaturalNumber(s.to_string())
                        } else {
                            U32Value(s.parse().unwrap())
                        }
                    } else if self.match_str("u64") {
                        if is_real {
                            NotANaturalNumber(s.to_string())
                        } else {
                            U64Value(s.parse().unwrap())
                        }
                    } else if self.match_str("usize") {
                        if is_real {
                            NotANaturalNumber(s.to_string())
                        } else {
                            UsizeValue(s.parse().unwrap())
                        }
                    } else if self.match_str("f32") {
                        F32Value(s.parse().unwrap())
                    } else if self.match_str("f64") {
                        F64Value(s.parse().unwrap())
                    } else {
                        if is_real {
                            RealValue(s.parse().unwrap())
                        } else {
                            NatValue(s.parse().unwrap())
                        }
                    }
                }
                c if c.is_ascii_alphabetic() || c == '_' => {
                    while self.peek_char().is_some_and(|c| c.is_ascii_alphanumeric() || c == '_') {
                        self.current += 1;
                        self.col += 1;
                    }

                    let end_idx = self.current;
                    Identifier(self.src[start_idx..end_idx].to_string())
                }
                // Error
                c => UnexpectedCharacter(c),
            }
        };

        let end_col = self.col;
        let span = Span::new(self.line, start_col, end_col);
        Some((token, span))
    }
}
