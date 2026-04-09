# Type Pattern Refactor Plan

## Goal

Refactor type-pattern handling so that:

1. Type patterns remain declarative and contain only type-level symbolic information.
2. Runtime shape extraction is synthesized explicitly into function-local statements that later code generation can reuse directly.
3. Symbolic relationships between argument and return shapes remain available for later type checking and optimization.
4. Return-side dimensionality is always constrained by the inputs.

This separates three concerns that are currently conflated:

- type-level symbolic shape contracts
- runtime value extraction for shape symbols
- special identifier plumbing for dimensions and shapes

## Current Problems

The current implementation encodes binding behavior inside type patterns via `SymbolRole` and then materializes shape symbols indirectly through special identifier forms such as `Id::Dim`, `Id::Shp`, and `Id::DimAt`.

That creates several issues:

1. Type patterns carry binding/runtime behavior instead of only type information.
2. Shape-derived values are not represented as ordinary statements, so code generation cannot reuse them cleanly.
3. Static reasoning about input/output shape equalities would have to reconstruct semantics from special identifiers.
4. Output-side shape symbols are not validated as being constrained by the inputs.

## Target Architecture

Each function definition should carry two compiler-synthesized shape artifacts:

1. `shape_prelude`
Compiler-generated statements inserted conceptually at the top of the function body. These bind runtime shape values as ordinary variables.

2. `shape_facts`
Symbolic facts derived from the function signature that later type checking and optimization can query without inspecting generated statements.

The type pattern itself remains declarative and symbolic.

## Example Lowerings

### `i32[n] a`

Generated runtime prelude:

```imp
usize n = @selVxA([0usize], @shapeA(a));
```

Generated symbolic facts:

- `arg0.axis0 == symbol(n)`

### `i32[n,n] b`

Generated runtime prelude:

```imp
usize n = @selVxA([0usize], @shapeA(b));
```

Deferred check metadata:

- `@selVxA([1usize], @shapeA(b)) == n`

Generated symbolic facts:

- `arg0.axis0 == symbol(n)`
- `arg0.axis1 == symbol(n)`

### `i32[n,d:shp] c`

Generated runtime prelude:

```imp
usize n = @selVxA([0usize], @shapeA(c));
usize d = @subSxS(@dimA(c), 1usize);
usize[d] shp = @takeSxV(d, @dropSxV(1usize, @shapeA(c)));
```

Generated symbolic facts:

- `arg0.axis0 == symbol(n)`
- `arg0.tail_shape(start = 1) == capture(d, shp)`

We omit runtime check generation for now, but we do keep the deferred equality facts so checks can be inserted later.

## Validation Rules

### Rule 1: Type patterns stay declarative

`TypePattern` may contain symbolic names such as `n` or `d:shp`, but it may not contain binding roles or runtime values.

### Rule 2: Return dimensionality must be constrained by inputs

Any return-side dimension or rank symbol must already be introduced by an argument pattern.

Allowed:

```imp
fn @add(i32[d:shp] a, i32[d:shp] b) -> i32[d:shp]
fn @copy(i32[n] a) -> i32[n]
```

Rejected:

```imp
fn bad(i32 a) -> i32[n]
fn bad2(i32[n] a) -> i32[m]
fn bad3(i32[n] a) -> i32[d:shp]
```

The `shp` tail-shape name is the only exception: it may appear on the return side without previously appearing by name on an input, as long as its associated rank symbol is input-constrained.

### Rule 2b: Multiple variable-rank patterns are allowed

Type patterns may contain multiple variable-rank captures (for example `o:sv, i:ishp`) as long as **at most one** captured rank variable is unconstrained by previously known symbols.

This allows patterns such as planar take:

```imp
fn take(int[o] sv, i32[o:ohsp,i:ishp] arr) -> i32[o:sv,i:ishp]
```

Interpretation:

1. `o` is constrained by `sv`.
2. `i` may remain the single unconstrained rank capture.
3. If two or more rank captures in the same pattern are unconstrained, the signature is invalid.

Implementation note:

- rank constraints must be tracked per axis/capture site, not as one global `RetRank` symbol.
- symbolic facts should preserve axis identity for rank captures.

### Rule 3: Repeated symbols imply equality constraints

Repeated symbolic dimensions do not redefine a symbol. They create symbolic equalities and, later, optional runtime checks.

For rank captures, repeated `dim_name` across capture sites similarly implies rank-equality facts.

### Rule 4: Runtime shape extraction is explicit

Every runtime-derived symbol used in the function body should come from synthesized prelude statements rather than special identifier forms.

## Data Model Changes

### 1. Simplify `TypePattern`

In `imp-lang/src/ast/typ.rs`:

- remove `SymbolRole`
- remove `ExtentVar.role`
- remove `RankCapture.dim_role`

