use std::{collections::HashMap, iter::Peekable};

use super::{lexer::*, operator::*, span::*};

use crate::ast::*;

pub struct Parser<'src> {
    lexer: Peekable<Lexer<'src>>,
}

#[derive(Debug)]
#[allow(unused)]
pub enum ParseError {
    NonAssociative,
    DuplicateFunctionSignature(String),
    UnknownPrimitive(String, Span),
    FoldSelectionMustBeTensor,
    ExpectedStatement(Token, Span),
    UnexpectedToken(String, Token, Span),
    UnexpectedEof,
}

type ParseResult<T> = Result<T, ParseError>;

impl<'src> Parser<'src> {
    pub fn new(lexer: Lexer<'src>) -> Self {
        Self {
            lexer: lexer.peekable(),
        }
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
        let mut overloads = HashMap::new();

        while let Some((token, _)) = self.lexer.peek() {
            match token {
                Token::Fn => {
                    let fundef = self.parse_fundef()?;
                    let group = overloads.entry(fundef.name.clone()).or_insert(HashMap::new());
                    let fundefs = group.entry(fundef.signature()).or_insert(Vec::new());
                    fundefs.push(fundef);
                }
                _ => {
                    let (token, span) = self.next()?;
                    return Err(ParseError::UnexpectedToken("top-level item".to_owned(), token, span));
                }
            }
        }

        Ok(Program { overloads })
    }

    fn parse_fundef(&mut self) -> ParseResult<Fundef<'static, ParsedAst>> {
        let _ = self.expect(Token::Fn)?;
        let name = self.parse_callable_name()?;

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
        let body = self.parse_body()?;
        self.expect(Token::RBrace)?;

