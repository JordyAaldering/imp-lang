# Fold syntax

```imp
x = @fold(neutral, foldfun, { expr(iv) | 0*ub <= iv < ub });

where

neutral :: T[d:shp]
foldfun :: (T[d:shp], T[d:shp]) -> T[d:shp]
expr(iv) :: T[d:shp]
x :: T[d:shp]
```

```
fn sum(i32[d:shp] arr) -> i32 {
    return @fold(0i32, +, { arr[iv] | 0*shp <= iv < shp });
    //                 |     |
    //                 |     Scalar selection (result is i32)
    //                 Scalar-scalar addition
}

fn sumlast(i32[d:shp,n] arr) -> i32[n] {
    return @fold(zeros(n), +, { arr[iv] | 0*shp <= iv < shp });
    //                     |     |
    //                     |     Planar selection (result is i32[n])
    //                     Vector-vector addition
}
```

Is some cases it is useful to have partial application.
This is not actually possible in our language, but we can mimic it.

For this, we use a special notation for the fold function: `foldfun(x,_,y,_,z)`.
Namely, it is written like a function application (where x, y, and z are local variables),
and where the two underscores are rewritten by the to-be-folded arguments.

Thus, writing `+(_, _)` means the same as just writing `+`.

This is useful in for example the convex hull algorithm, where we want to check which of any two points `u` and `v` is farthest from the line between two locally defined points `p` and `q`.

```
fn frt(point p, point q, point u, point v) -> point {
    return /* a difficult comparison */ ? u : v;
}

fn farthest(point p, point q, point[n] ps) -> point {
    neutral = p; // p has distance 0 so will never be the farthest
    return @fold(neutral, frt(p, q, _, _), { ps[iv] | [0] <= iv < [n] });
}
```

Since we don't actually have partial application, the underscores are replaced by the actual arguments during code generation (in the generated loop).
Thus, at that point they are simply function applications.
