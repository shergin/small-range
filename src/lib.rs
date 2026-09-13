#![no_std]
#![doc = include_str!("../README.md")]
#![warn(missing_docs)]
#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(feature = "alloc")]
extern crate alloc;

mod small_range;

#[cfg(feature = "serde")]
mod serde_impls;

pub use small_range::{OutOfRange, SmallRange, SmallRangeStorage};

#[cfg(test)]
#[path = "tests/small_range_tests.rs"]
mod tests;
