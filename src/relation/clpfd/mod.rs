//! # CLP(FD)
//! Proto-vulcan implements finite-domain constraints. For disequality, a `diseqfd(x, y)`-relation
//! must be used instead of `x != y`. Other supported CLP(FD) constraints are: `distinctfd`, `ltefd`
//! `ltfd`, `plusfd`, `minusfd` and `timesfd`. Domains are assigned to variables with `infd` or
//! `infdrange`. See `n-queens`-example for code using finite-domain constraints.
//!

pub mod diseqfd;
pub mod distinctfd;
pub mod domfd;
pub mod infd;
pub mod ltefd;
pub mod ltfd;
pub mod minusfd;
pub mod plusfd;
pub mod timesfd;
