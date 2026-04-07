Continuing from [traits-or-generics](traits-or-generics.md)

If we have a module A:

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

And module B defines a complex type (not yet possible, but assume it is) and a scalar addition of complex.

```
// Some typedef for `complex32`

fn add(complex32 a, complex32 b) -> complex32 {
    // This may look something like
    return complex32{ .real = a.real + b.real, .imag = a.imag + b.imag };
}
```

Since module A implements `fn add<T>(T[d:shp,n] a, T[d:shp,n] b)`, should that mean that this also defines an array-array case for complex?
The user might want different behaviour, in which case we should not do this.
Additionally, it does not make sense for a module with no knowledge of complex to define complex behaviour.
But on the other hand, how would they write this down? The function already exists so we cannot overwrite it.
This again calls the need for traits...

We have gone full circle from [modules-and-traits](modules-and-traits.md)
