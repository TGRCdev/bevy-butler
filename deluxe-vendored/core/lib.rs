//! # Deluxe Core
//!
//! Core functions and traits shared between [`deluxe`](https://docs.rs/deluxe) and
//! [`deluxe_macros`](https://docs.rs/deluxe-macros).
//!
//! This crate is used by [`deluxe_macros`](https://docs.rs/deluxe-macros) to parse its own
//! attributes. Code generated by its derive macros also references items from this crate
//! re-exported into [`deluxe`](https://docs.rs/deluxe). The functions in [`parse_helpers`] are used
//! internally by the derive macros, but can also be used for convenience when manually
//! implementing any of the parsing traits.
//!
//! See the documentation for the [`deluxe`](https://docs.rs/deluxe) crate for a high-level overview
//! of how Deluxe works.

#![deny(missing_docs)]
#![deny(unsafe_code)]
#![allow(warnings)] // not my code not my warnings

#[cfg(feature = "proc-macro")]
extern crate proc_macro;

mod parse_attributes;
pub mod parse_helpers;
mod parse_meta;
mod small_string;
mod util;
pub mod validations;
pub mod with;

pub use parse_attributes::*;
pub use parse_meta::*;
pub use util::*;

pub use proc_macro2::Span;

#[doc(hidden)]
pub use syn;
#[doc(hidden)]
pub use {
    std::{
        borrow::Borrow,
        collections::HashMap,
        fmt,
        hash::{Hash, Hasher},
        ops, primitive, stringify,
    },
    AsRef, Clone, Default, Eq, IntoIterator, Iterator, Option, PartialEq,
};
