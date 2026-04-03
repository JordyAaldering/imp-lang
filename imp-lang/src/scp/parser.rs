use std::iter::Peekable;

use crate::ast::*;

use super::{
    lexer::{Lexer, Token},
    operator::{self, Operator},
    span::Span,
};

pub struct Parser<'src> {
    lexer: Peekable<Lexer<'src>>,
    uid: usize,
}

#[derive(Debug)]
#[allow(unused)]
pub enum ParseError {
    NonAssociative,
    MissingReturn,
    UnexpectedToken(String, Token, Span),
    UnexpectedEof,
}

type ParseResult<T> = Result<T, ParseError>;

impl<'src> Parser<'src> {
    pub fn new(lexer: Lexer<'src>) -> Self {
        Self {
            lexer: lexer.peekable(),
            uid: 0,
        }
    }

    fn fresh_uid(&mut self) -> String {
        self.uid += 1;
        format!("_ret_{}", self.uid)
    }

    fn alloc_farg(&self, name: String, ty: Type) -> &'static Farg {
        Box::leak(Box::new(Farg { name, ty }))
    }

    fn alloc_lvis(&self, name: String, ty: Option<Type>) -> &'static VarInfo<'static, ParsedAst> {
        Box::leak(Box::new(VarInfo { name, ty, ssa: () }))
    }

    fn alloc_expr(&self, expr: Expr<'static, ParsedAst>) -> &'static Expr<'static, ParsedAst> {
        Box::leak(Box::new(expr))
    }

    fn matches(&mut self, expected: Token) -> Option<(Token, Span)> {
        self.lexer.next_if(|(token, _)| *token == expected)
    }

    fn expect(&mut self, expected: Token) -> ParseResult<(Token, Span)> {
        let (token, span) = self.next()?;
        if token == expected {
            Ok((token, span))
        } else {
            Err(ParseError::UnexpectedToken(
                format!("{:?}", expected),
                token,
                span,
            ))
        }
    }

    fn peek(&mut self) -> ParseResult<&(Token, Span)> {
        self.lexer.peek().ok_or(ParseError::UnexpectedEof)
    }

    fn next(&mut self) -> ParseResult<(Token, Span)> {
        self.lexer.next().ok_or(ParseError::UnexpectedEof)
    }
}

impl<'src> Parser<'src> {
    pub fn parse_program(&mut self) -> ParseResult<Program<'static, ParsedAst>> {
        let mut fundefs = Vec::new();

        while self.lexer.peek().is_some() {
            fundefs.push(self.parse_fundef()?);
        }

