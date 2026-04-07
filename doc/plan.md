# Plan: Traits, Type Patterns, and Mixed-Shape Overloading

## Chosen Notation

Use callable traits for operations and a dedicated concept notation for marker constraints.

```imp
type Num :: T;

trait Add :: (A, B) -> R;
```

Meaning:
1. `Num(T)` is a membership predicate, not a callable trait.
2. `Add` is a callable contract.
3. `A`, `B`, and `R` are signature slots.

Membership declarations:

```imp
member Num :: u32;
member Num :: usize;
member Num :: f32;
```

### User-defined types

Far in the future, we want to support user-defined types.
We don't do this yet, but should keep the notation in mind so that we can remain consistent across notations.

```imp
typedef complex32 :: (f32, f32);
typedef complex64 :: (f64, f64);

type Complex;
member Complex :: complex32;
member Complex :: complex64;
```

## Basic Examples

Scalar addition:

```imp
impl Add :: (u32, u32) -> u32 {
	fn +(u32 a, u32 b) -> u32 { return @addSxS(a, b); }
}

impl Add :: (usize, usize) -> usize {
	fn +(usize a, usize b) -> usize { return @addSxS(a, b); }
}
```

Array lifting:

```imp
impl<T> Add :: (T[d:shp,n], T[d:shp,n]) -> T[d:shp,n]
where
	Num(T[n]),
	Add :: (T[n], T[n]) -> T[n]
{
	fn +(T[d:shp,n] a, T[d:shp,n] b) -> T[d:shp,n] {
		return { a[iv] + b[iv] | iv < shp };
	}
}
```

Mixed scalar/array:

```imp
impl Add :: (f32, f32[d:shp]) -> f32[d:shp] {
	fn +(f32 a, f32[d:shp] b) -> f32[d:shp] {
		return { @addSxS(a, b[iv]) | iv < shp };
	}
}

impl Add :: (f32[d:shp], f32) -> f32[d:shp] {
	fn +(f32[d:shp] a, f32 b) -> f32[d:shp] {
		return { @addSxS(a[iv], b) | iv < shp };
	}
}
```

## Coherence Rules

1. Single best impl: for any call, exactly one impl may apply.
2. Orphan ownership: an impl is allowed only if the module owns the trait or at least one concrete base type in the impl head.
3. Overlap is an error unless one impl is strictly more specific.
4. Specificity order:
   - concrete dim (`[4]`) > named dim (`[n]`) > wildcard (`[.]`)
   - fixed-rank axes > rank-capture (`d:shp`)
5. Avoid mixing fully generic and concrete base impls for the same signature space unless a strict specialization rule exists.

These rules prevent silent cross-module behavior changes.
