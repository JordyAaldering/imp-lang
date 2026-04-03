//! Generates contraints on function types based on the type patterns.
//!
//! 1) GenTPC (type[<num>, …rest] v, dim, dep, env)
//!     = GenTPC (type[rest] v, dim+1, dep,
//!         env + { v: (dep, <num> == shape(v)[dim]) })
//!
//! 2) GenTPC (type[., …rest] v, dim, dep, env)
//!     = GenTPC (type[rest] v, dim+1, dep, env)
//!
//! 3) GenTPC (type[+, …rest] v, dim, dep, env)
//!     = GenTPCrhs (type[rest] v, dim(v)-1, {}, env)
//!
//! 4) GenTPC (type[*, …rest] v, dim, dep, env)
//!     = GenTPCrhs (type[rest] v, dim(v)-1, {}, env)
//!
//! 5) GenTPC (type[id, …rest] v, dim, dep, env)
//!     = GenTPC (type[rest] v, dim+1, dep,
//!         env + { id: (dep, shape(v)[dim]) })
//!
//! 6) GenTPC (type[<num>:shp, …rest] v, dim, dep, env)
//!     = GenTPC (type[rest] v, dim+num, dep,
//!         env + { shp: (dep, take(num, drop(dim, shape(v)))) })
//!
//! 7) GenTPC (type[id:shp, …rest] v, dim, dep, env)
//!     = GenTPC (type[rest] v, dim+id, dep+{id},
//!         env + { id: (dep+vdots\id, dim(v) - fdots - sum(vdots\id)) }
//!             + { shp: (dep+{id}, take(id, drop(dim, shape(v)))) })
//!
//! For example, for foo(u32[n] a, u32[n] b), we generate n = shape(a)[0], and a constraint that shape(b)[0] == n.
//! Then, we only dispatch to this function if that is the case.
//! And in the generated wrapper function (both on the Rust and C side),
//! the conditional checks if this condition holds, continuing with the other cases if not.

pub struct GenTPConstraints;