/// Functions may be overloaded, e.g.
///
/// ```
/// foo(u32 x) -> u32
///
/// foo(u32[n] x) -> u32[n]
///
/// foo(u32[n] x, u32[n] y) -> u32[n]
///
/// foo(u32[n,m] x) -> u32[n,m]
/// ```
///
/// The number of return values must stay the same.
/// It is not possible to dispatch based on the return type.
///
/// C does not have overloading, so we need a way to consistently rename functions.
/// For this, we append each argument type to the end of the function's name.
///
/// Not only the base type, but also the type pattern.
///
/// ```
/// foo__u32_0(u32 x) -> u32
///
/// foo__u32_n(u32[n] x) -> u32[n]
///
/// foo__u32_n__u32_n(u32[n] x, u32[n] y) -> u32[n]
///
/// foo__u32_n__u32_m(u32[n] x, u32[m] y) -> u32[n]
///
/// foo__u32_n_m(u32[n,m] x) -> u32[n,m]
/// ```
///
/// Although in SaC foo__u32_n__u32_n and foo__u32_n__u32_m would be considered the same, we do allow it here.
/// This is not possible in general: crucially, it requires some ordering in the functions.
/// Here, foo__u32_n__u32_n is a more specific overload of foo__u32_n__u32_m.
/// Thus, foo__u32_n__u32_n < foo__u32_n__u32_m
///
/// For example, this is not allowed for bar(u32[o:oshp,i:ishp] a, u32[o:oshp] b) and bar(u32[o:oshp] a, u32[o:osho,i:ishp] b).
/// As, in the case where the shapes of a and b are the same, and thus i == 0, both overloads would be equally specific.
/// Namely, there must be a clear ordering
pub struct RenameFundefs {

}
