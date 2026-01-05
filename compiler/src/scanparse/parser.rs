use std::iter::Peekable;

use crate::ast::{BaseType, Bop, Shape, Type, Uop};

use super::{lexer::{Lexer, Token}, operator::{self, Operator}, parse_ast::*, span::Span};

pub struct Parser<'src> {
    lexer: Peekable<Lexer<'src>>,
}

#[derive(Debug)]
#[allow(unused)]
pub enum ParseError {
    NonAssociative,
    MissingReturn,
    UnexpectedToken(String, Token, Span),
    UnexpectedEof,
}

// Alternatively: type ParseResult<T> = Result<(T, Span), ParseError>;
type ParseResult<T> = Result<T, ParseError>;

impl<'src> Parser<'src> {
    pub fn new(lexer: Lexer<'src>) -> Self {
        Self { lexer: lexer.peekable() }
    }

    /// ```bnf
    /// <program> := <fundef>*
    /// ```
    pub fn parse_program(&mut self) -> ParseResult<Program> {
        let mut fundefs = Vec::new();

        while self.lexer.peek().is_some() {
            fundefs.push(self.parse_fundef()?);
        }

        Ok(Program { fundefs } )
    }

    /// ```bnf
    /// <fundef> := "fn" <id> "(" [<farg> ("," <farg>)*]? ")" "->" <type>
    ///                 "{" <stmt>* <return> "}"
    /// ```
    fn parse_fundef(&mut self) -> ParseResult<Fundef> {
        let (_, _span_start) = self.expect(Token::Fn)?;

        let (id, _) = self.parse_id()?;
        println!("Parsing fundef {}", id);

        let mut args = Vec::new();

        self.expect(Token::LParen)?;
        if self.matches(Token::RParen).is_none() {
            args.push(self.parse_farg()?);

            while self.matches(Token::Comma).is_some() {
                args.push(self.parse_farg()?);
            }

            self.expect(Token::RParen)?;
        }

        self.expect(Token::Arrow)?;

        let (ret_type, _) = self.parse_type()?;

        self.expect(Token::LBrace)?;

        let mut body = Vec::new();
        let ret_expr;

        loop {
            match self.peek()?.0 {
                Token::RBrace => {
                    return Err(ParseError::MissingReturn);
                }
                Token::Return => {
                    ret_expr = self.parse_return()?;
                    break;
                },
                _ => {
                    body.push(self.parse_stmt()?);
                }
            }
        }

        self.expect(Token::RBrace)?;

        Ok(Fundef { id, args, ret_type, body, ret_expr })
    }

    /// ```bnf
    /// <farg> := <type> <id>
    /// ```
    fn parse_farg(&mut self) -> ParseResult<(Type, String)> {
        let (ty, _) = self.parse_type()?;

        let (id, _) = self.parse_id()?;

        Ok((ty, id))
    }

    /// ```bnf
    /// <stmt> := <assign>
    ///         | <return>
    ///
    /// <vardec> := <type> <id> "=" <expr> ";"
    ///           | <type> <id> ";"
    ///
    /// <assign> := <id> "=" <expr> ";"
    /// ```
    fn parse_stmt(&mut self) -> ParseResult<Stmt> {
        let (token, span) = self.next()?;

        let stmt = match token {
            Token::Identifier(lhs) => {
                self.expect(Token::Assign)?;
                let expr = self.parse_expr(None::<Bop>)?;
                Stmt::Assign { lhs, expr }
            },
            _ => return Err(ParseError::UnexpectedToken("statement".to_owned(), token, span)),
        };

        self.expect(Token::Semicolon)?;

        Ok(stmt)
    }

    /// ```bnf
    /// <return> := "return" <expr> ";"
    /// ```
    fn parse_return(&mut self) -> ParseResult<Expr> {
        self.expect(Token::Return)?;
        let expr = self.parse_expr(None::<Bop>)?;
        self.expect(Token::Semicolon)?;
        Ok(expr)
    }

    /// ```bnf
    /// <expr> := <tensor>
    ///         | <binary>
    ///         | <unary>
    ///         | <literal>
    ///         | "(" <expr> ")"
    /// ```
    fn parse_expr(&mut self, prev_op: Option<impl Operator>) -> ParseResult<Expr> {
        if let Some((Token::LBrace, _)) = self.lexer.peek() {
            self.parse_tensor()
        } else {
            self.parse_binary(prev_op)
        }
    }

    /// ```bnf
    /// <tensor> := "{" <expr> "|" <shp> "<=" <id> "<" <shp> "}"
    /// ```
    fn parse_tensor(&mut self) -> ParseResult<Expr> {
        self.expect(Token::LBrace)?;

        let expr = self.parse_expr(None::<Bop>)?;

        self.expect(Token::Bar)?;

        let lb = self.parse_expr(None::<Bop>)?;

        self.expect(Token::Le)?;

        let (iv, _) = self.parse_id()?;

        self.expect(Token::Lt)?;

        let ub = self.parse_expr(None::<Bop>)?;

        self.expect(Token::RBrace)?;

        Ok(Expr::Tensor { iv, expr: Box::new(expr), lb: Box::new(lb), ub: Box::new(ub) })
    }

