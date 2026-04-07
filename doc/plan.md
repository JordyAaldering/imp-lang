# Plan: Traits, Type Patterns, and Mixed-Shape Overloading

```imp
trait Add :: (A, B) -> R;
trait Num :: ();
```

Meaning:
1. `Add` represents one callable signature.
2. `A`, `B`, `R` are signature slots.
3. Impl heads bind these slots to concrete or pattern-generic types.

## Basic Examples

Scalar impls:

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

1. Single best impl: for any call, resolution must produce exactly one impl.
2. Orphan ownership: an impl is allowed only if the module owns the trait or at least one concrete base type in the impl head.
   (For example, it must be the module that defined `trait Add`, or it must be a module that defines a new trait `complex` and uses that type in an `impl`)
3. Overlap is an error unless one impl is strictly more specific.
4. Specificity order:
   - concrete dim (`[4]`) > named dim (`[n]`) > wildcard (`[.]`)
   - fixed-rank axes > rank-capture (`d:shp`)
   - But not concrete base type > type variable: one or the other should be implemented, never both.
     (E.g., it `Add :: (T, T) -> T` is defined for all `Num`, one may not add an explicit `f32` case.)

This prevents silent cross-module behaviour changes.
