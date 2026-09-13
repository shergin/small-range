extern crate alloc;
extern crate std;

use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::mem::{align_of, size_of};
use core::ops::Range;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::{OutOfRange, SmallRange};

type R16 = SmallRange<u16, 8>;
type R32 = SmallRange<u32, 16>;
type R64 = SmallRange<u64, 32>;
type Hop = SmallRange<u32, 8>;
type Wide = SmallRange<u64, 16>;

// =============================================================================
// Layout
// =============================================================================

#[test]
fn sizes_match_storage() {
    assert_eq!(size_of::<R16>(), 2);
    assert_eq!(size_of::<R32>(), 4);
    assert_eq!(size_of::<R64>(), 8);
    assert_eq!(size_of::<Hop>(), 4);
    assert_eq!(size_of::<Wide>(), 8);
    assert_eq!(size_of::<SmallRange<usize, 8>>(), size_of::<usize>());
    assert_eq!(align_of::<Hop>(), 4);
}

#[test]
fn option_is_free_for_every_split() {
    assert_eq!(size_of::<Option<R16>>(), 2);
    assert_eq!(size_of::<Option<R32>>(), 4);
    assert_eq!(size_of::<Option<R64>>(), 8);
    assert_eq!(size_of::<Option<Hop>>(), 4);
    assert_eq!(size_of::<Option<Wide>>(), 8);
    assert_eq!(size_of::<Option<SmallRange<u32, 1>>>(), 4);
    assert_eq!(size_of::<Option<SmallRange<u32, 31>>>(), 4);
    assert_eq!(size_of::<Option<SmallRange<u64, 63>>>(), 8);
    assert_eq!(size_of::<Option<Option<Hop>>>(), 8);
}

#[test]
fn versus_std_range() {
    assert_eq!(size_of::<Option<Range<u32>>>(), 12);
    assert_eq!(size_of::<Option<Range<usize>>>(), 24);
    assert_eq!(size_of::<Option<R64>>(), 8);
}

#[test]
fn hop_shaped_struct_is_eight_bytes() {
    #[allow(dead_code)]
    struct HopNode {
        swap_index: u32,
        next: Option<Hop>,
    }
    assert_eq!(size_of::<HopNode>(), 8);
}

// =============================================================================
// Capacities
// =============================================================================

#[test]
fn capacities() {
    assert_eq!(Hop::START_BITS, 24);
    assert_eq!(Hop::MAX_START, (1 << 24) - 1);
    assert_eq!(Hop::MAX_LEN, 254);

    assert_eq!(R16::MAX_START, 255);
    assert_eq!(R16::MAX_LEN, 254);

    assert_eq!(R32::MAX_START, 65_535);
    assert_eq!(R32::MAX_LEN, 65_534);

    assert_eq!(R64::MAX_START, u32::MAX as usize);
    assert_eq!(R64::MAX_LEN, u32::MAX as usize - 1);

    assert_eq!(Wide::MAX_START, (1 << 48) - 1);
    assert_eq!(Wide::MAX_LEN, 65_534);

    assert_eq!(SmallRange::<u64, 63>::MAX_START, 1);
    assert_eq!(SmallRange::<u64, 63>::MAX_LEN, (1u64 << 63) as usize - 2);

    assert_eq!(SmallRange::<u32, 1>::MAX_START, (1 << 31) - 1);
    assert_eq!(SmallRange::<u32, 1>::MAX_LEN, 0);
}

// =============================================================================
// Roundtrip at boundaries
// =============================================================================

