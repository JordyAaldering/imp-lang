# Overloading

## Modules

Overloading across module boundaries is problematic.

If one defines a new overload in another module, that might change the behaviour in the original module!

Thus, we only allow overloading within a single module.

## Ordering

We require that overloads are fully disjoint.

In other words, for any input, only one overload may possibly match.

E.g., we disallow:

```imp
fn foo(i32 x) -> i32;

fn foo(i32[d:shp] x) -> i32[d:shp];
```

As for a scalar input, `i32[d:shp]` still applies (for d = 0).

In this case, one should write:

```imp
fn foo(i32 x) -> i32;

fn foo(i32[d>0:shp] x) -> i32[d>0:shp];
```

Then, only one possible overload can match.

### Partial overlap

With multiple parameters, it is possible that some but not all arguments are the same.

```imp
fn foo(i32 x, i32 y) -> i32;

fn foo(i32 x, i32[d>0:shp] y) -> i32[d>0:shp];
```

At least one argument must be non-equal, the rest may be the same.
