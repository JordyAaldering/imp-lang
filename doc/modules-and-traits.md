The combination of modules and traits (or even overloading) presents a conceptual problem.

Say we have module A that implements `Add` for the scalar-scalar and array-array case.
It also has a function that operates on arrays. In particular, a vector.
It might look something like:

```
impl Add<T: Num, [], [], []> {
    fn +(T a, T b) -> T {
        return @addSxS(a, b);
    }
}

impl Add<T: Num, [d>0:shp], [d>0:shp], [d>0:shp]> {
    fn +(T[d>0:shp] a, T[d>0:shp] b) -> T[d>0:shp] {
        return { @addSxS(@selVxA(iv, a), @selVxA(iv, b)) | iv < shp };
    }
}

fn foo(f32[n] vec) -> f32[n] {
    return vec + vec;
}
```

The addition will dispatch to the array-array case.
But what if now in module B, we overload the function for the vector-vector case.

```
impl Add<T: Num, [n], [n], [n]> {
    fn +(T[n] a, T[n] b) -> T[n] {
        return { @addSxS(@selVxA(iv, a), @selVxA(iv, b)) | iv < shp };
    }
}
```

Does that mean that `foo`, in module A now behaves differently?
That would mean that one module can indirectly change the behaviour of another module, that should not be possible!!!

Prohibiting overloading across modules in not a good solution.
If one adds a new type, e.g. complex numbers, they would want to overload + for it.

I think the better solution is to _allow_ overloading across modules, but only for overloadings with different base types.
So if module A defines `add(f32, f32) -> f32`, then module B may **not** define `add(f32[n], f32[n]) -> f32[n]`, but it _can_ define `add(f32, complex32) -> complex32`.

This should be a simple type checking problem; we ignore it for now and assume users do not do this.
