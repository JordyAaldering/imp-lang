The AST has been deliberately constructed to be as safe as possible.

E.g. by requiring nested expressions to be only identifiers after the flattening phase, or that dispatches should exist.

Often, AST-type-modifying traversals occur in grouped blocks.
E.g., flattening and SSA. Or type checking and dispatching.

Having a different ASTconfig for each of these can be a frustrating to work with.
Thus, we allow for a little bit of unsafety during these grouped rewrites.
E.g., flattening might not change the actual ASTconfig, and then the SSA traversal simply unwraps all nested
expressions into identifiers, and only then changing the ASTconfig.

Concretely, we allow some unsafety, but only when this happens directy in order with another type-changing traversal.

In this way, we balance type safety with practicality.
