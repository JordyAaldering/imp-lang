use std::iter::Peekable;

use crate::ast::*;

use super::{lexer::*, operator::*, span::*};

pub struct Parser<'src> {
    lexer: Peekable<Lexer<'src>>,
    uid: usize,
}

#[derive(Debug)]
#[allow(unused)]
pub enum ParseError {
    NonAssociative,
    MissingReturn,
    DuplicateFunction(String),
    DuplicateGenericFunction(String),
    UnknownPrimitive(String, Span),
    UnexpectedToken(String, Token, Span),
    UnexpectedEof,
}

enum ParsedFnItem {
    Concrete(Fundef<'static, ParsedAst>),
    Generic(GenericFundef<'static, ParsedAst>),
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
        let mut functions = std::collections::HashMap::new();
        let mut generic_functions = std::collections::HashMap::new();
        let mut traits = std::collections::HashMap::new();
        let mut impls = Vec::new();

        while self.lexer.peek().is_some() {
            match self.peek()?.0.clone() {
                Token::Fn => {
                    match self.parse_fn_item()? {
                        ParsedFnItem::Concrete(fundef) => {
                            let name = fundef.name.clone();
                            if functions.insert(name.clone(), fundef).is_some() {
                                return Err(ParseError::DuplicateFunction(name));
                            }
                        }
                        ParsedFnItem::Generic(fundef) => {
                            let name = fundef.name.clone();
                            if generic_functions.insert(name.clone(), fundef).is_some() {
                                return Err(ParseError::DuplicateGenericFunction(name));
                            }
                        }
                    }
                }
                Token::Trait => {
                    let trait_def = self.parse_trait_def()?;
                    traits.insert(trait_def.name.clone(), trait_def);
                }
                Token::Impl => {
                    impls.push(self.parse_impl_def()?);
                }
                _ => {
                    let (token, span) = self.next()?;
                    return Err(ParseError::UnexpectedToken("top-level item".to_owned(), token, span));
                }
            }
        }

        Ok(Program { functions, generic_functions, traits, impls })
    }

    fn parse_fn_item(&mut self) -> ParseResult<ParsedFnItem> {
        let (_, _span_start) = self.expect(Token::Fn)?;
        let (name, _) = self.parse_id()?;

        if self.matches(Token::Lt).is_some() {
            let (type_param, _) = self.parse_id()?;
            self.expect(Token::Gt)?;

            let args = self.parse_poly_args()?;
            self.expect(Token::Arrow)?;
            let ret_type = self.parse_poly_type()?;

            let mut where_bounds = Vec::new();
            if self.matches(Token::Where).is_some() {
                where_bounds.push(self.parse_trait_bound()?);
                while self.matches(Token::Comma).is_some() {
                    where_bounds.push(self.parse_trait_bound()?);
                }
            }

            self.expect(Token::LBrace)?;
            let mut body = Vec::new();
            while self.peek()?.0 != Token::RBrace {
                body.extend(self.parse_stmt()?);
            }
            self.expect(Token::RBrace)?;

            if !matches!(body.last(), Some(Stmt::Return(_))) {
                return Err(ParseError::MissingReturn);
            }

            Ok(ParsedFnItem::Generic(GenericFundef {
                name,
                type_param,
                where_bounds,
                ret_type,
                args,
                decs: Vec::new(),
                body,
            }))
        } else {
            Ok(ParsedFnItem::Concrete(self.parse_fundef_after_name(name)?))
        }
    }

    fn parse_fundef_after_name(&mut self, name: String) -> ParseResult<Fundef<'static, ParsedAst>> {

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

    fn parse_binary(&mut self, prev_op: Option<impl Operator>) -> ParseResult<&'static Expr<'static, ParsedAst>> {
        let (token, span_start) = self.next()?;

        let mut left = match token {
            Token::Prf(id) => self.parse_prf_call(id, span_start)?,
            Token::Identifier(id) => {
                if let Some((Token::LParen, _)) = self.lexer.peek() {
                    self.parse_call(id)?
                } else {
                    self.alloc_expr(Expr::Id(Id::Var(id)))
                }
            },
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
        if let Some((Token::LSquare, _)) = self.lexer.peek() {
            self.parse_sel(operand)
        } else {
            Ok(operand)
        }
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

    fn parse_trait_def(&mut self) -> ParseResult<TraitDef> {
        self.expect(Token::Trait)?;
        let (name, _) = self.parse_id()?;
        self.expect(Token::Lt)?;
        let (param, _) = self.parse_id()?;
        self.expect(Token::Gt)?;
        self.expect(Token::LBrace)?;

        let mut methods = Vec::new();
        while self.peek()?.0 != Token::RBrace {
            methods.push(self.parse_trait_method_sig()?);
        }

        self.expect(Token::RBrace)?;
        Ok(TraitDef { name, param, methods })
    }

    fn parse_impl_def(&mut self) -> ParseResult<ImplDef> {
        self.expect(Token::Impl)?;
        let (trait_name, _) = self.parse_id()?;
        self.expect(Token::Lt)?;
        let for_type = self.parse_poly_type()?;
        self.expect(Token::Gt)?;

        let mut where_bounds = Vec::new();
        if self.matches(Token::Where).is_some() {
            where_bounds.push(self.parse_trait_bound()?);
            while self.matches(Token::Comma).is_some() {
                where_bounds.push(self.parse_trait_bound()?);
            }
        }

        self.expect(Token::LBrace)?;
        let mut methods = Vec::new();
        while self.peek()?.0 != Token::RBrace {
            methods.push(self.parse_impl_method_sig()?);
        }
        self.expect(Token::RBrace)?;

        Ok(ImplDef {
            trait_name,
            for_type,
            where_bounds,
            methods,
        })
    }

    fn parse_trait_method_sig(&mut self) -> ParseResult<TraitMethodSig> {
        self.expect(Token::Fn)?;
        let name = self.parse_method_name()?;
        let args = self.parse_poly_args()?;
        self.expect(Token::Arrow)?;
        let ret_type = self.parse_poly_type()?;
        self.expect(Token::Semicolon)?;
        Ok(TraitMethodSig { name, args, ret_type })
    }

    fn parse_impl_method_sig(&mut self) -> ParseResult<TraitMethodSig> {
        self.expect(Token::Fn)?;
        let name = self.parse_method_name()?;
        let args = self.parse_poly_args()?;
        self.expect(Token::Arrow)?;
        let ret_type = self.parse_poly_type()?;
        self.skip_block()?;
        Ok(TraitMethodSig { name, args, ret_type })
    }

    fn parse_poly_args(&mut self) -> ParseResult<Vec<PolyArg>> {
        let mut args = Vec::new();
        self.expect(Token::LParen)?;
        if self.matches(Token::RParen).is_none() {
            args.push(self.parse_poly_arg()?);
            while self.matches(Token::Comma).is_some() {
                args.push(self.parse_poly_arg()?);
            }
            self.expect(Token::RParen)?;
        }
        Ok(args)
    }

    fn parse_poly_arg(&mut self) -> ParseResult<PolyArg> {
        let ty = self.parse_poly_type()?;
        let (name, _) = self.parse_id()?;
        Ok(PolyArg { name, ty })
    }

    fn parse_poly_type(&mut self) -> ParseResult<PolyType> {
        let (token, span) = self.next()?;
        let head = match token {
            Token::U32Type => "u32".to_owned(),
            Token::UsizeType => "usize".to_owned(),
            Token::BoolType => "bool".to_owned(),
            Token::Identifier(name) => name,
            _ => return Err(ParseError::UnexpectedToken("type".to_owned(), token, span)),
        };

        let shape = if self.matches(Token::LSquare).is_some() {
            if self.matches(Token::Mul).is_some() {
                self.expect(Token::RSquare)?;
                Some(ShapePattern::Any)
            } else {
                let axes = self.parse_axes()?;
                self.expect(Token::RSquare)?;
                Some(ShapePattern::Axes(axes))
            }
        } else {
            None
        };

        Ok(PolyType { head, shape })
    }

    fn parse_trait_bound(&mut self) -> ParseResult<TraitBound> {
        let (ty_name, _) = self.parse_id()?;
        self.expect(Token::Colon)?;
        let (trait_name, _) = self.parse_id()?;
        Ok(TraitBound { ty_name, trait_name })
    }

    fn parse_method_name(&mut self) -> ParseResult<String> {
        let (token, span) = self.next()?;
        match token {
            Token::Identifier(name) => Ok(name),
            Token::Add => Ok("+".to_owned()),
            Token::Sub => Ok("-".to_owned()),
            Token::Mul => Ok("*".to_owned()),
            Token::Div => Ok("/".to_owned()),
            Token::Eq => Ok("==".to_owned()),
            Token::Ne => Ok("!=".to_owned()),
            Token::Lt => Ok("<".to_owned()),
            Token::Le => Ok("<=".to_owned()),
            Token::Gt => Ok(">".to_owned()),
            Token::Ge => Ok(">=".to_owned()),
            Token::Not => Ok("!".to_owned()),
            _ => Err(ParseError::UnexpectedToken("method name".to_owned(), token, span)),
        }
    }

    fn skip_block(&mut self) -> ParseResult<()> {
        self.expect(Token::LBrace)?;
        let mut depth = 1usize;
        while depth > 0 {
            let (token, _) = self.next()?;
            match token {
                Token::LBrace => depth += 1,
                Token::RBrace => depth -= 1,
                _ => {}
            }
        }
        Ok(())
    }

    fn parse_prf_call(&mut self, id: String, at_span: Span) -> ParseResult<&'static Expr<'static, ParsedAst>> {
        let prf = Prf::try_from(id.as_str())
            .map_err(|_| ParseError::UnknownPrimitive(id.clone(), at_span))?;

        self.expect(Token::LParen)?;

        let mut args = Vec::new();

        if self.matches(Token::RParen).is_none() {
            args.push(self.parse_expr(None::<Bop>)?);

            while self.matches(Token::Comma).is_some() {
                args.push(self.parse_expr(None::<Bop>)?);
            }

            self.expect(Token::RParen)?;
        }

        Ok(self.alloc_expr(Expr::PrfCall(PrfCall { id: prf, args })))
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

    // TODO: as with binary operators, this should assume that a trait Sel is defined.
    // Currently, we just map this to @selAxV, but that should change.
    fn parse_sel(&mut self, arr: &'static Expr<'static, ParsedAst>) -> ParseResult<&'static Expr<'static, ParsedAst>> {
        self.expect(Token::LSquare)?;

        let idx = self.parse_expr(None::<Bop>)?;
        self.expect(Token::RSquare)?;

        Ok(self.alloc_expr(Expr::PrfCall(PrfCall { id: Prf::SelAxV, args: vec![arr, idx] })))
    }


    fn parse_binary_operator(&mut self, previous: &Option<impl Operator>) -> ParseResult<Option<(Bop, Span)>> {
        if let Some((token, _)) = self.lexer.peek()
            && let Ok(op) = token.try_into()
            && precedes(previous, &op)? {
            let (_, span) = self.lexer.next().unwrap();
            return Ok(Some((op, span)));
        }

        Ok(None)
    }

    fn parse_type(&mut self) -> ParseResult<(Type, Span)> {
        let (token, span) = self.next()?;
        let base = match token {
            Token::U32Type => BaseType::U32,
            Token::UsizeType => BaseType::Usize,
            Token::BoolType => BaseType::Bool,
            _ => return Err(ParseError::UnexpectedToken("type".to_owned(), token, span)),
        };

        let ty = if self.matches(Token::LSquare).is_some() {
            if self.matches(Token::Mul).is_some() {
                // u32[*] — shape fully unconstrained
                self.expect(Token::RSquare)?;
                Type {
                    ty: base,
                    shape: ShapePattern::Any,
                    knowledge: TypeKnowledge::AUD,
                }
            } else {
                let axes = self.parse_axes()?;
                self.expect(Token::RSquare)?;
                // Knowledge and symbol roles are resolved later by tp::analyse_tp.
                Type {
                    ty: base,
                    shape: ShapePattern::Axes(axes),
                    knowledge: TypeKnowledge::AUD,
                }
            }
        } else {
            Type::scalar(base)
        };

        Ok((ty, span))
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
            Token::U32Value(n) => Ok(AxisPattern::Dim(DimPattern::Known(n as u64))),
            Token::Identifier(name) => {
                if name == "_" {
                    Ok(AxisPattern::Dim(DimPattern::Any))
                } else if self.matches(Token::Colon).is_some() {
                    // `d:shp` rank-and-shape capture.
                    // Roles will be resolved by tp::analyse_tp.
                    let (shp_name, _) = self.parse_id()?;
                    Ok(AxisPattern::Rank(RankCapture {
                        dim_name: name,
                        shp_name,
                        dim_role: SymbolRole::Define,
                    }))
                } else {
                    // Plain dimension variable.
                    // Role will be resolved by tp::analyse_tp.
                    Ok(AxisPattern::Dim(DimPattern::Var(ExtentVar { name, role: SymbolRole::Define })))
                }
            }
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