Keep symbolic names only.

### 2. Extend `Fundef`

In `imp-lang/src/ast/fundef.rs`, add:

- `shape_prelude: Vec<Stmt<'ast, Ast>>`
- `shape_facts: ShapeFacts<'ast, Ast>`

`shape_prelude` is executable compiler-synthesized code.
`shape_facts` is symbolic metadata for reasoning.

### 3. Introduce symbolic shape fact types

Add a new AST module for symbolic facts. Recommended representation:

```rust
pub struct ShapeFacts<'ast, Ast: AstConfig> {
    pub bindings: Vec<ShapeBinding<'ast, Ast>>,
    pub equalities: Vec<ShapeEquality>,
    pub output_constraints: Vec<OutputShapeConstraint>,
}
```

Use normalized symbolic terms rather than raw strings where possible:

```rust
pub enum ShapeTerm {
    Known(usize),
    Symbol(String),
    ArgDim { arg_index: usize, axis_index: usize },
    ArgRank { arg_index: usize, axis_index: usize },
    RetDim { axis_index: usize },
    RetRank { axis_index: usize },
}
```

This is the stable contract later passes should consume.

## New Primitive Operations

The refactor requires additional primitive shape intrinsics:

- `@shapeA(array) -> usize[*]`
- `@dimA(array) -> usize`
- `@dropSxV(usize, usize[*]) -> usize[*]`
- `@takeSxV(usize, usize[*]) -> usize[*]`

Initially, they can be typed conservatively. Precision can be improved later.

## Pass Responsibilities

### Phase 1: AST scaffolding

Add `shape_prelude` and `shape_facts` to `Fundef`, define the new shape-fact AST types, and thread them through traversal without changing semantics.

### Phase 2: Type-pattern elaboration

Repurpose `tp::analyse_tp` into a real elaboration pass that:

1. scans argument type patterns left-to-right
2. records which symbols are introduced by inputs
3. synthesizes runtime `shape_prelude` statements
4. synthesizes symbolic `shape_facts`
5. records deferred equality checks for repeated symbols
6. validates that return-side rank/dimension symbols are input-constrained
7. allows multiple rank captures only if at most one remains unconstrained

### Phase 3: Primitive support

Add the new shape intrinsics to:

- parser
- AST `PrfCall`
- type inference
- code generation

### Phase 4: Flatten and SSA

Flatten and SSA-transform `shape_prelude` before the user body.

At this point, `n`, `d`, and `shp` become ordinary variables in the body environment.

### Phase 5: Remove special shape identifiers

Once the prelude pipeline works, remove:

- `Id::Dim`
- `Id::Shp`
- `Id::DimAt`

Then clean up all dependent logic in:

- flattening
- SSA
- type inference
- constant folding
- show
- code generation

### Phase 6: Use symbolic shape facts in type checking

Update type inference and overload result typing to consult `shape_facts` instead of relying purely on name coincidences.

This is the step that enables later optimization passes to prove that output shapes equal input shapes.

### Phase 7: Optimization hooks

Use `shape_facts` to statically simplify shape comparisons and remove redundant runtime checks.

## Design Choices

### Keep symbolic facts separate from runtime bindings

Do not attempt to recover symbolic equalities by inspecting synthesized prelude statements. Both artifacts should be produced from the same elaboration pass, but they serve different purposes.

### Keep `TypePattern` non-executable

Type patterns should not store runtime values or binding roles. They remain declarative.

### Do not immediately make `Type` depend on `AstConfig`

Pattern elements may eventually want links to corresponding SSA definitions, but that should come through `Fundef`-level metadata first. Making `Type` generic over AST stage would be a much larger invasive change.

### Keep deferred checks as metadata first

Even if runtime checks are not emitted yet, repeated-symbol constraints should be recorded so they can be reused for validation and future code generation.

## First Implementation Step

Introduce the new AST scaffolding only:

1. add `shape_prelude` and `shape_facts` to `Fundef`
2. define `ShapeFacts`, `ShapeBinding`, `ShapeEquality`, `OutputShapeConstraint`, and `ShapeTerm`
3. thread them through traversal and constructors
4. keep them empty everywhere for now

This step is intentionally non-semantic. It creates the structure needed for the later elaboration pass without changing behavior.

## Expected Dead Code After Full Refactor

Once the full refactor is complete, the following should disappear:

- `SymbolRole`
- role-aware type-pattern logic
- flatten-time type-pattern env binding
- `Id::Dim`, `Id::Shp`, `Id::DimAt`
- direct codegen special-casing for shape-derived identifiers

## Verification Strategy

After each phase:

1. `cargo build -q`
2. `cargo run --bin example`
3. add targeted tests for legal/illegal constrained return patterns
4. add tests for future static resolvability of input/output shape comparisons