    /// Uses Pratt parsing to handle associativity and operator precedence.
    ///
    /// ```bnf
    /// <binary> := <expr> <bop> <expr>
    ///
    /// <bop> := "+" | "-" | "*" | "/"
    ///        | "==" | "!=" | "<" | "<=" | ">" | ">="
    /// ```
    fn parse_binary(&mut self, prev_op: Option<impl Operator>) -> ParseResult<Expr> {
        let (token, span_start) = self.next()?;

        let mut left = match token {
            Token::Identifier(id) => Expr::Identifier(id),
            Token::BoolValue(v) => Expr::Bool(v),
            Token::U32Value(v) => Expr::U32(v),
            // Nested expression
            Token::LParen => {
                let expr = self.parse_expr(None::<Bop>)?;

                let (token, rloc) = self.next()?;
                if token != Token::RParen {
                    // Unbalanced parenthesis
                    return Err(ParseError::UnexpectedToken("expected ')'".to_owned(), token, rloc));
                }

                expr
            },
            // Parse unary expressions before trying to parse binary expressions
            token => {
                let op = (&token).try_into()
                    .map_err(|_| ParseError::UnexpectedToken("expected unary expression".to_owned(), token, span_start))?;
                self.parse_unary(op)?
            },
        };

        while let Some((op, _loc)) = self.parse_binary_operator(&prev_op)? {
            let right = self.parse_expr(Some(op))?;
            // Update `left`
            left = Expr::Binary {
                l: Box::new(left),
                r: Box::new(right),
                op,
            };
        }

        Ok(left)
    }

    /// ```bnf
    /// <unary> := <uop> <expr>
    ///
    /// <uop> := "!" | "-"
    /// ```
    fn parse_unary(&mut self, op: Uop) -> ParseResult<Expr> {
        let r = self.parse_expr(Some(op))?;
        Ok(Expr::Unary { r: Box::new(r), op })
    }

    fn parse_binary_operator(&mut self, previous: &Option<impl Operator>) -> ParseResult<Option<(Bop, Span)>> {
        if let Some((token, _)) = self.lexer.peek() {
            if let Ok(op) = token.try_into() {
                if operator::precedes(&previous, &op)? {
                    // Consume the token
                    let (_, span) = self.lexer.next().unwrap();
                    return Ok(Some((op, span)));
                }
            }
        }

        Ok(None)
    }

    /// ```bnf
    /// <type> := <basetype>
    ///         | <basetype> "[" "." "]"
    /// ```
    fn parse_type(&mut self) -> ParseResult<(Type, Span)> {
        let (token, span) = self.next()?;
        let basetype = match token {
            Token::U32Type => BaseType::U32,
            Token::BoolType => BaseType::Bool,
            _ => return Err(ParseError::UnexpectedToken("type".to_owned(), token, span)),
        };

        let ty = if self.matches(Token::LSquare).is_some() {
            let (id, _) = self.parse_id()?;

            self.expect(Token::RSquare)?;

            Type {
                basetype,
                shp: Shape::Vector(id)
            }
        } else {
            Type {
                basetype,
                shp: Shape::Scalar,
            }
        };
        Ok((ty, span))
    }

    fn parse_id(&mut self) -> ParseResult<(String, Span)> {
        let (token, span) = self.next()?;
        match token {
            Token::Identifier(id) => Ok((id, span)),
            _ => Err(ParseError::UnexpectedToken("identifier".to_owned(), token, span)),
        }
    }

    fn matches(&mut self, expected: Token) -> Option<(Token, Span)> {
        self.lexer.next_if(|(token, _)| *token == expected)
    }

    fn expect(&mut self, expected: Token) -> ParseResult<(Token, Span)> {
        let (token, span) = self.next()?;
        if token == expected {
            Ok((token, span))
        } else {
            Err(ParseError::UnexpectedToken(format!("{:?}", expected), token, span))
        }
    }

    fn peek(&mut self) -> ParseResult<&(Token, Span)> {
        self.lexer.peek()
            .ok_or(ParseError::UnexpectedEof)
    }

    fn next(&mut self) -> ParseResult<(Token, Span)> {
        self.lexer.next()
            .ok_or(ParseError::UnexpectedEof)
    }
}

impl TryInto<Bop> for &Token {
    type Error = ();

    fn try_into(self) -> Result<Bop, Self::Error> {
        match self {
            Token::Add => Ok(Bop::Add),
            Token::Sub => Ok(Bop::Sub),
            Token::Mul => Ok(Bop::Mul),
            Token::Div => Ok(Bop::Div),
            Token::Eq => Ok(Bop::Eq),
            Token::Ne => Ok(Bop::Ne),
            _ => Err(()),
        }
    }
}

impl TryInto<Uop> for &Token {
    type Error = ();

    fn try_into(self) -> Result<Uop, Self::Error> {
        match self {
            Token::Sub => Ok(Uop::Neg),
            Token::Not => Ok(Uop::Not),
            _ => Err(()),
        }
    }
}
