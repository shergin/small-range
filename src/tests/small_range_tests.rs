extern crate alloc;
extern crate std;

use crate::SmallRange;
use alloc::format;
use alloc::vec;
use alloc::vec::Vec;
use core::mem::size_of;
use core::ops::Range;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// =============================================================================
// Memory Layout Tests
// =============================================================================

#[test]
fn test_space_savings() {
    // SmallRange<T> is half the size of Range<T>
    assert_eq!(size_of::<SmallRange<u16>>(), 2);
    assert_eq!(size_of::<SmallRange<u32>>(), 4);
    assert_eq!(size_of::<SmallRange<u64>>(), 8);
    assert_eq!(size_of::<SmallRange<usize>>(), 8);

    assert_eq!(size_of::<Range<u16>>(), 4);
    assert_eq!(size_of::<Range<u32>>(), 8);
    assert_eq!(size_of::<Range<u64>>(), 16);
    assert_eq!(size_of::<Range<usize>>(), 16);

    // Option<SmallRange<T>> has same size as SmallRange<T> (niche optimization)
    assert_eq!(size_of::<Option<SmallRange<u16>>>(), 2);
    assert_eq!(size_of::<Option<SmallRange<u32>>>(), 4);
    assert_eq!(size_of::<Option<SmallRange<u64>>>(), 8);
    assert_eq!(size_of::<Option<SmallRange<usize>>>(), 8);

    // Option<Range<T>> requires extra space for discriminant (no niche optimization)
    assert_eq!(size_of::<Option<Range<u16>>>(), 6);
    assert_eq!(size_of::<Option<Range<u32>>>(), 12);
    assert_eq!(size_of::<Option<Range<u64>>>(), 24);
    assert_eq!(size_of::<Option<Range<usize>>>(), 24);

    // 3x space savings for Option<Range<u64/usize>> (24 bytes -> 8 bytes)
    assert_eq!(
        size_of::<Option<Range<u64>>>() / size_of::<Option<SmallRange<u64>>>(),
        3
    );
}

// =============================================================================
// Roundtrip Encoding/Decoding Tests
// =============================================================================

macro_rules! test_roundtrip_for_type {
    ($name:ident, $ty:ty, $max_val:expr) => {
        #[test]
        fn $name() {
            let test_values: &[$ty] = &[0, 1, 2, 10, 100, $max_val / 2, $max_val - 1, $max_val];

            for &start in test_values {
                for &len in test_values {
                    if start.checked_add(len).is_none() {
                        continue; // Skip overflow cases
                    }
                    let end = start + len;

                    let range = SmallRange::<$ty>::new(start, end);
                    assert_eq!(
                        range.start(),
                        start,
                        "start mismatch for {}..{}",
                        start,
                        end
                    );
                    assert_eq!(range.end(), end, "end mismatch for {}..{}", start, end);
                    assert_eq!(
                        range.len(),
                        len as usize,
                        "len mismatch for {}..{}",
                        start,
                        end
                    );
                }
            }
        }
    };
}

test_roundtrip_for_type!(test_roundtrip_u16, u16, 254);
test_roundtrip_for_type!(test_roundtrip_u32, u32, 65534);
test_roundtrip_for_type!(test_roundtrip_u64, u64, 0xFFFF_FFFE);

// =============================================================================
// Boundary Value Tests
// =============================================================================

#[test]
fn test_maximum_values_u16() {
    // Max start with zero length
    let r = SmallRange::<u16>::new(254, 254);
    assert_eq!(r.start(), 254);
    assert_eq!(r.end(), 254);
    assert!(r.is_empty());

    // Max length from zero
    let r = SmallRange::<u16>::new(0, 254);
    assert_eq!(r.start(), 0);
    assert_eq!(r.end(), 254);
    assert_eq!(r.len(), 254);

    // Max start + max length would overflow, test just under
    let r = SmallRange::<u16>::new(100, 200);
    assert_eq!(r.start(), 100);
    assert_eq!(r.end(), 200);
    assert_eq!(r.len(), 100);
}

