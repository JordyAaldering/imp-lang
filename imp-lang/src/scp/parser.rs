use std::{collections::{HashMap, HashSet}, iter::Peekable};

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
    DuplicateTypeset(String),
    UndefinedTypeset(String),
    DuplicateGenericFunction(String),
    UnknownPrimitive(String, Span),
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
        let mut functions = HashMap::new();
        let mut typesets = HashSet::new();
        let mut members = HashMap::new();
        let mut traits = HashMap::new();
        let mut impls = Vec::new();

        while self.lexer.peek().is_some() {
            match self.peek()?.0.clone() {
                Token::Public | Token::Fn => {
                    let fundef = self.parse_fundef()?;
                    let name = fundef.name.clone();
                    if functions.insert(name.clone(), fundef).is_some() {
                        return Err(ParseError::DuplicateFunction(name));
                    }
                }
                Token::Trait => {
                    let trait_def = self.parse_trait_def()?;
                    traits.insert(trait_def.name.clone(), trait_def);
                }
                Token::Typeset => {
                    let typeset = self.parse_typeset_def()?;
                    if !typesets.insert(typeset.clone()) {
                        return Err(ParseError::DuplicateTypeset(typeset));
                    }
                    members.insert(typeset, Vec::new());
                }
                Token::Member => {
                    let (typeset_name, member) = self.parse_member_def()?;
                    if let Some(typeset) = members.get_mut(&typeset_name) {
                        typeset.push(member);
                    } else {
                        return Err(ParseError::UndefinedTypeset(typeset_name));
                    }
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

        Ok(Program { functions, typesets, members, traits, impls })
    }

    fn parse_fundef(&mut self) -> ParseResult<Fundef<'static, ParsedAst>> {
        let is_public = self.matches(Token::Public).is_some();

        let _ = self.expect(Token::Fn)?;
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
            is_public,
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
                let lhs = self.alloc_lvis(lhs, None);
                vec![Stmt::Assign(Assign { lhs, expr })]
            }
            Token::Return => {
                let expr = self.parse_expr(None::<Bop>)?;
                match expr {
                    Expr::Id(id) => vec![Stmt::Return(Return { id: id.clone() })],
                    _ => {
                        let ret_name = self.fresh_uid();
                        let ret_lvis = self.alloc_lvis(ret_name.clone(), None);
                        vec![
                            Stmt::Assign(Assign { lhs: ret_lvis, expr }),
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
            Token::NatValue(v) => self.alloc_expr(Expr::I32(v)),
            Token::I32Value(v) => self.alloc_expr(Expr::I32(v)),
            Token::I64Value(v) => self.alloc_expr(Expr::I64(v)),
            Token::U32Value(v) => self.alloc_expr(Expr::U32(v)),
            Token::U64Value(v) => self.alloc_expr(Expr::U64(v)),
            Token::UsizeValue(v) => self.alloc_expr(Expr::Usize(v)),
            Token::RealValue(v) => self.alloc_expr(Expr::F32(v)),
            Token::F32Value(v) => self.alloc_expr(Expr::F32(v)),
            Token::F64Value(v) => self.alloc_expr(Expr::F64(v)),
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

        // Parse optional <T, U, ...> type parameters
        let mut type_params = Vec::new();
        if self.matches(Token::Lt).is_some() {
            let (param, _) = self.parse_id()?;
            type_params.push(param);
            while self.matches(Token::Comma).is_some() {
                let (param, _) = self.parse_id()?;
                type_params.push(param);
            }
            self.expect(Token::Gt)?;
        }

        self.expect(Token::ColonColon)?;
        let args = self.parse_poly_sig_types()?;
        self.expect(Token::Arrow)?;
        let ret = self.parse_poly_type()?;
        self.expect(Token::Semicolon)?;
        Ok(TraitDef { name, type_params, args, ret })
    }

    fn parse_poly_sig_types(&mut self) -> ParseResult<Vec<PolyType>> {
        self.expect(Token::LParen)?;
        let mut args = Vec::new();
        if self.matches(Token::RParen).is_none() {
            loop {
                let ty = self.parse_poly_type()?;
                // Optional argument name in signatures, e.g. `(usize[n] idx, T[n] arr)`.
                if matches!(self.peek()?.0, Token::Identifier(_)) {
                    let _ = self.parse_id()?;
                }
                args.push(ty);

                if self.matches(Token::Comma).is_some() {
                    continue;
                }
                self.expect(Token::RParen)?;
                break;
            }
        }
        Ok(args)
    }

    fn parse_typeset_def(&mut self) -> ParseResult<String> {
        self.expect(Token::Typeset)?;
        let (name, _) = self.parse_id()?;
        self.expect(Token::Semicolon)?;
        Ok(name)
    }

    fn parse_member_def(&mut self) -> ParseResult<(String, BaseType)> {
        self.expect(Token::Member)?;
        let (typeset, _) = self.parse_id()?;
        self.expect(Token::ColonColon)?;
        let (member, _) = self.parse_basetype()?;
        self.expect(Token::Semicolon)?;
        Ok((typeset, member))
    }

    fn parse_impl_def(&mut self) -> ParseResult<ImplDef> {
        self.expect(Token::Impl)?;

        // Parse optional <T: Constraint, U: Other, ...> — inline type params + bounds
        let mut type_params = Vec::new();
        let mut where_bounds = Vec::new();
        if self.matches(Token::Lt).is_some() {
            let (param, _) = self.parse_id()?;
            if self.matches(Token::Colon).is_some() {
                let (type_set, _) = self.parse_id()?;
                where_bounds.push(MemberBound { type_var: param.clone(), type_set });
            }
            type_params.push(param);
            while self.matches(Token::Comma).is_some() {
                let (param, _) = self.parse_id()?;
                if self.matches(Token::Colon).is_some() {
                    let (type_set, _) = self.parse_id()?;
                    where_bounds.push(MemberBound { type_var: param.clone(), type_set });
                }
                type_params.push(param);
            }
            self.expect(Token::Gt)?;
        }

        let (trait_name, _) = self.parse_id()?;
        // New syntax: TraitName(args) -> ret  (no `::` before args)
        let args = self.parse_poly_sig_types()?;
        self.expect(Token::Arrow)?;
        let ret_type = self.parse_poly_type()?;

        self.expect(Token::LBrace)?;
        let mut methods = Vec::new();
        if matches!(self.peek()?.0, Token::Fn) {
            while self.peek()?.0 != Token::RBrace {
                methods.push(self.parse_impl_method_sig()?);
            }
            self.expect(Token::RBrace)?;
        } else {
            self.skip_block_contents()?;
        }

        Ok(ImplDef {
            trait_name,
            args,
            ret_type,
            type_params,
            where_bounds,
            methods,
        })
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
            Token::I32Type => "i32".to_owned(),
            Token::I64Type => "i64".to_owned(),
            Token::U32Type => "u32".to_owned(),
            Token::U64Type => "u64".to_owned(),
            Token::UsizeType => "usize".to_owned(),
            Token::F32Type => "f32".to_owned(),
            Token::F64Type => "f64".to_owned(),
            Token::BoolType => "bool".to_owned(),
            // User-defined types, such as `member Complex :: complex32`
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

    fn skip_block_contents(&mut self) -> ParseResult<()> {
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

        Ok(self.alloc_expr(Expr::Array(Array { elems: values })))
    }

    // TODO: as with binary operators, this should assume that a trait Sel is defined.
    // Currently, we just map this to @selVxA, but that should change.
    fn parse_sel(&mut self, arr: &'static Expr<'static, ParsedAst>) -> ParseResult<&'static Expr<'static, ParsedAst>> {
        self.expect(Token::LSquare)?;

        let idx = self.parse_expr(None::<Bop>)?;
        self.expect(Token::RSquare)?;

        Ok(self.alloc_expr(Expr::Call(Call {
            id: "sel".to_owned(),
            args: vec![idx, arr],
        })))
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
        let (base, span) = self.parse_basetype()?;

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

    fn parse_basetype(&mut self) -> ParseResult<(BaseType, Span)> {
        let (token, span) = self.next()?;

        let base = match token {
            Token::I32Type => BaseType::I32,
            Token::I64Type => BaseType::I64,
            Token::U32Type => BaseType::U32,
            Token::U64Type => BaseType::U64,
            Token::UsizeType => BaseType::Usize,
            Token::F32Type => BaseType::F32,
            Token::F64Type => BaseType::F64,
            Token::BoolType => BaseType::Bool,
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
            Token::NatValue(n) => Ok(AxisPattern::Dim(DimPattern::Known(n as u64))),
            Token::Identifier(name) => {
                if name == "_" {
                    Ok(AxisPattern::Dim(DimPattern::Any))
                } else if self.matches(Token::Gt).is_some() || self.matches(Token::Ge).is_some() {
                    // Constrained rank-and-shape capture like `m>0:ishp`.
                    // The lower-bound is currently syntax-only and not stored in the AST.
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
                        dim_role: SymbolRole::Define,
                    }))
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
