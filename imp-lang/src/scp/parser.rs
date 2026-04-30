use std::{collections::HashMap, iter::Peekable, mem};

use typed_arena::Arena;

use super::{lexer::*, operator::*, span::*};

use crate::ast::*;

pub struct Parser<'src, 'ast> {
    lexer: Peekable<Lexer<'src>>,
    decs_arena: Arena<VarInfo<'ast, ParsedAst>>,
    expr_arena: Arena<Expr<'ast, ParsedAst>>,
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

impl<'src, 'ast> Parser<'src, 'ast> {
    pub fn new(lexer: Lexer<'src>) -> Self {
        Self {
            lexer: lexer.peekable(),
            decs_arena: Arena::new(),
            expr_arena: Arena::new(),
        }
    }

    fn alloc_lvis(&self, name: String, ty: Option<Type>) -> &'ast VarInfo<'ast, ParsedAst> {
        unsafe { mem::transmute(self.decs_arena.alloc(VarInfo { name, ty, ssa: () })) }
    }

    fn alloc_expr(&self, expr: Expr<'ast, ParsedAst>) -> &'ast Expr<'ast, ParsedAst> {
        unsafe { mem::transmute(self.expr_arena.alloc(expr)) }
    }

    fn matches(&mut self, expected: &Token) -> Option<(Token, Span)> {
        self.lexer.next_if(|(token, _)| token == expected)
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

    /// ```bnf
    /// <items> = <item> ("sep" <item>)*
    /// ```
    fn parse_items<T, F>(&mut self, sep: &Token, parse_item: F) -> ParseResult<(Vec<T>, Span)>
    where
        F: Fn(&mut Self) -> ParseResult<(T, Span)>,
    {
        let mut items = Vec::new();
        let (arg0, mut span) = parse_item(self)?;
        items.push(arg0);
        while self.matches(sep).is_some() {
            let (arg, s) = parse_item(self)?;
            span.extend(&s);
            items.push(arg);
        }
        Ok((items, span))
    }

    fn parse_items_enclosed<T, F >(&mut self, open: Token, close: Token, sep: &Token, parse_item: F) -> ParseResult<(Vec<T>, Span)>
    where
        F: Fn(&mut Self) -> ParseResult<(T, Span)>,
    {
        let (_, span_from) = self.expect(open)?;

        if let Some((_, span_to)) = self.matches(&close) {
            Ok((Vec::new(), span_from.to(&span_to)))
        } else {
            let (items, _) = self.parse_items(sep, parse_item)?;
            let (_, span_to) = self.expect(close)?;
            Ok((items, span_from.to(&span_to)))
        }
    }

    /// ```bnf
    /// <program> = <fundef>*
    /// ```
    pub fn parse_program(&mut self) -> ParseResult<Program<'ast, ParsedAst>> {
        let mut overloads = HashMap::new();
        let fundefs_arena: Arena<Fundef<'ast, ParsedAst>> = Arena::new();

        while let Some((token, _)) = self.lexer.peek() {
            match token {
                Token::Fn => {
                    let (fundef, _) = self.parse_fundef()?;
                    let name = fundef.name.clone();
                    let sig = fundef.signature();
                    let fundef_ref = fundefs_arena.alloc(fundef);
                    // SAFETY: fundefs_arena is moved into Program before return.
                    let fundef_ref: &'ast Fundef<'ast, ParsedAst> = unsafe { mem::transmute(fundef_ref) };
                    let group = overloads.entry(name).or_insert(HashMap::new());
                    let fundefs = group.entry(sig).or_insert(Vec::new());
                    fundefs.push(fundef_ref);
                }
                _ => {
                    let (token, span) = self.next()?;
                    return Err(ParseError::UnexpectedToken("top-level item".to_owned(), token, span));
                }
            }
        }

        Ok(Program {
            overloads,
            fundefs: fundefs_arena,
        })
    }

