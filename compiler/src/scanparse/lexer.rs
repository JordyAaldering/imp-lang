use super::span::Span;

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    // Type names
    U32Type,
    BoolType,
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
    Comma,
    Colon,
    Semicolon,
    // Keywords
    Fn,
    Return,
    // Arithmetic operators
    Add, Sub, Mul, Div,
    // Comparison operators
    Not, Eq, Ne, Lt, Le, Gt, Ge,
    // Literals
    U32Value(u32),
    BoolValue(bool),
    Identifier(String),
    // Error
    Unexpected(char),
}

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
    // Should this return a tuple, or should we create a wrapper struct that contains the token and span?
    type Item = (Token, Span);

    fn next(&mut self) -> Option<Self::Item> {
        use Token::*;

        self.skip_whitespace();

        let start_idx = self.current;
        let start_col = self.col;

        // Keywords
        let token = if self.match_str("u32") {
            U32Type
        } else if self.match_str("bool") {
            BoolType
        } else if self.match_str("true") {
            BoolValue(true)
        } else if self.match_str("false") {
            BoolValue(false)
        } else if self.match_str("fn") {
            Fn
        } else if self.match_str("return") {
            Return
        } else {
            match self.next_char()? {
                // Symbols
                '{' => LBrace,
                '}' => RBrace,
                '(' => LParen,
                ')' => RParen,
                '[' => LSquare,
                ']' => RSquare,
                '|' => Bar,
                ',' => Comma,
                ':' => Colon,
                ';' => Semicolon,
                '-' if self.match_char('>') => Arrow,
                // Arithmetic operators
                '+' => Add,
                '-' => Sub,
                '*' => Mul,
                '/' => Div,
                // Comparison operators
                '=' if self.match_char('=') => Eq,
                '!' if self.match_char('=') => Ne,
                '<' if self.match_char('=') => Le,
                '<' => Lt,
                '>' if self.match_char('=') => Ge,
                '>' => Gt,
                '!' => Not,
                // Assignment
                '=' => Assign,
                // Literals
                c if c.is_ascii_digit() => {
                    while self.peek_char().is_some_and(|c| c.is_ascii_digit()) {
                        self.current += 1;
                        self.col += 1;
                    }

                    let end_idx = self.current;
                    U32Value(self.src[start_idx..end_idx].parse().unwrap())
                }
                c if c.is_ascii_alphabetic() => {
                    while self.peek_char().is_some_and(|c| c.is_ascii_alphanumeric()) {
                        self.current += 1;
                        self.col += 1;
                    }

                    let end_idx = self.current;
                    Identifier(self.src[start_idx..end_idx].to_string())
                }
                // Error
                c => Unexpected(c),
            }
        };

        let end_col = self.col;
        let span = Span::new(self.line, start_col, end_col);
        Some((token, span))
    }
}
