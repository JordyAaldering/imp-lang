/// Not all patterns that can be constructed from the grammar are actually resolvable.
/// This is mainly due to variable-rank patterns (d:shp).
///
/// One such variable-rank pattern is always okay.
///    u32[d:shp], u32[x,d:shp], u32[u,w,d:shp,u]
/// However, multiples are not allowed in general
///    u32[d1:shp1,d2:shp2]
/// As it is unclear how to resolve the values of d1 and d2
/// For a matrix, is d1 == 0 and d2 == 2, d1 == d2 == 1, or d1 == 2 and d2 == 0?
///
/// The only case where this is allowed, is when d1 or d2 is explicitly constrained by another pattern
///   foo(usize d1, u32[d1:shp1,d2:shp2] arr)
/// or even
///   foo(usize[d1] vec, u32[d1:shp1,d2:shp2] arr)
/// Now, we know that d2 == dim(arr) - d1, so the pattern is always resolvable,
/// and cases where the rank is too small will be rejected
#[allow(dead_code)]
pub struct CheckTypePatterns;
