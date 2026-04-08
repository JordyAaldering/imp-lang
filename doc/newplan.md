# Typesets

I like the typeset and membership notation we already have.
I see no reason to change this.

```imp
typeset Num;

member Num :: i32;
member Num :: i64;
```

Besides the obvious reduction in code duplication, another benefit of this notation is that we only need to generate the necessary cases on demand.
If one defines `iota<T: Num> :: usize[n] -> T`, we only generate the necessary cases based on applications of `iota`.

This is possible for base types (usize, i32, and even user-defined types like complex32), but not in general for shapes.

If we define some function for both `i32[n]` and `i32[d>2:shp]`, we may not know which occurrence we need as shape information may only be available at runtime.
Thus, we can group overloads based on base types, but need to generate all shape variants at once.

## Disable for now

Although the typeset and member notation will be incredibly useful in the future, we should strip it from the compiler for now.

It is not necessary for a proof-of-concept compiler, and makes the compilation process significantly more complicated.

# Traits

Although we currently use a trait-based method, I think this is overkill, and stops us from overloading with different argument counts.
In any case, the trait based syntax would require so many additional parameters to support the base type/shape mismatch in the previous paragraph that any potential readability gains are lost.

For example, for `Add`, we had:

```imp
trait Add<T> :: (T[*], T[*]) -> T[*];
```

Which is to say, the addition of any two arguments of the same type, returning something of the same type.
The `[*]` notation here means that in any position, the shape may be anything.
This is necessary because we may have scalar-scalar addition, but also scalar-array, array-scalar, or array-array.

```imp
impl Add(i32 a, i32 b) -> i32
{
    return @addSxS(a, b);
}

impl Add(i32[d:shp] a, i32 b) -> i32[d:shp]
{
    return { @addSxS(a[iv], b) | iv < shp };
}
```

Furthermore, the user may or may not define that the two shapes in the array-array case should be the same.
Such constraints can never be expressed in the trait, but this means that we also cannot add such constraints if the trait actually expects it.

The main motivation for traits was to map them to unary/binary operators.
E.g., the trait `Add` is expected to be implemented whenever `a + b` is encountered.
This is not necessary however, and actually limits users from adding values of different types, which may make sense like `f32 + complex32`.

## Remove entirely

The trait-based method does not fit our HPC functional array-language needs.
We get rid of the trait-based method entirely.

# Overloading

Using overloading instead, it may look as:

```imp
fn +(i32 a, i32 b) -> i32 {
    return @addSxS(a, b);
}

fn +(i32[d:shp] a, i32 b) -> i32[d:shp] {
    return { @addSxS(@selVxA(iv, a), b) | iv < shp };
}

fn +(f32 a, complex32 b) -> complex32 b {
    // User-defined types are not supported yet, this is just to highlight the idea
    return b
}
```

Overloading based on base types is simple, everything else being the same,
clearly a function may not be overloaded if the base types of the arguments are the same.

```imp
fn illegal_overload(i32 a) -> f32 { return 0f32; }

// We overload only based on formal arguments.
// The number of return values must be the same for all overloads.
fn illegal_overload(i32 a) -> f64 { return 0f64; }
```

We need to be able to decide base types at compile time.
Thus, return base types may not change arbitrarily.
For any overload with the same base type arguments, disregarding their shapes, the return value must have the same base type.

E.g., although the previous example is illegal, the following are legal:

```imp
fn legal_overload(i32 a) -> i32 { ... }

fn legal_overload(i32[n] a) -> i32[n] { ... }

fn legal_overload(i32 a, i32 b) -> bool { ... }

fn legal_overload(f64 a) -> f32 { ... }
```

## Replace traits with overloading

Overloading fits our needs better, we should replace the trait-based method with an overloading-method.

Whenever we find an operator, e.g. `a + b`, we naively replace it with a function call `+(a, b)`, letting type inferencing or dispatching figure out if the needed overload actually exists.

# Overload ordering

Deciding which overload to pick based on base types is simple; base types are always known at compile time.

In the previous example, we get three 'groups' of base-type-overloads.
Namely, based on the base types of the arguments: `(i32)`, `(i32, i32)`, and `(f64)`.
Note that, the first two overloads with argument types `i32 a` and `i32[n] a` end up in the same 'group',
because it is not necessarily known at compile time what the shape of a value is.

Thus, within each group, we may need to create so-called 'wrappers' which, if need be, check the shape information during runtime to pick the correct overload.

In our mental model, this looks somewhat like:

```imp
// rank-0 case (i.e., the scalar case)
fn legal_overload__i32_0(i32 a) -> i32 { ... }

// rank-1 case, where the first rank has length n (i.e., the vector case)
fn legal_overload__i32_1_n(i32[n] a) -> i32[n] { ... }

fn legal_overload__i32(i32[d:shp] a) -> i32[*] {
    if d == 0 {
        legal_overload__i32_0(a)
    } else if d == 1 {
        legal_overload__i32_1_n(a)
    } else if /* potentially more complicated checks, e.g. `d == 2 && shp[0] == shp[1]` for a pattern `i32[n,n]` */ {
        ...
    } else {
        error!
    }
}
```

This also motivates why the return value must have the same base type.
As the wrapper function must return one single base type.

In practise, it will likely not be explicitly implemented like this in the AST.
(Though the generated C code will likely mirror this idea.)

## Ambiguity

It is required that there is no ambiguity between cases.
For any input shapes, only one overload may be the most specific.

Thus, the following is not allowed.

```imp
fn foo(i32[n] a) -> i32 { ... }

fn foo(i32[n,d2:shp2] a) -> i32 { ... }
```

As for a vector input, both cases may apply.

The solution is to require d2 to be at least one, ensuring a fixed order.

```imp
fn foo(i32[n] a) -> i32 { ... }

fn foo(i32[n,d2>0:shp2] a) -> i32 { ... }
```

For now, we will just assume that such ambiguity does not exist.
This is an additional check we could add later without having to change other parts of the code.

## Directed Acyclic Graph

Overloads may also 'diverge', which is immediately obvious for the addition cases.

```imp
fn +(i32 a, i32 b) -> i32 { ... } // (1)

fn +(i32[d>0:shp] a, i32 b) -> i32[d>0:shp] { ... } // (2)

fn +(i32 a, i32[d>0:shp] b) -> i32[d>0:shp] { ... } // (3)

fn +(i32[d>0:shp] a, i32[d>0:shp] b) -> i32[d>0:shp] { ... } // (4)
```

As for the 'order' of these overloads, we have:
* (1) < (2) < (4)
* (1) < (3) < (4)
But no relation between (2) and (3).

Although this theoretically defines a DAG:

```
  (1)
  / \
(2) (3)
  \ /
  (4)
```

Note that we can push this flat, and can just store this as a `Vec` in our AST.
Since these two cases diverge, it does not matter whether we store this as `vec![(1), (2), (3), (4)]` or `vec![(1), (3), (2), (4)]`.
And similarly when generating the C wrapper function conditionals: whether we check the condition of (2) or (3) first will never matter.

Thus, after grouping overloads based on their base types and argument counts, we can sort the overloads-vector of each 'group' with a comparison function between pairs of overloads.

E.g., we have:
```
cmp(i32, i32[n]) == -1

cmp(i32[n], i32) == 1

cmp(i32[4,n], i32) == 1

cmp(i32[3], i32[4]) == 0

cmp(i32[n], i32[n,d>0:shp]) == -1
```
