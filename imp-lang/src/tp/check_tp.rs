use std::collections::HashSet;

use crate::ast::*;

/// Not all patterns that can be constructed from the grammar are actually resolvable.
/// This pass rejects unresolved variable-rank patterns (`d:shp`) at compile time.
pub fn check_tp(program: Program<'static, ParsedAst>) -> Result<Program<'static, ParsedAst>, String> {
	CheckTypePatterns::new().run(program)
}

struct CheckTypePatterns {
	errors: Vec<String>,
}

impl CheckTypePatterns {
	fn new() -> Self {
		Self { errors: Vec::new() }
	}

	fn run(mut self, program: Program<'static, ParsedAst>) -> Result<Program<'static, ParsedAst>, String> {
        for (_, groups) in &program.overloads {
            for (_, fundefs) in groups {
                for fundef in fundefs {
                    self.check_fundef(fundef);
                }
            }
        }

		if self.errors.is_empty() {
			Ok(program)
		} else {
			Err(self.errors.join("\n"))
		}
	}

	fn check_fundef(&mut self, fundef: &Fundef<'static, ParsedAst>) {
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

		self.check_return_pattern(&fundef.name, &fundef.ret_type.shape, &defined_symbols);
	}

	fn collect_arg_symbols(
		&mut self,
		arg: &Farg,
		defined_symbols: &mut HashSet<String>,
		unconstrained_rank_captures: &mut usize,
	) {
		let TypePattern::Axes(axes) = &arg.ty.shape else {
			return;
		};

		for axis in axes {
			match axis {
				AxisPattern::Dim(DimPattern::Var(var)) => {
					defined_symbols.insert(var.clone());
				}
				AxisPattern::Rank(capture) => {
					if !defined_symbols.contains(&capture.dim_name) {
						*unconstrained_rank_captures += 1;
					}

					defined_symbols.insert(capture.dim_name.clone());
					defined_symbols.insert(capture.shp_name.clone());
				}
				AxisPattern::Dim(DimPattern::Any) | AxisPattern::Dim(DimPattern::Known(_)) => {}
			}
		}

	}

	fn check_return_pattern(&mut self, fundef_name: &str, ret_shape: &TypePattern, defined_symbols: &HashSet<String>) {
		let TypePattern::Axes(axes) = ret_shape else {
			return;
		};

		for axis in axes {
			if let AxisPattern::Rank(capture) = axis
				&& !defined_symbols.contains(&capture.dim_name) {
				self.errors.push(format!(
					"function `{}` return type contains unconstrained rank capture `{}`; return rank captures must be constrained by argument symbols",
					fundef_name, capture.dim_name
				));
			}
		}
	}
}
