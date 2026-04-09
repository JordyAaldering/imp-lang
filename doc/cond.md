# Conditionals

We support an extended conditional expression.

```imp
res = if {
    b = a + a;
    c = b - 1;
    c
} else {
    5
};
```

Similarly to tensor comprehensions, each branch must end with an expression, which is the returned value.

For now, we return only a single value.

For now, we disallow side effects.
Namely, in the true-branch, b and c must not yet exist.
Or more generally, any variables defined in either conditional scope must be uniquely named.

## Allowing 'side effects' (future work)

In the future, it is possible to allow this with relatively simply rewrite rules.
Say `b` would be a variable in the outer scope, then we can rewrite the conditional as:

```imp
res, b = if {
    b2 = a + a;
    c = b2 - 1;
    (c, b2)
} else {
    (5, b)
};
```