        Ok(Fundef {
            name,
            args,
            shape_prelude: Vec::new(),
            shape_facts: ShapeFacts::default(),
            decs: Vec::new(),
            body,
            ret_type,
        })
    }

    fn parse_farg(&mut self) -> ParseResult<Farg> {
        let (ty, _) = self.parse_type()?;
        let (id, _) = self.parse_id()?;
        Ok(Farg { id, ty })
    }

    /// ```bnf
    /// <body> = <stmt>* <expr>
    /// ```
    fn parse_body(&mut self) -> ParseResult<Body<'static, ParsedAst>> {
        let mut stmts = Vec::new();

        loop {
            let checkpoint = self.lexer.clone();
            match self.parse_stmt() {
                Ok(stmt) => stmts.push(stmt),
                Err(ParseError::ExpectedStatement(_, _)) => {
                    self.lexer = checkpoint;
                    break;
                }
                Err(e) => return Err(e),
            }
        }

        let ret = self.parse_expr(None::<Bop>)?;
        Ok(Body { stmts, ret })
    }

    fn parse_stmt(&mut self) -> ParseResult<Stmt<'static, ParsedAst>> {
        let (token, span) = self.next()?;
        let stmts = match token {
            Token::Identifier(lhs) => {
                // A bit ugly, but good enough for now
                let (err_token, err_loc) = self.peek()?.clone();
                self.expect(Token::Assign)
                    .map_err(|_| ParseError::ExpectedStatement(err_token, err_loc))?;

                let expr = self.parse_expr(None::<Bop>)?;
                let lhs = self.alloc_lvis(lhs, None);
                Stmt::Assign(Assign { lhs, expr })
            }
            _ => {
                return Err(ParseError::ExpectedStatement(token, span));
            }
        };

        self.expect(Token::Semicolon)?;
        Ok(stmts)
    }

    fn parse_expr(&mut self, prev_op: Option<impl Operator>) -> ParseResult<&'static Expr<'static, ParsedAst>> {
        if let Some((Token::If, _)) = self.lexer.peek() {
            self.parse_cond()
        } else if let Some((Token::LBrace, _)) = self.lexer.peek() {
            self.parse_tensor()
        } else if let Some((Token::LSquare, _)) = self.lexer.peek() {
            self.parse_array()
        } else {
            self.parse_binary(prev_op)
        }
    }

    fn parse_cond(&mut self) -> ParseResult<&'static Expr<'static, ParsedAst>> {
        self.expect(Token::If)?;

        let cond = self.parse_expr(None::<Bop>)?;

        self.expect(Token::LBrace)?;
        let then_branch = self.parse_body()?;
        self.expect(Token::RBrace)?;

        self.expect(Token::Else)?;

        self.expect(Token::LBrace)?;
        let else_branch = self.parse_body()?;
        self.expect(Token::RBrace)?;

        Ok(self.alloc_expr(Expr::Cond(Cond { cond, then_branch, else_branch })))
    }

    fn parse_tensor(&mut self) -> ParseResult<&'static Expr<'static, ParsedAst>> {
        self.expect(Token::LBrace)?;

        let body = self.parse_body()?;

        self.expect(Token::Bar)?;

        let lb = self.parse_expr(Some(PrecedenceFloor(2)))?;

        let (token, span) = self.next()?;
        let (lb, iv, ub) = match token {
            Token::Le => {
                let (iv, _) = self.parse_id()?;
                self.expect(Token::Lt)?;
                let ub = self.parse_expr(Some(PrecedenceFloor(2)))?;
                (Some(lb), iv, ub)
            }
            Token::Lt => {
                let Expr::Id(Id::Var(iv)) = lb else {
                    return Err(ParseError::UnexpectedToken("iteration variable".to_owned(), token, span));
                };
                let ub = self.parse_expr(Some(PrecedenceFloor(2)))?;
                (None, iv.clone(), ub)
            }
            _ => {
                return Err(ParseError::UnexpectedToken("expected '<' or '<='".to_owned(), token, span));
            }
        };

        self.expect(Token::RBrace)?;

        let iv = self.alloc_lvis(iv, None);
        Ok(self.alloc_expr(Expr::Tensor(Tensor {
            body,
            iv,
            lb,
            ub,
        })))
    }

    fn parse_binary(&mut self, prev_op: Option<impl Operator>) -> ParseResult<&'static Expr<'static, ParsedAst>> {
        let (token, span_start) = self.next()?;

        let mut left = match token {
            Token::Fold => self.parse_fold()?,
            Token::Prf(id) => self.parse_prf_call(id, span_start)?,
            Token::Identifier(id) => {
                if let Some((Token::LParen, _)) = self.lexer.peek() {
                    self.parse_call(id)?
                } else {
                    self.alloc_expr(Expr::Id(Id::Var(id)))
                }
            }
            Token::BoolValue(v) => self.alloc_expr(Expr::Const(Const::Bool(v))),
            Token::U32Value(v) => self.alloc_expr(Expr::Const(Const::U32(v))),
            Token::U64Value(v) => self.alloc_expr(Expr::Const(Const::U64(v))),
            Token::NatValue(v) => self.alloc_expr(Expr::Const(Const::Usize(v))),
            Token::UsizeValue(v) => self.alloc_expr(Expr::Const(Const::Usize(v))),
            Token::I32Value(v) => self.alloc_expr(Expr::Const(Const::I32(v))),
            Token::I64Value(v) => self.alloc_expr(Expr::Const(Const::I64(v))),
            Token::RealValue(v) => self.alloc_expr(Expr::Const(Const::F32(v))),
            Token::F32Value(v) => self.alloc_expr(Expr::Const(Const::F32(v))),
            Token::F64Value(v) => self.alloc_expr(Expr::Const(Const::F64(v))),
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

        // Handle postfix operators (selection, function calls, etc.)
        left = self.parse_postfix(left)?;

        while let Some((op, _loc)) = self.parse_binary_operator(&prev_op)? {
            let right = self.parse_expr(Some(op))?;
            left = self.alloc_expr(Expr::Call(Call {
                id: op.symbol().to_owned(),
                args: vec![left, right],
            }));

            left = self.parse_postfix(left)?;
        }

        Ok(left)
    }

    fn parse_postfix(&mut self, operand: &'static Expr<'static, ParsedAst>) -> ParseResult<&'static Expr<'static, ParsedAst>> {
        let mut expr = operand;

        while let Some((Token::LSquare, _)) = self.lexer.peek() {
            expr = self.parse_sel(expr)?;
        }

        Ok(expr)
    }

    fn parse_unary(&mut self, op: Uop) -> ParseResult<&'static Expr<'static, ParsedAst>> {
        let r = self.parse_expr(Some(op))?;
        Ok(self.alloc_expr(Expr::Call(Call {
            id: op.symbol().to_owned(),
            args: vec![r],
        })))
    }

    fn parse_call(&mut self, id: String) -> ParseResult<&'static Expr<'static, ParsedAst>> {
        self.expect(Token::LParen)?;

        let mut args = Vec::new();

        if self.matches(Token::RParen).is_none() {
            args.push(self.parse_expr(None::<Bop>)?);

            while self.matches(Token::Comma).is_some() {
                args.push(self.parse_expr(None::<Bop>)?);
            }

            self.expect(Token::RParen)?;
        }

        Ok(self.alloc_expr(Expr::Call(Call { id, args })))
    }

    fn parse_callable_name(&mut self) -> ParseResult<String> {
        let (token, span) = self.next()?;
        match token {
            Token::Identifier(name) => Ok(name),
            Token::Prf(name) => Ok(format!("@{name}")),
            _ => Err(ParseError::UnexpectedToken("function name".to_owned(), token, span)),
        }
    }

    fn parse_prf_call(&mut self, id: String, span: Span) -> ParseResult<&'static Expr<'static, ParsedAst>> {
        let mut args = Vec::new();

        self.expect(Token::LParen)?;
        if self.matches(Token::RParen).is_none() {
            args.push(self.parse_expr(None::<Bop>)?);

            while self.matches(Token::Comma).is_some() {
                args.push(self.parse_expr(None::<Bop>)?);
            }

            self.expect(Token::RParen)?;
        }

        use PrfCall::*;
        let call = match (id.as_str(), args.as_slice()) {
            ("dimA", [a]) => DimA(*a),
            ("shapeA", [a]) => ShapeA(*a),
            ("selVxA", [a, b]) => SelVxA(*a, *b),
            ("addSxS", [a, b]) => AddSxS(*a, *b),
            ("subSxS", [a, b]) => SubSxS(*a, *b),
            ("mulSxS", [a, b]) => MulSxS(*a, *b),
            ("divSxS", [a, b]) => DivSxS(*a, *b),
            ("ltSxS", [a, b]) => LtSxS(*a, *b),
            ("leSxS", [a, b]) => LeSxS(*a, *b),
            ("gtSxS", [a, b]) => GtSxS(*a, *b),
            ("geSxS", [a, b]) => GeSxS(*a, *b),
            ("eqSxS", [a, b]) => EqSxS(*a, *b),
            ("neSxS", [a, b]) => NeSxS(*a, *b),
            ("negS", [a]) => NegS(*a),
            ("notS", [a]) => NotS(*a),
            _ => {
                return Err(ParseError::UnknownPrimitive(id.clone(), span));
            }
        };

        Ok(self.alloc_expr(Expr::PrfCall(call)))
    }

    fn fold_dispatch_from_token(&self, token: Token, span: Span) -> ParseResult<String> {
        let id = match token {
            Token::Identifier(name) => name,
            Token::Prf(name) => format!("@{name}"),
            token => {
                let op: Bop = (&token).try_into().map_err(|_| {
                    ParseError::UnexpectedToken(
                        "fold function".to_owned(),
                        token.clone(),
                        span,
                    )
                })?;
                op.symbol().to_owned()
            }
        };
        Ok(id)
    }

    fn parse_fold_fun_arg(&mut self) -> ParseResult<FoldFunArg<'static, ParsedAst>> {
        let expr = self.parse_expr(None::<Bop>)?;
        if matches!(expr, Expr::Id(Id::Var(name)) if name == "_") {
            Ok(FoldFunArg::Placeholder)
        } else {
            Ok(FoldFunArg::Bound(expr))
        }
    }

    fn parse_fold_fun(&mut self) -> ParseResult<FoldFun<'static, ParsedAst>> {
        let (token, span) = self.next()?;
        let id = self.fold_dispatch_from_token(token, span)?;

        if self.matches(Token::LParen).is_none() {
            return Ok(FoldFun::Name(id));
        }

        let mut args = Vec::new();
        if self.matches(Token::RParen).is_none() {
            args.push(self.parse_fold_fun_arg()?);
            while self.matches(Token::Comma).is_some() {
                args.push(self.parse_fold_fun_arg()?);
            }
            self.expect(Token::RParen)?;
        }

        Ok(FoldFun::Apply { id, args })
    }

    fn parse_fold(&mut self) -> ParseResult<&'static Expr<'static, ParsedAst>> {
        self.expect(Token::LParen)?;

        let neutral = self.parse_expr(None::<Bop>)?;
        self.expect(Token::Comma)?;

        let foldfun = self.parse_fold_fun()?;
        self.expect(Token::Comma)?;

        let selection_expr = self.parse_expr(None::<Bop>)?;
        let selection = match selection_expr {
            Expr::Tensor(tensor) => tensor.clone(),
            _ => return Err(ParseError::FoldSelectionMustBeTensor),
        };

        self.expect(Token::RParen)?;

        Ok(self.alloc_expr(Expr::Fold(Fold {
            neutral,
            foldfun,
            selection,
        })))
    }

    fn parse_array(&mut self) -> ParseResult<&'static Expr<'static, ParsedAst>> {
        let mut values = Vec::new();

        self.expect(Token::LSquare)?;
        if self.matches(Token::RSquare).is_none() {
            values.push(self.parse_expr(None::<Bop>)?);

            while self.matches(Token::Comma).is_some() {
                let v = self.parse_expr(None::<Bop>)?;
                values.push(v);
            }

            self.expect(Token::RSquare)?;
        }

        Ok(self.alloc_expr(Expr::Array(Array { elems: values })))
    }

    fn parse_sel(&mut self, arr: &'static Expr<'static, ParsedAst>) -> ParseResult<&'static Expr<'static, ParsedAst>> {
        self.expect(Token::LSquare)?;
        let idx = self.parse_expr(None::<Bop>)?;
        self.expect(Token::RSquare)?;

        Ok(self.alloc_expr(Expr::Call(Call {
            id: "@sel".to_owned(),
            args: vec![idx, arr],
        })))
    }

    fn parse_binary_operator(&mut self, previous: &Option<impl Operator>) -> ParseResult<Option<(Bop, Span)>> {
        if let Some((token, _)) = self.lexer.peek()
            && let Ok(op) = token.try_into()
            && precedes(previous, &op)?
        {
            let (_, span) = self.lexer.next().unwrap();
            Ok(Some((op, span)))
        } else {
            Ok(None)
        }
    }

    fn parse_type(&mut self) -> ParseResult<(Type, Span)> {
        let (base, span) = self.parse_basetype()?;

        let ty = if self.matches(Token::LSquare).is_some() {
            let shape = if self.matches(Token::Mul).is_some() {
                TypePattern::Any
            } else {
                let axes = self.parse_axes()?;
                TypePattern::Axes(axes)
            };

            self.expect(Token::RSquare)?;
            Type { ty: base, shape }
        } else {
            Type::scalar(base)
        };

        Ok((ty, span))
    }

    fn parse_basetype(&mut self) -> ParseResult<(BaseType, Span)> {
        let (token, span) = self.next()?;
        let base = match token {
            Token::BoolType  => BaseType::Bool,
            Token::I32Type   => BaseType::I32,
            Token::I64Type   => BaseType::I64,
            Token::U32Type   => BaseType::U32,
            Token::U64Type   => BaseType::U64,
            Token::UsizeType => BaseType::Usize,
            Token::F32Type   => BaseType::F32,
            Token::F64Type   => BaseType::F64,
            Token::Identifier(udf) => BaseType::Udf(udf),
            _ => return Err(ParseError::UnexpectedToken("base type".to_owned(), token, span)),
        };

        Ok((base, span))
    }

    fn parse_axes(&mut self) -> ParseResult<Vec<AxisPattern>> {
        let mut axes = Vec::new();
        axes.push(self.parse_axis()?);
        while self.matches(Token::Comma).is_some() {
            axes.push(self.parse_axis()?);
        }
        Ok(axes)
    }

    fn parse_axis(&mut self) -> ParseResult<AxisPattern> {
        let (token, span) = self.next()?;
        match token {
            Token::NatValue(n) => Ok(AxisPattern::Dim(DimPattern::Known(n as usize))),
            Token::Identifier(name) => {
                if name == "_" {
                    Ok(AxisPattern::Dim(DimPattern::Any))
                } else if self.matches(Token::Gt).is_some() || self.matches(Token::Ge).is_some() {
                    match self.next()? {
                        (Token::NatValue(_), _) => {}
                        (token, span) => {
                            return Err(ParseError::UnexpectedToken(
                                "natural number bound".to_owned(),
                                token,
                                span,
                            ))
                        }
                    }
                    self.expect(Token::Colon)?;
                    let (shp_name, _) = self.parse_id()?;
                    Ok(AxisPattern::Rank(RankCapture {
                        dim_name: name,
                        shp_name,
                    }))
                } else if self.matches(Token::Colon).is_some() {
                    let (shp_name, _) = self.parse_id()?;
                    Ok(AxisPattern::Rank(RankCapture {
                        dim_name: name,
                        shp_name,
                    }))
                } else {
                    Ok(AxisPattern::Dim(DimPattern::Var(name)))
                }
            }
            Token::Dot => Ok(AxisPattern::Dim(DimPattern::Any)),
            _ => Err(ParseError::UnexpectedToken("axis pattern".to_owned(), token, span)),
        }
    }

    fn parse_id(&mut self) -> ParseResult<(String, Span)> {
        let (token, span) = self.next()?;
        match token {
            Token::Identifier(id) => Ok((id, span)),
            _ => Err(ParseError::UnexpectedToken("identifier".to_owned(), token, span)),
        }
    }
}
