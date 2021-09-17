#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod always;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod append;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod cons;

#[cfg(feature = "core")]
#[doc(hidden)]
pub mod diseq;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod distinct;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod empty;

#[cfg(feature = "core")]
#[doc(hidden)]
pub mod eq;

#[cfg(feature = "core")]
#[doc(hidden)]
pub mod fail;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod first;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod member1;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod member;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod never;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod permute;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod rember;

#[cfg(feature = "extras")]
#[doc(hidden)]
pub mod rest;

#[cfg(feature = "core")]
#[doc(hidden)]
pub mod succeed;

// CLP(FD)
#[cfg(feature = "clpfd")]
#[doc(hidden)]
pub mod diseqfd;

#[cfg(feature = "clpfd")]
#[doc(hidden)]
pub mod distinctfd;

#[cfg(feature = "clpfd")]
#[doc(hidden)]
pub mod domfd;

#[cfg(feature = "clpfd")]
#[doc(hidden)]
pub mod infd;

#[cfg(feature = "clpfd")]
#[doc(hidden)]
pub mod ltefd;

#[cfg(feature = "clpfd")]
#[doc(hidden)]
pub mod ltfd;

#[cfg(feature = "clpfd")]
#[doc(hidden)]
pub mod minusfd;

#[cfg(feature = "clpfd")]
#[doc(hidden)]
pub mod plusfd;

#[cfg(feature = "clpfd")]
#[doc(hidden)]
pub mod timesfd;

// CLP(Z)
#[cfg(feature = "clpz")]
#[doc(hidden)]
pub mod plusz;
#[cfg(feature = "clpz")]
#[doc(hidden)]
pub mod timesz;

#[cfg(feature = "core")]
#[doc(inline)]
pub use diseq::diseq;

#[cfg(feature = "core")]
#[doc(inline)]
pub use eq::eq;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use always::always;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use append::append;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use cons::cons;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use distinct::distinct;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use empty::empty;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use first::first;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use member1::member1;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use member::member;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use never::never;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use permute::permute;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use rember::rember;

#[cfg(feature = "extras")]
#[doc(inline)]
pub use rest::rest;

#[cfg(feature = "core")]
#[doc(inline)]
pub use fail::fail;

#[cfg(feature = "core")]
#[doc(inline)]
pub use succeed::succeed;

#[cfg(feature = "clpfd")]
#[doc(inline)]
pub use diseqfd::diseqfd;

#[cfg(feature = "clpfd")]
#[doc(inline)]
pub use distinctfd::distinctfd;

#[cfg(feature = "clpfd")]
#[doc(inline)]
pub use infd::infd;

#[cfg(feature = "clpfd")]
#[doc(inline)]
pub use infd::infdrange;

#[cfg(feature = "clpfd")]
#[doc(inline)]
pub use ltefd::ltefd;

#[cfg(feature = "clpfd")]
#[doc(inline)]
pub use ltfd::ltfd;

#[cfg(feature = "clpfd")]
#[doc(inline)]
pub use minusfd::minusfd;

#[cfg(feature = "clpfd")]
#[doc(inline)]
pub use plusfd::plusfd;

#[cfg(feature = "clpfd")]
#[doc(inline)]
pub use timesfd::timesfd;

#[cfg(feature = "clpz")]
#[doc(inline)]
pub use plusz::plusz;

#[cfg(feature = "clpz")]
#[doc(inline)]
pub use timesz::timesz;
