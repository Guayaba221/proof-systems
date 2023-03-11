// #![deny(missing_docs)]

//! Snarky is the front end to kimchi, allowing users to write their own programs and convert them to kimchi circuits.

pub mod api;
pub mod asm;
pub mod boolean;
pub mod checked_runner;
pub mod constants;
pub mod constraint_system;
pub mod cvar;
pub mod errors;
pub(crate) mod poseidon;
pub mod traits;
pub mod union_find;

#[cfg(test)]
mod tests;

/// A handy module that you can import the content of to easily use snarky.
pub mod prelude {
    use super::*;
    pub use crate::loc;
    pub use boolean::Boolean;
    pub use checked_runner::RunState;
    pub use cvar::FieldVar;
    pub use traits::SnarkyType;
}