#[test]
fn test_maximum_values_u32() {
    // Max start with zero length
    let r = SmallRange::<u32>::new(65534, 65534);
    assert_eq!(r.start(), 65534);
    assert_eq!(r.end(), 65534);
    assert!(r.is_empty());

    // Max length from zero
    let r = SmallRange::<u32>::new(0, 65534);
    assert_eq!(r.start(), 0);
    assert_eq!(r.end(), 65534);
    assert_eq!(r.len(), 65534);
}

#[test]
fn test_maximum_values_u64() {
    let max: u64 = 0xFFFF_FFFE;

    // Max start with zero length
    let r = SmallRange::<u64>::new(max, max);
    assert_eq!(r.start(), max);
    assert_eq!(r.end(), max);
    assert!(r.is_empty());

    // Max length from zero
    let r = SmallRange::<u64>::new(0, max);
    assert_eq!(r.start(), 0);
    assert_eq!(r.end(), max);
    assert_eq!(r.len(), max as usize);
}

// =============================================================================
// Empty Range Tests
// =============================================================================

#[test]
fn test_empty_range() {
    let r = SmallRange::<u32>::new(0, 0);
    assert!(r.is_empty());
    assert_eq!(r.len(), 0);
    assert_eq!(r.start(), 0);
    assert_eq!(r.end(), 0);

    let r = SmallRange::<u32>::new(100, 100);
    assert!(r.is_empty());
    assert_eq!(r.len(), 0);
    assert_eq!(r.start(), 100);
    assert_eq!(r.end(), 100);
}

#[test]
fn test_single_element_range() {
    let r = SmallRange::<u32>::new(42, 43);
    assert!(!r.is_empty());
    assert_eq!(r.len(), 1);
    assert_eq!(r.start(), 42);
    assert_eq!(r.end(), 43);
}

// =============================================================================
// Default Tests
// =============================================================================

#[test]
fn test_default() {
    let r = SmallRange::<u32>::default();
    assert!(r.is_empty());
    assert_eq!(r.start(), 0);
    assert_eq!(r.end(), 0);
    assert_eq!(r.len(), 0);
}

// =============================================================================
// to_range() Tests
// =============================================================================

#[test]
fn test_to_range() {
    let small = SmallRange::<u32>::new(10, 20);
    let std_range = small.to_range();
    assert_eq!(std_range, 10..20);

    let empty = SmallRange::<u32>::new(5, 5);
    assert_eq!(empty.to_range(), 5..5);
}

// =============================================================================
// Iterator Tests
// =============================================================================

#[test]
fn test_iteration() {
    let r = SmallRange::<u32>::new(5, 10);
    let collected: Vec<_> = r.into_iter().collect();
    assert_eq!(collected, vec![5, 6, 7, 8, 9]);
}

#[test]
fn test_iteration_empty() {
    let r = SmallRange::<u32>::new(5, 5);
    let collected: Vec<_> = r.into_iter().collect();
    assert!(collected.is_empty());
}

#[test]
fn test_iteration_single() {
    let r = SmallRange::<u32>::new(42, 43);
    let collected: Vec<_> = r.into_iter().collect();
    assert_eq!(collected, vec![42]);
}

#[test]
fn test_iteration_by_ref() {
    let r = SmallRange::<u32>::new(0, 3);
    let collected: Vec<_> = (&r).into_iter().collect();
    assert_eq!(collected, vec![0, 1, 2]);

    // Can iterate again since we borrowed
    let collected2: Vec<_> = (&r).into_iter().collect();
    assert_eq!(collected2, vec![0, 1, 2]);
}

// =============================================================================
// Debug Formatting Tests
// =============================================================================

#[test]
fn test_debug_format() {
    let r = SmallRange::<u32>::new(10, 20);
    let debug_str = format!("{:?}", r);
    assert!(debug_str.contains("SmallRange"));
    assert!(debug_str.contains("start"));
    assert!(debug_str.contains("end"));
    assert!(debug_str.contains("10"));
    assert!(debug_str.contains("20"));
}

// =============================================================================
// Equality and Hash Tests
// =============================================================================