        Ok(Program { fundefs })
    }

    fn parse_fundef(&mut self) -> ParseResult<Fundef<'static, ParsedAst>> {
        let (_, _span_start) = self.expect(Token::Fn)?;

        let (name, _) = self.parse_id()?;

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

        while self.peek()?.0 != Token::RBrace {
            body.extend(self.parse_stmt()?);
        }

        self.expect(Token::RBrace)?;

        if !matches!(body.last(), Some(Stmt::Return(_))) {
            return Err(ParseError::MissingReturn);
        }

        Ok(Fundef {
            name,
            args,
            decs: Vec::new(),
            body,
            ret_type,
        })
    }

    fn parse_farg(&mut self) -> ParseResult<&'static Farg> {
        let (ty, _) = self.parse_type()?;
        let (id, _) = self.parse_id()?;
        Ok(self.alloc_farg(id, ty))
    }

    fn parse_stmt(&mut self) -> ParseResult<Vec<Stmt<'static, ParsedAst>>> {
        let (token, span) = self.next()?;

        let stmts = match token {
            Token::Identifier(lhs) => {
                self.expect(Token::Assign)?;
                let expr = self.parse_expr(None::<Bop>)?;
                let lvis = self.alloc_lvis(lhs, None);
                vec![Stmt::Assign(Assign { lvis, expr })]
            }
            Token::Return => {
                let expr = self.parse_expr(None::<Bop>)?;
                match expr {
                    Expr::Id(id) => vec![Stmt::Return(Return { id: id.clone() })],
                    _ => {
                        let ret_name = self.fresh_uid();
                        let ret_lvis = self.alloc_lvis(ret_name.clone(), None);
                        vec![
                            Stmt::Assign(Assign { lvis: ret_lvis, expr }),
                            Stmt::Return(Return {
                                id: Id::Var(ret_name),
                            }),
                        ]
                    }
                }
            }
            _ => {
                return Err(ParseError::UnexpectedToken(
                    "statement".to_owned(),
                    token,
                    span,
                ));
            }
        };

        self.expect(Token::Semicolon)?;

        Ok(stmts)
    }

    fn parse_expr(&mut self, prev_op: Option<impl Operator>) -> ParseResult<&'static Expr<'static, ParsedAst>> {
        if let Some((Token::LBrace, _)) = self.lexer.peek() {
            self.parse_tensor()
        } else if let Some((Token::LSquare, _)) = self.lexer.peek() {
            self.parse_array()
        } else {
            self.parse_binary(prev_op)
        }
    }

    fn parse_tensor(&mut self) -> ParseResult<&'static Expr<'static, ParsedAst>> {
        self.expect(Token::LBrace)?;

        let ret = self.parse_expr(None::<Bop>)?;

        self.expect(Token::Bar)?;

        let lb = self.parse_expr(None::<Bop>)?;

        self.expect(Token::Le)?;

        let (iv, _) = self.parse_id()?;

        self.expect(Token::Lt)?;

        let ub = self.parse_expr(None::<Bop>)?;

        self.expect(Token::RBrace)?;

        let iv = self.alloc_lvis(iv, None);
        Ok(self.alloc_expr(Expr::Tensor(Tensor {
            body: Vec::new(),
            ret,
            iv,
            lb,
            ub,
        })))
    }

    fn parse_array(&mut self) -> ParseResult<&'static Expr<'static, ParsedAst>> {
        self.expect(Token::LSquare)?;

        let mut values = Vec::new();

        if self.matches(Token::RSquare).is_none() {
            values.push(self.parse_expr(None::<Bop>)?);

            while self.matches(Token::Comma).is_some() {
                let v = self.parse_expr(None::<Bop>)?;
                values.push(v);
            }

            self.expect(Token::RSquare)?;
        }

        Ok(self.alloc_expr(Expr::Array(Array { values })))
    }

    fn parse_binary(&mut self, prev_op: Option<impl Operator>) -> ParseResult<&'static Expr<'static, ParsedAst>> {
        let (token, span_start) = self.next()?;

        let mut left = match token {
            Token::Identifier(id) => self.alloc_expr(Expr::Id(Id::Var(id))),
            Token::BoolValue(v) => self.alloc_expr(Expr::Bool(v)),
            Token::U32Value(v) => self.alloc_expr(Expr::U32(v)),
            Token::LParen => {
                let expr = self.parse_expr(None::<Bop>)?;

                let (token, rloc) = self.next()?;
                if token != Token::RParen {
                    return Err(ParseError::UnexpectedToken(
                        "expected ')'".to_owned(),
                        token,
                        rloc,
                    ));
                }

                expr
            }
            token => {
                let op = (&token).try_into().map_err(|_| {
                    ParseError::UnexpectedToken(
                        "expected unary expression".to_owned(),
                        token,
                        span_start,
                    )
                })?;
                self.parse_unary(op)?
            }
        };

        while let Some((op, _loc)) = self.parse_binary_operator(&prev_op)? {
            let right = self.parse_expr(Some(op))?;
            left = self.alloc_expr(Expr::Binary(Binary {
                l: left,
                r: right,
                op,
            }));
        }

        Ok(left)
    }

    fn parse_unary(&mut self, op: Uop) -> ParseResult<&'static Expr<'static, ParsedAst>> {
        let r = self.parse_expr(Some(op))?;
        Ok(self.alloc_expr(Expr::Unary(Unary { r, op })))
    }

    fn parse_binary_operator(&mut self, previous: &Option<impl Operator>) -> ParseResult<Option<(Bop, Span)>> {
        if let Some((token, _)) = self.lexer.peek() {
            if let Ok(op) = token.try_into() {
                if operator::precedes(&previous, &op)? {
                    let (_, span) = self.lexer.next().unwrap();
                    return Ok(Some((op, span)));
                }
            }
        }

        Ok(None)
    }

    fn parse_type(&mut self) -> ParseResult<(Type, Span)> {
        let (token, span) = self.next()?;
        let ty = match token {
            Token::U32Type => BaseType::U32,
            Token::BoolType => BaseType::Bool,
            _ => return Err(ParseError::UnexpectedToken("type".to_owned(), token, span)),
        };

        let ty = if self.matches(Token::LSquare).is_some() {
            let (id, _) = self.parse_id()?;

            self.expect(Token::RSquare)?;

            Type { ty, shp: Shape::Vector(id) }
        } else {
            Type { ty, shp: Shape::Scalar }
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
