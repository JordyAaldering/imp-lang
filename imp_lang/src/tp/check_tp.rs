use std::collections::HashSet;

use crate::ast::{AxisPattern::VariableRank, *};

/// Not all patterns that can be constructed from the grammar are actually resolvable.
/// This pass rejects unresolved variable-rank patterns (`d:shp`) at compile time.
pub fn check_tp<'ast>(program: Program<'ast, ParsedAst>) -> Result<Program<'ast, ParsedAst>, String> {
	CheckTypePatterns::new().run(program)
}

struct CheckTypePatterns {
	errors: Vec<String>,
}

impl CheckTypePatterns {
	fn new() -> Self {
		Self { errors: Vec::new() }
	}

	fn run<'ast>(mut self, program: Program<'ast, ParsedAst>) -> Result<Program<'ast, ParsedAst>, String> {
        for (_, groups) in &program.overloads {
            for (_, fundef_ids) in groups {
                for fundef_id in fundef_ids {
					self.check_fundef(program.fundef(*fundef_id));
                }
            }
        }

		if self.errors.is_empty() {
			Ok(program)
		} else {
			Err(self.errors.join("\n"))
		}
	}

	fn check_fundef(&mut self, fundef: &Fundef<'_, ParsedAst>) {
		let mut defined_symbols: HashSet<String> = HashSet::new();

		// Scalar argument names are valid symbolic constraints for later type patterns.
		for arg in &fundef.args {
			defined_symbols.insert(arg.id.clone());
		}

		let mut unconstrained_rank_captures = 0usize;

		for arg in &fundef.args {
			self.collect_arg_symbols(arg, &mut defined_symbols, &mut unconstrained_rank_captures);
		}

		if unconstrained_rank_captures > 1 {
			self.errors.push(format!(
				"function `{}` has {} unconstrained rank captures in argument type patterns; at most one is allowed",
				fundef.name, unconstrained_rank_captures
			));
		}

		self.check_return_pattern(&fundef.name, &fundef.ret_type, &defined_symbols);
	}

	fn collect_arg_symbols(
		&mut self,
		arg: &Farg,
		defined_symbols: &mut HashSet<String>,
		unconstrained_rank_captures: &mut usize,
	) {
		let Some(axes) = arg.ty.type_pattern() else {
			return;
		};

		for axis in axes {
			match axis {
				AxisPattern::VariableRank { dim, shp } => {
					if !defined_symbols.contains(dim) {
						*unconstrained_rank_captures += 1;
					}

					defined_symbols.insert(dim.clone());
					defined_symbols.insert(shp.clone());
				},
				AxisPattern::FixedRank { dim: _, shp: _ } => {
					todo!()
				},
				AxisPattern::VariableLength { len } => {
					defined_symbols.insert(len.clone());
				},
				AxisPattern::FixedLength { len: _ } => {},
			}
		}
	}

	fn check_return_pattern(&mut self, fundef_name: &str, ret_shape: &Type, defined_symbols: &HashSet<String>) {
		let Some(axes) = ret_shape.type_pattern() else {
			return;
		};

		for axis in axes {
			if let VariableRank { dim, shp: _ } = axis
				&& !defined_symbols.contains(dim)
			{
				self.errors.push(format!(
					"function `{}` return type contains unconstrained rank capture `{}`; return rank captures must be constrained by argument symbols",
					fundef_name, dim
				));
			}
		}
	}
}
