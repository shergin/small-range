#![no_std]
#![doc = include_str!("../README.md")]
//!
//! # Quick Start
//!
//! ```rust
//! use small_range::SmallRange;
//!
//! // Create a range from 10 to 20 (defaults to u64 storage)
//! let range = SmallRange::new(10u64, 20u64);
//!
//! assert_eq!(range.start(), 10u64);
//! assert_eq!(range.end(), 20u64);
//! assert_eq!(range.len(), 10);
//!
//! // Convert to a standard Range
//! assert_eq!(range.to_range(), 10u64..20u64);
//!
//! // Iterate over the range
//! for i in &range {
//!     println!("{}", i);
//! }
//! ```
//!
//! # Storage Type Support
//!
//! `SmallRange` supports `u16`, `u32`, `u64`, and `usize` storage types.
//! Each stores start and length in half the bits, achieving 50% space savings:
//!
//! ```rust
//! use small_range::SmallRange;
//! use core::mem::size_of;
//!
//! // SmallRange<u16>: 2 bytes (vs 4 bytes for Range<u16>)
//! let r16 = SmallRange::<u16>::new(0, 100);
//! assert_eq!(size_of::<SmallRange<u16>>(), 2);
//!
//! // SmallRange<u32>: 4 bytes (vs 8 bytes for Range<u32>)
//! let r32 = SmallRange::<u32>::new(0, 1000);
//! assert_eq!(size_of::<SmallRange<u32>>(), 4);
//!
//! // SmallRange<u64>: 8 bytes (vs 16 bytes for Range<u64>) - default
//! let r64 = SmallRange::<u64>::new(0, 1_000_000);
//! assert_eq!(size_of::<SmallRange<u64>>(), 8);
//!
//! // SmallRange<usize>: convenient for indexing
//! let r_usize = SmallRange::<usize>::new(0, 100);
//! ```
//!
//! # Memory Efficiency
//!
//! `SmallRange` uses niche optimization, so `Option<SmallRange<T>>` is the same size
//! as `SmallRange<T>` itself.
//!
//! ```rust
//! use small_range::SmallRange;
//! use core::mem::size_of;
//!
//! assert_eq!(size_of::<SmallRange<u64>>(), size_of::<Option<SmallRange<u64>>>());
//! assert_eq!(size_of::<SmallRange<u32>>(), size_of::<Option<SmallRange<u32>>>());
//! assert_eq!(size_of::<SmallRange<u16>>(), size_of::<Option<SmallRange<u16>>>());
//! ```

mod small_range;

pub use small_range::{SmallRange, SmallRangeStorage};

#[cfg(test)]
#[path = "tests/small_range_tests.rs"]
mod tests;
