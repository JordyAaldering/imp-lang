# return keyword?

The question is whether we should have a `return` keyword, or whether we just require that function bodies end in an expression,
similarly to what is possible in Rust.

The reason why I suggest dropping the `return` keyword is twofold:
1) It makes it obvious that early returning is not possible, as this simply cannot be expressed.
2) It enables us to parse function bodies the same as tensor and conditional bodies.
   (Tensors and conditionals are also a chain of statements followed by an expression.)
   In tensors and conditionals a return keyword is nonsensical, so we might as well keep all bodies the same.