    /// ```bnf
    /// <fundef> = "fn" <id> "(" <fargs>? ")" "->" <type> "{" <body> "}"
    /// ```
    fn parse_fundef(&mut self) -> ParseResult<(Fundef<'ast, ParsedAst>, Span)> {
        self.decs_arena = Arena::new();
        self.expr_arena = Arena::new();

        let (_, span_from) = self.expect(Token::Fn)?;
        let (name, _) = self.parse_id()?;

        let (args, _) = self.parse_items_enclosed(
            Token::LParen, Token::RParen, &Token::Comma,
            |p| p.parse_farg())?;

        self.expect(Token::Arrow)?;

        let (ret_type, _) = self.parse_type()?;

        self.expect(Token::LBrace)?;
        let body = self.parse_body()?;
        let (_, span_to) = self.expect(Token::RBrace)?;

        let decs = mem::take(&mut self.decs_arena);
        let exprs = mem::take(&mut self.expr_arena);

        Ok((Fundef {
            name,
            args,
            shape_prelude: Vec::new(),
            shape_facts: ShapeFacts::default(),
            decs,
            exprs,
            body,
            ret_type,
        }, span_from.to(&span_to)))
    }

    /// ```bnf
    /// <farg> = <type> <id>
    /// ```
    fn parse_farg(&mut self) -> ParseResult<(Farg, Span)> {
        let (ty, ty_span) = self.parse_type()?;
        let (id, id_span) = self.parse_id()?;
        Ok((Farg { id, ty }, ty_span.to(&id_span)))
    }

    /// ```bnf
    /// <body> = <stmt>* <expr>
    /// ```
    fn parse_body(&mut self) -> ParseResult<Body<'ast, ParsedAst>> {
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

        let (ret, _) = self.parse_expr(None::<Bop>)?;
        Ok(Body { stmts, ret })
    }

    fn parse_stmt(&mut self) -> ParseResult<Stmt<'ast, ParsedAst>> {
        let (token, span) = self.next()?;
        let stmts = match token {
            Token::Identifier(lhs) => {
                // A bit ugly, but good enough for now
                let (err_token, err_loc) = self.peek()?.clone();
                self.expect(Token::Assign)
                    .map_err(|_| ParseError::ExpectedStatement(err_token, err_loc))?;

                let (expr, _) = self.parse_expr(None::<Bop>)?;
                let lhs = self.alloc_lvis(lhs, None);
                Stmt::Assign(Assign { lhs, expr })
            }
            Token::Printf => {
                self.expect(Token::LParen)?;
                let (id, _) = self.parse_id()?;
                self.expect(Token::RParen)?;
                Stmt::Printf(Printf { id: Id::Var(id) })
            },
            _ => {
                return Err(ParseError::ExpectedStatement(token, span));
            }
        };