#[test]
fn test_equality() {
    let a = SmallRange::<u32>::new(10, 20);
    let b = SmallRange::<u32>::new(10, 20);
    let c = SmallRange::<u32>::new(10, 21);

    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn test_hash_consistency() {
    fn hash<T: Hash>(t: &T) -> u64 {
        let mut hasher = DefaultHasher::new();
        t.hash(&mut hasher);
        hasher.finish()
    }

    let a = SmallRange::<u32>::new(10, 20);
    let b = SmallRange::<u32>::new(10, 20);

    assert_eq!(hash(&a), hash(&b));
}

#[test]
fn test_copy_clone() {
    let original = SmallRange::<u32>::new(10, 20);
    let copied = original; // Copy
    let cloned = original.clone(); // Clone

    assert_eq!(original, copied);
    assert_eq!(original, cloned);
}

// =============================================================================
// try_new() Tests
// =============================================================================

#[test]
fn test_try_new_valid() {
    let r = SmallRange::<u32>::try_new(10, 20);
    assert!(r.is_some());
    let r = r.unwrap();
    assert_eq!(r.start(), 10);
    assert_eq!(r.end(), 20);
}

#[test]
fn test_try_new_empty_range() {
    let r = SmallRange::<u32>::try_new(10, 10);
    assert!(r.is_some());
    assert!(r.unwrap().is_empty());
}

#[test]
fn test_try_new_invalid_start_exceeds_end() {
    let r = SmallRange::<u32>::try_new(20, 10);
    assert!(r.is_none());
}

#[test]
fn test_try_new_start_exceeds_capacity() {
    // u16 max start is 254 (LOW_MASK - 1 = 255 - 1)
    let r = SmallRange::<u16>::try_new(255, 255);
    assert!(r.is_none());

    // 254 should work
    let r = SmallRange::<u16>::try_new(254, 254);
    assert!(r.is_some());
}

#[test]
fn test_try_new_length_exceeds_capacity() {
    // u16 max length is 254
    let r = SmallRange::<u16>::try_new(0, 255);
    assert!(r.is_none());

    // 254 should work
    let r = SmallRange::<u16>::try_new(0, 254);
    assert!(r.is_some());
}

// =============================================================================
// contains() Tests
// =============================================================================

#[test]
fn test_contains_basic() {
    let r = SmallRange::<u32>::new(5, 10);

    // Values inside range
    assert!(r.contains(5)); // start is included
    assert!(r.contains(6));
    assert!(r.contains(9)); // last value before end

    // Values outside range
    assert!(!r.contains(4)); // before start
    assert!(!r.contains(10)); // end is excluded
    assert!(!r.contains(11)); // after end
}

#[test]
fn test_contains_empty_range() {
    let r = SmallRange::<u32>::new(5, 5);
    assert!(!r.contains(4));
    assert!(!r.contains(5)); // empty range contains nothing
    assert!(!r.contains(6));
}

#[test]
fn test_contains_single_element() {
    let r = SmallRange::<u32>::new(42, 43);
    assert!(!r.contains(41));
    assert!(r.contains(42));
    assert!(!r.contains(43));
}

#[test]
fn test_contains_zero_start() {
    let r = SmallRange::<u32>::new(0, 5);
    assert!(r.contains(0));
    assert!(r.contains(4));
    assert!(!r.contains(5));
}

// =============================================================================
// overlaps() Tests
// =============================================================================

#[test]
fn test_overlaps_basic() {
    let a = SmallRange::<u32>::new(0, 10);
    let b = SmallRange::<u32>::new(5, 15);
    let c = SmallRange::<u32>::new(10, 20);
    let d = SmallRange::<u32>::new(20, 30);

    // a and b overlap at 5..10
    assert!(a.overlaps(&b));
    assert!(b.overlaps(&a)); // symmetric

    // a ends where c starts - no overlap
    assert!(!a.overlaps(&c));
    assert!(!c.overlaps(&a));

    // b and c overlap at 10..15
    assert!(b.overlaps(&c));
    assert!(c.overlaps(&b));

    // a and d are far apart
    assert!(!a.overlaps(&d));
    assert!(!d.overlaps(&a));
}

#[test]
fn test_overlaps_adjacent() {
    let a = SmallRange::<u32>::new(0, 10);
    let b = SmallRange::<u32>::new(10, 20);

    // Adjacent ranges don't overlap (a.end == b.start but end is exclusive)
    assert!(!a.overlaps(&b));
    assert!(!b.overlaps(&a));
}

#[test]
fn test_overlaps_contained() {
    let outer = SmallRange::<u32>::new(0, 100);
    let inner = SmallRange::<u32>::new(25, 75);

    // Contained ranges overlap
    assert!(outer.overlaps(&inner));
    assert!(inner.overlaps(&outer));
}

#[test]
fn test_overlaps_identical() {
    let a = SmallRange::<u32>::new(10, 20);
    let b = SmallRange::<u32>::new(10, 20);

    // Identical non-empty ranges overlap
    assert!(a.overlaps(&b));
}

#[test]
fn test_overlaps_empty_range() {
    let empty = SmallRange::<u32>::new(10, 10);
    let normal = SmallRange::<u32>::new(5, 15);

    // Empty range doesn't overlap with anything
    assert!(!empty.overlaps(&normal));
    assert!(!normal.overlaps(&empty));

    // Empty range doesn't even overlap with itself
    assert!(!empty.overlaps(&empty));
}

#[test]
fn test_overlaps_single_point_shared() {
    // These share the point at position 10
    let a = SmallRange::<u32>::new(5, 11);
    let b = SmallRange::<u32>::new(10, 15);

    assert!(a.overlaps(&b));
    assert!(b.overlaps(&a));
}

// =============================================================================
// Panic Tests (debug assertions only)
// =============================================================================

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "start must not exceed end")]
fn test_new_panics_on_invalid_range() {
    SmallRange::<u32>::new(20, 10);
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "start+1 exceeds half-width capacity")]
fn test_new_panics_on_start_overflow() {
    SmallRange::<u16>::new(255, 255);
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "length+1 exceeds half-width capacity")]
fn test_new_panics_on_length_overflow() {
    SmallRange::<u16>::new(0, 255);
}