macro_rules! roundtrip_at_boundaries {
    ($name:ident, $ty:ty) => {
        #[test]
        fn $name() {
            let starts = [
                0,
                1,
                2,
                <$ty>::MAX_START / 2,
                <$ty>::MAX_START.saturating_sub(1),
                <$ty>::MAX_START,
            ];
            let lens = [
                0,
                1,
                2,
                <$ty>::MAX_LEN / 2,
                <$ty>::MAX_LEN.saturating_sub(1),
                <$ty>::MAX_LEN,
            ];
            for start in starts {
                for len in lens {
                    if start > <$ty>::MAX_START || len > <$ty>::MAX_LEN {
                        continue;
                    }
                    let Some(end) = start.checked_add(len) else {
                        continue;
                    };
                    let r = <$ty>::new(start, end);
                    assert_eq!(r.start(), start, "start of {start}..{end}");
                    assert_eq!(r.end(), end, "end of {start}..{end}");
                    assert_eq!(r.len(), len, "len of {start}..{end}");
                    assert_eq!(r.is_empty(), len == 0);
                    assert_eq!(r.to_range(), start..end);
                    assert_eq!(<$ty>::try_new(start, end), Some(r));
                    assert_eq!(<$ty>::from_start_len(start, len), r);
                    assert_eq!(<$ty>::from_bits(r.to_bits()), Some(r));
                }
            }
        }
    };
}

roundtrip_at_boundaries!(roundtrip_u16_8, R16);
roundtrip_at_boundaries!(roundtrip_u32_16, R32);
roundtrip_at_boundaries!(roundtrip_u32_8, Hop);
roundtrip_at_boundaries!(roundtrip_u64_32, R64);
roundtrip_at_boundaries!(roundtrip_u64_16, Wide);
roundtrip_at_boundaries!(roundtrip_u64_1, SmallRange<u64, 1>);
roundtrip_at_boundaries!(roundtrip_u64_63, SmallRange<u64, 63>);
roundtrip_at_boundaries!(roundtrip_usize_8, SmallRange<usize, 8>);

// =============================================================================
// Rejection and panics (release builds included)
// =============================================================================

#[test]
fn try_new_rejects_out_of_range() {
    assert!(Hop::try_new(20, 10).is_none());
    assert!(Hop::try_new(Hop::MAX_START + 1, Hop::MAX_START + 1).is_none());
    assert!(Hop::try_new(0, Hop::MAX_LEN + 1).is_none());
    assert!(Hop::try_new(usize::MAX, usize::MAX).is_none());
    assert!(Hop::try_new(0, usize::MAX).is_none());

    assert!(Hop::try_new(Hop::MAX_START, Hop::MAX_START).is_some());
    assert!(Hop::try_new(0, Hop::MAX_LEN).is_some());
    assert!(Hop::try_new(Hop::MAX_START, Hop::MAX_START + Hop::MAX_LEN).is_some());
}

#[test]
fn try_from_start_len_rejects_overflow() {
    assert!(R64::try_from_start_len(usize::MAX, 1).is_none());
    assert!(R64::try_from_start_len(1, usize::MAX).is_none());
    assert_eq!(R64::try_from_start_len(3, 4), Some(R64::new(3, 7)));
}

#[test]
#[should_panic(expected = "invalid range 20..10: start exceeds end")]
fn new_panics_when_start_exceeds_end() {
    let _ = Hop::new(20, 10);
}

#[test]
#[should_panic(
    expected = "range 16777216..16777216 does not fit: max start is 16777215, max length is 254"
)]
fn new_panics_when_start_too_large() {
    let _ = Hop::new(1 << 24, 1 << 24);
}

#[test]
#[should_panic(expected = "range 0..255 does not fit")]
fn new_panics_when_length_too_large() {
    let _ = Hop::new(0, 255);
}

#[test]
#[should_panic(expected = "overflows usize")]
fn from_start_len_panics_on_overflow() {
    let _ = R64::from_start_len(usize::MAX, 1);
}

#[test]
fn panic_message_names_the_type() {
    let err = std::panic::catch_unwind(|| Hop::new(5, 3)).unwrap_err();
    let msg = err.downcast_ref::<String>().cloned().unwrap();
    assert!(msg.contains("SmallRange<u32, 8>"), "{msg}");
}

