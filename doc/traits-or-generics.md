It is unclear how we should write down traits.

```
impl Add<T: Num, [], [], []> {
    fn +(T a, T b) -> T {
        return @addSxS(a, b);
    }
}

impl Add<T: Num, [d>0:shp], [d>0:shp], [d>0:shp]> {
    fn +(T[d>0:shp] a, T[d>0:shp] b) -> T[d>0:shp] {
        return { a[iv] + b[iv] | iv < shp };
    }
}

```

The array-array case calls the scalar-scalar case.
But how do we specify that the scalar case implements addition?
We cannot write a recursive defintion `impl Add<T: Num + Add>`.

Additionally, adding all shape parameters as generic arguments is a bit of a pain.
Perhaps we should stick to overloading.

But this also poses a problem; internally we would have traits like `Num`, that users can extend with their own types, e.g. complex numbers.
Perhaps instead of types/traits, generic parameters can define which overloaded function is required.

```
fn add<T: add>(T[d>0:shp] a, T[d>0:shp] b) -> T[d>0:shp] {
    return { a[iv] + b[iv] | iv < shp };
}
```

But what does `T: add` even mean: the function takes two arguments!
What if we would want to write down that scalar-array addition is implemented.
`T: add(T[d:shp], T[d:shp])`?
But then what is the preceding `T: ` even used for?

```
fn add<T>(T[d>0:shp] a, T[d>0:shp] b) -> T[d>0:shp]
where
    T: add(T, T[d:shp]) -> T[d:shp]
{
    return a[0*shp] + b;
}
```

What if we for some reason want the ability to add any two numbers?

```
fn add<T, U>(T[d>0:shp] a, U[d>0:shp] b) -> T[d>0:shp]
where
    T, U: add(T, U) -> T
{
    return { a[iv] + b[iv] | iv < shp };
}
```

But this notation is a bit weird.
The real way someone would implement this is with a `ToT` and `FromT` function.

```
fn add<T, U>(T[d>0:shp] a, U[d>0:shp] b) -> T[d>0:shp]
where
    T: add(T, T) -> T
    U: To<T>(U) -> T
{
    return { a[iv] + To<T>(b[iv]) | iv < shp };
}
```

But that is probably overkill for our purposes.

For now, I would stick with:

```
fn add<T>(T[d>0:shp] a, T[d>0:shp] b) -> T[d>0:shp]
where
    add(T, T) -> T
{
    return { a[iv] + b[iv] | iv < shp };
}
```

Where we make shapes explicit. E.g.

```
fn add<T>(T[d:shp,n] a, T[d:shp,n] b) -> T[d:shp,n]
where
    add(T[n], T[n]) -> T[n]
{
    return { a[iv] + b[iv] | iv < shp };
}
```

This does mean that we must manually define addition for all scalar types, but at least not for scalar-array combinations.
For now, I think that is good enough.

```
fn add(u32 a, u32 b) -> u32 { return @addSxS(a, b); }
fn add(f32 a, f32 b) -> f32 { return @addSxS(a, b); }

fn add<T>(T[d:shp,n] a, T[d:shp,n] b) -> T[d:shp,n]
where
    add(T[n], T[n]) -> T[n]
{
    return { a[iv] + b[iv] | iv < shp };
}
```