// =============================================================================
// Property-Based Tests
// =============================================================================

mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn roundtrip_u32(start in 0u32..65000, len in 0u32..65000) {
            let end = start.saturating_add(len).min(65534);
            let len = end - start;

            let range = SmallRange::<u32>::new(start, end);
            prop_assert_eq!(range.start(), start);
            prop_assert_eq!(range.end(), end);
            prop_assert_eq!(range.len(), len as usize);
        }

        #[test]
        fn roundtrip_u64(start in 0u64..0xFFFF_0000u64, len in 0u64..0xFFFF_0000u64) {
            let max = 0xFFFF_FFFEu64;
            let end = start.saturating_add(len).min(max);
            let len = end - start;

            let range = SmallRange::<u64>::new(start, end);
            prop_assert_eq!(range.start(), start);
            prop_assert_eq!(range.end(), end);
            prop_assert_eq!(range.len(), len as usize);
        }

        #[test]
        fn try_new_never_panics(start in 0u64..=u64::MAX, end in 0u64..=u64::MAX) {
            // try_new should never panic, just return None for invalid inputs
            let _ = SmallRange::<u64>::try_new(start, end);
        }

        #[test]
        fn try_new_roundtrip(start in 0u32..65000, len in 0u32..65000) {
            let end = start.saturating_add(len).min(65534);

            if let Some(range) = SmallRange::<u32>::try_new(start, end) {
                prop_assert_eq!(range.start(), start);
                prop_assert_eq!(range.end(), end);
            }
        }

        #[test]
        fn contains_matches_std_range(start in 0u32..1000, len in 0u32..1000, value in 0u32..2000) {
            let end = start + len;
            let small = SmallRange::<u32>::new(start, end);
            let std_range = start..end;

            prop_assert_eq!(small.contains(value), std_range.contains(&value));
        }

        #[test]
        fn to_range_roundtrip(start in 0u32..65000, len in 0u32..65000) {
            let end = start.saturating_add(len).min(65534);

            let small = SmallRange::<u32>::new(start, end);
            let std_range = small.to_range();

            prop_assert_eq!(std_range.start, start);
            prop_assert_eq!(std_range.end, end);
        }

        #[test]
        fn overlaps_is_symmetric(
            start1 in 0u32..1000,
            len1 in 0u32..1000,
            start2 in 0u32..1000,
            len2 in 0u32..1000
        ) {
            let a = SmallRange::<u32>::new(start1, start1 + len1);
            let b = SmallRange::<u32>::new(start2, start2 + len2);

            prop_assert_eq!(a.overlaps(&b), b.overlaps(&a));
        }
    }
}
