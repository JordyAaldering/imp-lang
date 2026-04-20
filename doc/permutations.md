# Permutations

Idea Thomas.

```imp
{ Q[i] -> expr(i) | [i] < [n] }
```

Which is conceptually similar to:

```imp
{ expr(Q_inv[i]) | [i] < [n] }
```

(With the inverse `Q_inv == Q^{-1}`, which may be difficult to compute/come up with.)

This is a useful notation when we want to generate code like:

```C
for (i = 0; i < n; i++) {
    X[Q[i]] = expr(i);
}
```

Which can be important for performance reasons.
Because in some cases we are fine writing to `X` in a non-linear fashion,
but we want reads in `expr` to happen linearly in memory.

## Example

```imp
fn apply_reverse(i32[n,m] arr) -> i32[n,m] {
    Q = { [n-i-1, m-j-1] | [i,j] < [n,m] }; // Q :: i32[n,m] -> i32[n,m]
    { Q[i,j] -> expr(arr[i,j]) | [i,j] < [n,m] }
}
```

Which generates:

```C
for (i = 0; i < n; i++) {
    for (j = 0; i < m; j++) {
        // `arr` is indexed in order,
        // but the result is written into Q in reversed order
        X[Q[i,j]] = expr(arr[i,j]);
    }
}
```

## Requirements

Dit vereist dat het domein en bereik het zelfde zijn.
For example, for a function such as the transpose this does not make sense, as the shape permutes from `[n,m]` to `[m,n]`.
This is notationally hard to define, and anyway in these cases one would just write `iv -> arr[P(iv)]`.
E.g.,

```imp
fn transpose(i32[d:shp] arr) -> i32[d:shp_t] {
    shp_t = reverse(shp);
    { arr[reverse(iv)] | iv < shp_t }
}
```
