# Plan: Compiler Rewrite for Overloading-First Semantics

This plan implements the design in [doc/newplan.md](doc/newplan.md):

1. Remove trait-based dispatch entirely.
2. Disable typesets/member constraints for now.
3. Make overloading the core abstraction.
4. Resolve overloads by base-type groups at compile time and shape checks at runtime.

## Design Targets

### Language Targets (Phase-1 scope)

1. Keep function declarations and calls.
2. Allow overloads by argument base types and argument count.
3. For overloads with the same argument base types, require identical return base types and return arity.
4. Lower operators to function calls (for example `a + b` to `+(a, b)`).
5. Keep shape patterns (`n`, `d:shp`, `d>0:shp`, literals), but do not solve global ambiguity yet.
6. Disable trait syntax, trait impl syntax, and all trait type-checking/codegen paths.
7. Disable typeset/member/generic membership constraints for now.

### Non-Goals (explicitly deferred)

1. Trait system redesign.
2. Global proof of non-ambiguous overload DAGs.
3. Generic function constraints over typesets.
4. User-defined algebraic types.

## Architecture Changes by Compiler Stage

### 1) AST and IR

1. Replace trait-specific nodes with overload-aware function groups.
2. Add canonical signatures:
	- `name`
	- `arg_base_types`
	- `arg_count`
	- `return_base_types`
	- shape-pattern metadata per argument and return.
3. Add an `OverloadSet` map keyed by `(name, arg_count, arg_base_types)`.
4. Keep function bodies as they are; only dispatch metadata changes.

### 2) Parser and Surface Syntax

1. Remove parser branches for:
	- `trait ...`
	- `impl ...`
	- trait-style `where` bounds.
2. Keep parsing for overloaded `fn name(...)` declarations.
3. Parse primitive-style overload names (`fn @add(...)`, `fn @sel(...)`, and so on).
4. Keep shape annotations and constrained rank captures (`d>0:shp`) in types.

### 3) Name Resolution and Symbol Tables

1. Change symbol table from `name -> single function` to `name -> overload family`.
2. Build base-type groups early (before type inference finishes bodies).
3. Validate that duplicate signatures are rejected.

### 4) Type Inference and Overload Resolution

1. Resolve call target in two stages:
	- stage A: choose overload family by `name`, `arg_count`, and argument base types.
	- stage B: within family, choose most specific shape-pattern match.
2. Enforce legality rule:
	- all overloads in one base-type family have same return base types and return count.
3. If shape choice is statically unique, bind directly.
4. If shape choice needs runtime info, mark call as `requires_wrapper_dispatch`.
5. For now, if two candidates are both best and incomparable, emit a deterministic ambiguity error.

### 5) Operator and Selection Lowering

1. Replace trait checks for unary/binary operators.
2. Lower operators directly to function calls:
	- `a + b` -> `+(a, b)`
	- `-a` -> `-(a)`
3. Keep selection lowering independent of traits (no `Sel` trait lookup).
4. Ensure all previous trait-based diagnostics are replaced with overload-not-found diagnostics.

### 6) Code Generation

1. Generate concrete implementations per overload case.
2. For each base-type family with multiple shape variants, generate one wrapper dispatcher.
3. Wrapper strategy:
	- evaluate runtime shape/rank predicates in sorted specificity order,
	- call first matching concrete overload,
	- emit runtime error if no case matches.
4. Preserve existing ABI behavior for arrays and FFI.

### 7) Pretty Printer / Show

1. Remove trait/impl rendering.
2. Print overload groups for debugging:
	- group key
	- sorted case list
	- selected wrapper symbol.

## Comparator and Ordering Rules (Implementation)

For a fixed base-type group, define `cmp_shape_pattern(a, b)`:

1. `scalar` is more specific than any non-scalar capture.
2. fixed-rank fixed-dim pattern is more specific than rank-capture pattern.
3. constrained rank-capture (`d>k:shp`) is more specific than unconstrained rank-capture (`d:shp`).
4. if two patterns constrain different dimensions and neither implies the other, result is `incomparable`.