        self.expect(Token::Semicolon)?;
        Ok(stmts)
    }

    fn parse_expr(&mut self, prev_op: Option<impl Operator>) -> ParseResult<(&'ast Expr<'ast, ParsedAst>, Span)> {
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

    fn parse_cond(&mut self) -> ParseResult<(&'ast Expr<'ast, ParsedAst>, Span)> {
        let (_, span_from) = self.expect(Token::If)?;

        let (cond, _) = self.parse_expr(None::<Bop>)?;

        self.expect(Token::LBrace)?;
        let then_branch = self.parse_body()?;
        self.expect(Token::RBrace)?;

        self.expect(Token::Else)?;

        self.expect(Token::LBrace)?;
        let else_branch = self.parse_body()?;
        let (_, span_to) = self.expect(Token::RBrace)?;

        let expr = self.alloc_expr(Expr::Cond(Cond { cond, then_branch, else_branch }));
        Ok((expr, span_from.to(&span_to)))
    }

    fn parse_tensor(&mut self) -> ParseResult<(&'ast Expr<'ast, ParsedAst>, Span)> {
        let (_, span_from) = self.expect(Token::LBrace)?;

        let body = self.parse_body()?;

        self.expect(Token::Bar)?;

        let (lb, _) = self.parse_expr(Some(PrecedenceFloor(2)))?;

        let (token, span) = self.next()?;
        let (lb, iv, ub) = match token {
            Token::Le => {
                let (iv, _) = self.parse_id()?;
                self.expect(Token::Lt)?;
                let (ub, _) = self.parse_expr(Some(PrecedenceFloor(2)))?;
                (Some(lb), iv, ub)
            }
            Token::Lt => {
                let Expr::Id(Id::Var(iv)) = lb else {
                    return Err(ParseError::UnexpectedToken("iteration variable".to_owned(), token, span));
                };
                let (ub, _) = self.parse_expr(Some(PrecedenceFloor(2)))?;
                (None, iv.clone(), ub)
            }
            _ => {
                return Err(ParseError::UnexpectedToken("expected '<' or '<='".to_owned(), token, span));
            }
        };

        let (_, span_to) = self.expect(Token::RBrace)?;

        let iv = self.alloc_lvis(iv, None);
        let tensor = self.alloc_expr(Expr::Tensor(Tensor {
            body,
            iv,
            lb,
            ub,
        }));

        Ok((tensor, span_from.to(&span_to)))
    }

    fn parse_binary(&mut self, prev_op: Option<impl Operator>) -> ParseResult<(&'ast Expr<'ast, ParsedAst>, Span)> {
        let (token, span_from) = self.next()?;

        let mut left = match token {
            Token::Fold => self.parse_fold()?,
            Token::Prf(id) => self.parse_prf_call(id, span_from)?,
            Token::Identifier(id) => {
                if let Some((Token::LParen, _)) = self.lexer.peek() {
                    self.parse_call(id)?
                } else {
                    self.alloc_expr(Expr::Id(Id::Var(id)))
                }
            }
            Token::RealValue(v) => self.alloc_expr(Expr::Const(Const::F32(v))),
            Token::NatValue(v) => self.alloc_expr(Expr::Const(Const::Usize(v))),
            Token::BoolValue(v) => self.alloc_expr(Expr::Const(Const::Bool(v))),
            Token::UsizeValue(v) => self.alloc_expr(Expr::Const(Const::Usize(v))),
            Token::U32Value(v) => self.alloc_expr(Expr::Const(Const::U32(v))),
            Token::U64Value(v) => self.alloc_expr(Expr::Const(Const::U64(v))),
            Token::I32Value(v) => self.alloc_expr(Expr::Const(Const::I32(v))),
            Token::I64Value(v) => self.alloc_expr(Expr::Const(Const::I64(v))),
            Token::F32Value(v) => self.alloc_expr(Expr::Const(Const::F32(v))),
            Token::F64Value(v) => self.alloc_expr(Expr::Const(Const::F64(v))),
            Token::LParen => {
                let (expr, _) = self.parse_expr(None::<Bop>)?;

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
                        span_from,
                    )
                })?;
                self.parse_unary(op)?
            }
        };

        // Handle postfix operators (selection, function calls, etc.)
        left = self.parse_postfix(left)?;

        while let Some((op, _loc)) = self.parse_binary_operator(&prev_op)? {
            let (right, _) = self.parse_expr(Some(op))?;
            left = self.alloc_expr(Expr::Call(Call {
                id: op.symbol().to_owned(),
                args: vec![left, right],
            }));

            left = self.parse_postfix(left)?;
        }

        Ok((left, span_from))
    }

    fn parse_postfix(&mut self, operand: &'ast Expr<'ast, ParsedAst>) -> ParseResult<&'ast Expr<'ast, ParsedAst>> {
        let mut expr = operand;

        while let Some((Token::LSquare, _)) = self.lexer.peek() {
            expr = self.parse_sel(expr)?;
        }

        Ok(expr)
    }

    fn parse_unary(&mut self, op: Uop) -> ParseResult<&'ast Expr<'ast, ParsedAst>> {
        let (r, _) = self.parse_expr(Some(op))?;
        Ok(self.alloc_expr(Expr::Call(Call {
            id: op.symbol().to_owned(),
            args: vec![r],
        })))
    }

    fn parse_call(&mut self, id: String) -> ParseResult<&'ast Expr<'ast, ParsedAst>> {
        let (args, _) = self.parse_items_enclosed(
            Token::LParen, Token::RParen, &Token::Comma,
            |p| p.parse_expr(None::<Bop>))?;
        Ok(self.alloc_expr(Expr::Call(Call { id, args })))
    }

    fn parse_prf_call(&mut self, id: String, span: Span) -> ParseResult<&'ast Expr<'ast, ParsedAst>> {
        let (args, _) = self.parse_items_enclosed(
            Token::LParen, Token::RParen, &Token::Comma,
            |p| p.parse_expr(None::<Bop>))?;

        use Prf::*;
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

        Ok(self.alloc_expr(Expr::Prf(call)))
    }

    fn fold_dispatch_from_token(&self, token: Token, span: Span) -> ParseResult<String> {
        let id = match token {
            Token::Identifier(name) => name,
            Token::Prf(_) => panic!("folding with primitive functions not yet supported"),
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

    fn parse_fold_fun_arg(&mut self) -> ParseResult<(FoldFunArg<'ast, ParsedAst>, Span)> {
        let (expr, span) = self.parse_expr(None::<Bop>)?;
        if matches!(expr, Expr::Id(Id::Var(name)) if name == "_") {
            Ok((FoldFunArg::Placeholder, span))
        } else {
            Ok((FoldFunArg::Bound(expr), span))
        }
    }

    fn parse_fold_fun(&mut self) -> ParseResult<FoldFun<'ast, ParsedAst>> {
        let (token, span) = self.next()?;
        let id = self.fold_dispatch_from_token(token, span)?;

        if self.matches(&Token::LParen).is_none() {
            return Ok(FoldFun::Name(id));
        }

        let (args, _) = self.parse_items_enclosed(
            Token::LParen, Token::RParen, &Token::Comma,
            |p| p.parse_fold_fun_arg())?;

        Ok(FoldFun::Apply { id, args })
    }

    fn parse_fold(&mut self) -> ParseResult<&'ast Expr<'ast, ParsedAst>> {
        self.expect(Token::LParen)?;

        let (neutral, _) = self.parse_expr(None::<Bop>)?;
        self.expect(Token::Comma)?;

        let foldfun = self.parse_fold_fun()?;
        self.expect(Token::Comma)?;

        let (selection_expr, _) = self.parse_expr(None::<Bop>)?;
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

    fn parse_array(&mut self) -> ParseResult<(&'ast Expr<'ast, ParsedAst>, Span)> {
        let (elems, span) = self.parse_items_enclosed(
            Token::LSquare, Token::RSquare, &Token::Comma,
            |p| p.parse_expr(None::<Bop>))?;

        let expr = self.alloc_expr(Expr::Array(Array { elems }));
        Ok((expr, span))
    }

    fn parse_sel(&mut self, arr: &'ast Expr<'ast, ParsedAst>) -> ParseResult<&'ast Expr<'ast, ParsedAst>> {
        self.expect(Token::LSquare)?;
        let (idx, _) = self.parse_expr(None::<Bop>)?;
        self.expect(Token::RSquare)?;

        Ok(self.alloc_expr(Expr::Call(Call {
            id: "sel".to_owned(),
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

        let ty = if self.matches(&Token::LSquare).is_some() {
            let shape = if self.matches(&Token::Mul).is_some() {
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
        while self.matches(&Token::Comma).is_some() {
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
                } else if self.matches(&Token::Gt).is_some() || self.matches(&Token::Ge).is_some() {
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
                } else if self.matches(&Token::Colon).is_some() {
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