#[test]
fn out_of_range_display() {
    let e = OutOfRange {
        start: 5,
        end: 3,
        max_start: 10,
        max_len: 10,
    };
    assert_eq!(format!("{e}"), "invalid range 5..3: start exceeds end");
    let e = OutOfRange {
        start: 0,
        end: 300,
        max_start: 10,
        max_len: 254,
    };
    assert_eq!(
        format!("{e}"),
        "range 0..300 does not fit: max start is 10, max length is 254"
    );
}

// =============================================================================
// Bits
// =============================================================================

#[test]
fn encoding_is_documented() {
    assert_eq!(Hop::new(0, 0).to_bits(), 1);
    assert_eq!(Hop::new(0, 1).to_bits(), 2);
    assert_eq!(Hop::new(1, 1).to_bits(), (1 << 8) | 1);
    assert_eq!(Hop::new(2, 5).to_bits(), (2 << 8) | 4);
    assert_eq!(R64::new(7, 9).to_bits(), (7 << 32) | 3);
    assert_eq!(
        Hop::new(Hop::MAX_START, Hop::MAX_START + Hop::MAX_LEN).to_bits(),
        u32::MAX
    );
}

#[test]
fn from_bits_rejects_invalid_words() {
    assert_eq!(Hop::from_bits(0), None);
    assert_eq!(Hop::from_bits(7 << 8), None);
    assert_eq!(
        Hop::from_bits(u32::MAX),
        Some(Hop::new(Hop::MAX_START, Hop::MAX_START + Hop::MAX_LEN))
    );
    assert_eq!(Hop::from_bits(1), Some(Hop::default()));
}

#[test]
fn new_unchecked_matches_new() {
    for (s, e) in [
        (0, 0),
        (1, 1),
        (3, 200),
        (Hop::MAX_START, Hop::MAX_START + Hop::MAX_LEN),
    ] {
        // SAFETY: every pair is within capacity.
        let r = unsafe { Hop::new_unchecked(s, e) };
        assert_eq!(r, Hop::new(s, e));
    }
}

// =============================================================================
// Semantics
// =============================================================================

#[test]
fn default_is_empty_at_zero() {
    let r = Hop::default();
    assert_eq!(r.to_range(), 0..0);
    assert!(r.is_empty());
    assert_eq!(r.len(), 0);
}

#[test]
fn some_empty_and_none_are_distinct() {
    let slots: [Option<Hop>; 3] = [None, Some(Hop::default()), Some(Hop::new(3, 5))];
    assert!(slots[0].is_none());
    assert!(slots[1].is_some_and(|r| r.is_empty()));
    assert!(slots[2].is_some_and(|r| !r.is_empty()));
}

#[test]
fn debug_prints_like_std_range() {
    assert_eq!(format!("{:?}", Hop::new(10, 20)), "10..20");
    assert_eq!(format!("{:?}", Hop::new(5, 5)), "5..5");
    assert_eq!(format!("{:?}", Some(Hop::new(1, 2))), "Some(1..2)");
}

#[test]
fn equality_and_hash() {
    fn hash<T: Hash>(t: &T) -> u64 {
        let mut h = DefaultHasher::new();
        t.hash(&mut h);
        h.finish()
    }
    let a = R32::new(10, 20);
    let b = R32::new(10, 20);
    let c = R32::new(10, 21);
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(hash(&a), hash(&b));
}

#[test]
fn ord_is_start_then_len() {
    let mut v = [
        Hop::new(5, 9),
        Hop::new(1, 3),
        Hop::new(5, 6),
        Hop::new(0, 0),
        Hop::new(1, 1),
    ];
    v.sort();
    let sorted: Vec<Range<usize>> = v.iter().map(|r| r.to_range()).collect();
    assert_eq!(sorted, vec![0..0, 1..1, 1..3, 5..6, 5..9]);
}

#[test]
fn copy_semantics() {
    let a = Hop::new(1, 2);
    let b = a;
    assert_eq!(a, b);
}