This yields a partial order (DAG). Implementation detail for phase 1:

1. sort with a stable topological heuristic where possible,
2. keep incomparable peers in source order,
3. detect runtime ambiguity at compile time when two incomparable overloads can both match known static shape facts.

## Migration Plan (Execution Order)

### Milestone 0: Freeze and Baseline

1. Add snapshot tests for currently supported non-trait functions.
2. Add negative tests for removed syntax with expected diagnostics.
3. Record baseline of `cargo check` and sample codegen outputs.

### Milestone 1: Syntax Removal and AST Pivot

1. Delete trait and impl AST variants.
2. Remove parser support for trait/impl syntax.
3. Keep primitive-style overload names parseable.
4. Make all existing examples compile with plain overloaded `fn` declarations.

Exit criteria:
1. No trait-related code path compiles.
2. No trait-related syntax remains in the active parser pipeline.

### Milestone 2: Overload Symbol Table

1. Introduce overload family index.
2. Add duplicate-signature and return-base consistency checks.
3. Wire call-site lookup by `(name, arg_count, arg_base_types)`.

Exit criteria:
1. Deterministic overload lookup for scalar-only overloads.

### Milestone 3: Shape-Specific Dispatch in Type Inference

1. Implement shape-pattern comparator and candidate filtering.
2. Mark wrapper-required calls.
3. Implement ambiguity diagnostics for incomparable best candidates.

Exit criteria:
1. Vector/scalar and array/array overload samples type-check correctly.
2. Ambiguous samples fail with clear errors.

### Milestone 4: Codegen Wrappers

1. Emit wrapper functions per overload family needing runtime shape checks.
2. Emit runtime condition chain and fallthrough error path.
3. Route call-sites to direct symbol or wrapper symbol.

Exit criteria:
1. Generated C executes correct case for mixed shape inputs.
2. No trait shim generation remains.

### Milestone 5: Cleanup and Hardening

1. Remove trait-specific files/modules and dead tests.
2. Remove temporary compatibility code.
3. Expand test matrix for operators rewritten as overload calls.

Exit criteria:
1. `cargo check` clean.
2. Integration examples pass.
3. No references to traits/typeset membership in active pipeline.

## File-Level Refactor Map

Likely primary touch points:

1. Parser and lexer: [imp-lang/src/scp/parser.rs](imp-lang/src/scp/parser.rs), [imp-lang/src/scp/lexer.rs](imp-lang/src/scp/lexer.rs)
2. AST definitions: [imp-lang/src/ast.rs](imp-lang/src/ast.rs) and [imp-lang/src/ast](imp-lang/src/ast)
3. Type inference: [imp-lang/src/tc/type_infer.rs](imp-lang/src/tc/type_infer.rs)
4. Codegen: [imp-lang/src/cg/codegen_c.rs](imp-lang/src/cg/codegen_c.rs), [imp-lang/src/cg](imp-lang/src/cg)
5. Pretty printer: [imp-lang/src/show.rs](imp-lang/src/show.rs)
6. Legacy pass cleanup: [imp-lang/src/pre](imp-lang/src/pre), [imp-lang/src/opt](imp-lang/src/opt)
7. Language examples: [example/src](example/src)

## Test Strategy

### Positive tests

1. Overloaded operator `+` for scalar/scalar, scalar/array, array/scalar, array/array.
2. Same function name with different base-type groups.
3. Same base-type group with different shape patterns and deterministic selection.

### Negative tests

1. Duplicate overload with same argument base-type signature.
2. Same argument base-type signature but incompatible return base types.
3. Ambiguous shape patterns.
4. Use of removed syntax (`trait`, `impl`, typeset/member constraints).

## Practical Rollout

1. Implement milestones 1 through 3 first in one branch to stabilize front-end semantics.
2. Implement milestone 4 once inference metadata is stable.
3. Final cleanup and example updates in milestone 5.

This order minimizes rework by making dispatch semantics final before C wrapper emission.
