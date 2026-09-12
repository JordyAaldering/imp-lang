use std::collections::HashSet;

use crate::{Phase, ast::*};

/// Not all patterns that can be constructed from the grammar are actually resolvable.
/// This pass rejects unresolved variable-rank patterns (`d:shp`) at compile time.
pub fn check_tp<'ast>(mut program: Program<'ast, ParsedAst>) -> Result<Program<'ast, ParsedAst>, String> {
	let mut trav = CheckTypePatterns::default();

	trav.trav_program(&mut program);

	if trav.errors.is_empty() {
		Ok(program)
	} else {
		Err(trav.errors.join("\n"))
	}
}

#[derive(Default)]
struct CheckTypePatterns {
	fundef_name: String,
	unconstrained_rank_captures: usize,
	defined_symbols: HashSet<String>,
	errors: Vec<String>,
}

impl<'ast> Traverse<'ast> for CheckTypePatterns {
	const PHASE: Phase = Phase::CTP;

	type Ast = ParsedAst;

	type DeclOut = ();

	type ExprOut = ();

	fn trav_fundef(&mut self, fundef: &mut Fundef<'ast, Self::Ast>) {
		self.fundef_name = fundef.name.clone();
		self.unconstrained_rank_captures = 0;
		self.defined_symbols.clear();

		for arg in &fundef.args {
			self.defined_symbols.insert(arg.id.clone());
		}

		self.trav_fargs(&mut fundef.args);

		if self.unconstrained_rank_captures > 1 {
			self.errors.push(format!(
				"function `{}` has {} unconstrained rank captures in argument type patterns; at most one is allowed",
				fundef.name, self.unconstrained_rank_captures
			));
		}

		self.trav_fret(&mut fundef.ret_type);
	}

	fn trav_farg(&mut self, arg: &mut Farg) {
		for axis in &arg.ty.shape.0 {
			match axis {
				AxisPattern::FixedDim { .. } => {}
				AxisPattern::VarDim { len: None } => {}
				AxisPattern::VarDim { len: Some(len) } => {
					self.defined_symbols.insert(len.clone());
				}
				AxisPattern::FixedShape { shp, .. } => {
					if let Some(shp) = shp {
						self.defined_symbols.insert(shp.clone());
					}
				}
				AxisPattern::VarShape { dim, shp, .. } => {
					if let Some(dim) = dim {
						if !self.defined_symbols.contains(dim) {
							self.unconstrained_rank_captures += 1;
						}

						self.defined_symbols.insert(dim.clone());
					} else {
						self.unconstrained_rank_captures += 1;
					}

					if let Some(shp) = shp {
						self.defined_symbols.insert(shp.clone());
					}
				}
			}
		}
	}

	fn trav_fret(&mut self, ret_type: &mut Type) {
		for axis in &ret_type.shape.0 {
			match axis {
				AxisPattern::VarShape { dim, .. } => {
					if dim.as_ref().is_none_or(|dim| !self.defined_symbols.contains(dim)) {
						self.errors.push(format!(
							"function `{}` return type contains unconstrained rank capture `{}`; return rank captures must be constrained by argument symbols",
							self.fundef_name, dim.as_ref().unwrap_or(&"_".to_string())
						));
					}
				}
				_ => {}
			}
		}
	}
}