#[test]
fn iteration_by_value_and_by_ref() {
    let r = Hop::new(5, 8);
    assert_eq!(r.into_iter().collect::<Vec<_>>(), vec![5, 6, 7]);
    assert_eq!((&r).into_iter().collect::<Vec<_>>(), vec![5, 6, 7]);
    let mut seen = Vec::new();
    for i in &r {
        seen.push(i);
    }
    for i in r {
        seen.push(i);
    }
    assert_eq!(seen, vec![5, 6, 7, 5, 6, 7]);
    assert!(Hop::new(3, 3).into_iter().next().is_none());
    assert_eq!(r.into_iter().len(), 3);
}

#[test]
#[allow(clippy::reversed_empty_ranges)]
fn conversions_with_std_range() {
    let std: Range<usize> = Hop::new(2, 4).into();
    assert_eq!(std, 2..4);
    assert_eq!(Hop::try_from(2..4), Ok(Hop::new(2, 4)));
    assert_eq!(
        Hop::try_from(9..3),
        Err(OutOfRange {
            start: 9,
            end: 3,
            max_start: Hop::MAX_START,
            max_len: Hop::MAX_LEN
        })
    );
    assert_eq!(
        Hop::try_from(0..1000),
        Err(OutOfRange {
            start: 0,
            end: 1000,
            max_start: Hop::MAX_START,
            max_len: 254
        })
    );
}

#[test]
fn indexes_slices_arrays_and_str() {
    let data = [10, 11, 12, 13, 14];
    let r = Hop::new(1, 4);
    assert_eq!(&data[r], &[11, 12, 13]);
    assert_eq!(&data[..][r], &[11, 12, 13]);

    let mut buf = [0u8; 5];
    buf[..][r].fill(7);
    assert_eq!(buf, [0, 7, 7, 7, 0]);

    let s = "hello world";
    assert_eq!(&s[Hop::new(6, 11)], "world");
    let mut bytes = *b"hello";
    let text = core::str::from_utf8_mut(&mut bytes).unwrap();
    text[Hop::new(0, 1)].make_ascii_uppercase();
    assert_eq!(text, "Hello");
}

#[cfg(feature = "alloc")]
#[test]
fn indexes_vec_and_string() {
    let r = Hop::new(1, 4);
    let mut v: Vec<u8> = vec![0; 5];
    v[r].fill(7);
    assert_eq!(&v[r], &[7, 7, 7]);
    assert_eq!(v, vec![0, 7, 7, 7, 0]);

    let mut owned = String::from("hello");
    owned[Hop::new(0, 1)].make_ascii_uppercase();
    assert_eq!(&owned[Hop::new(0, 2)], "He");
    assert_eq!(owned, "Hello");
}

#[test]
#[should_panic]
fn indexing_out_of_bounds_panics() {
    let data = [1, 2, 3];
    let _ = &data[Hop::new(1, 5)];
}

#[test]
fn contains() {
    let r = R32::new(5, 10);
    assert!(r.contains(5));
    assert!(r.contains(9));
    assert!(!r.contains(4));
    assert!(!r.contains(10));
    assert!(!r.contains(usize::MAX));
    assert!(!R32::new(5, 5).contains(5));
    assert!(R32::new(0, 1).contains(0));
}

#[test]
fn overlaps() {
    let a = R32::new(0, 10);
    let b = R32::new(5, 15);
    let c = R32::new(10, 20);
    let empty = R32::new(5, 5);
    assert!(a.overlaps(b) && b.overlaps(a));
    assert!(!a.overlaps(c) && !c.overlaps(a));
    assert!(b.overlaps(c));
    assert!(a.overlaps(a));
    assert!(!empty.overlaps(a) && !a.overlaps(empty) && !empty.overlaps(empty));
    assert!(R32::new(0, 100).overlaps(R32::new(25, 75)));
}

#[test]
fn methods_work_through_references() {
    let r = Hop::new(1, 3);
    let by_ref = &r;
    assert_eq!(by_ref.start(), 1);
    assert_eq!(by_ref.len(), 2);
    assert!(!by_ref.is_empty());
    let opt = Some(r);
    assert!(opt.as_ref().is_some_and(|r| r.contains(2)));
}

// =============================================================================
// Property-based
// =============================================================================

mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn roundtrip_hop(start in 0usize..=Hop::MAX_START, len in 0usize..=Hop::MAX_LEN) {
            let r = Hop::new(start, start + len);
            prop_assert_eq!(r.start(), start);
            prop_assert_eq!(r.len(), len);
            prop_assert_eq!(r.end(), start + len);
            prop_assert_eq!(Hop::from_bits(r.to_bits()), Some(r));
        }

        #[test]
        fn roundtrip_r64(start in 0usize..=R64::MAX_START, len in 0usize..=R64::MAX_LEN) {
            let r = R64::new(start, start + len);
            prop_assert_eq!(r.to_range(), start..start + len);
        }

        #[test]
        fn roundtrip_wide(start in 0usize..=Wide::MAX_START, len in 0usize..=Wide::MAX_LEN) {
            let r = Wide::new(start, start + len);
            prop_assert_eq!(r.to_range(), start..start + len);
        }

        #[test]
        fn try_new_never_panics(start in any::<usize>(), end in any::<usize>()) {
            let _ = Hop::try_new(start, end);
            let _ = R16::try_new(start, end);
            let _ = R64::try_new(start, end);
            let _ = SmallRange::<u64, 63>::try_new(start, end);
        }

        #[test]
        fn try_new_agrees_with_capacity(start in any::<usize>(), end in any::<usize>()) {
            let fits = start <= end && start <= Hop::MAX_START && end - start <= Hop::MAX_LEN;
            prop_assert_eq!(Hop::try_new(start, end).is_some(), fits);
        }

        #[test]
        fn from_bits_accepts_exactly_valid_words(word in any::<u32>()) {
            let valid = word & 0xFF != 0;
            let r = Hop::from_bits(word);
            prop_assert_eq!(r.is_some(), valid);
            if let Some(r) = r {
                prop_assert_eq!(r.to_bits(), word);
            }
        }

        #[test]
        fn contains_matches_std(start in 0usize..1000, len in 0usize..250, value in 0usize..2000) {
            let r = Hop::new(start, start + len);
            prop_assert_eq!(r.contains(value), (start..start + len).contains(&value));
        }

        #[test]
        fn overlaps_is_symmetric(a in 0usize..500, la in 0usize..250, b in 0usize..500, lb in 0usize..250) {
            let x = Hop::new(a, a + la);
            let y = Hop::new(b, b + lb);
            prop_assert_eq!(x.overlaps(y), y.overlaps(x));
        }

        #[test]
        fn ord_matches_tuple_order(a in 0usize..300, la in 0usize..250, b in 0usize..300, lb in 0usize..250) {
            let x = Hop::new(a, a + la);
            let y = Hop::new(b, b + lb);
            prop_assert_eq!(x.cmp(&y), (a, la).cmp(&(b, lb)));
        }
    }
}

// =============================================================================
// serde
// =============================================================================

#[cfg(feature = "serde")]
mod serde_tests {
    use super::*;

    #[test]
    fn serializes_as_start_and_end() {
        let json = serde_json::to_string(&Hop::new(3, 7)).unwrap();
        assert_eq!(json, r#"{"start":3,"end":7}"#);
        let back: Hop = serde_json::from_str(&json).unwrap();
        assert_eq!(back, Hop::new(3, 7));
    }

    #[test]
    fn deserialize_validates() {
        let err = serde_json::from_str::<Hop>(r#"{"start":9,"end":3}"#).unwrap_err();
        assert!(format!("{err}").contains("start exceeds end"), "{err}");
        let err = serde_json::from_str::<Hop>(r#"{"start":0,"end":1000}"#).unwrap_err();
        assert!(format!("{err}").contains("does not fit"), "{err}");
    }

    #[test]
    fn option_roundtrip() {
        let v: Vec<Option<Hop>> = vec![None, Some(Hop::default()), Some(Hop::new(1, 2))];
        let json = serde_json::to_string(&v).unwrap();
        let back: Vec<Option<Hop>> = serde_json::from_str(&json).unwrap();
        assert_eq!(back, v);
    }
}
