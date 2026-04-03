/// At this point, each wrapper contains functions with the same name.
///
/// Overloaded functions have a few constraints.
///
/// 1) the number of return values must be the same.
///
/// 2) functions cannot be overloaded based on the type of return values, only based on the type of arguments.
///
/// 3) there must be a strict ordering in the 'preciceness' of overloads that only differ in their type patterns.
///    Although in SaC foo__u32_n__u32_n and foo__u32_n__u32_m would be considered the same, we do allow it here.
///    This is not possible in general: crucially, it requires some ordering in the functions.
///    Here, foo__u32_n__u32_n is a more specific overload of foo__u32_n__u32_m.
///    Thus, foo__u32_n__u32_n < foo__u32_n__u32_m
///
///    For example, this is not allowed for bar(u32[o:oshp,i:ishp] a, u32[o:oshp] b) and bar(u32[o:oshp] a, u32[o:osho,i:ishp] b).
///    As, in the case where the shapes of a and b are the same, and thus i == 0, both overloads would be equally specific.
///    Namely, there must be a clear ordering
pub struct CheckWrappers {

}
