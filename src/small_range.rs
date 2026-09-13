use core::fmt;
use core::hash::Hash;
use core::num::NonZero;
use core::ops::{Index, IndexMut, Range};

mod sealed {
    pub trait Sealed {}
}

/// Storage word of a [`SmallRange`].
///
/// This trait is sealed and implemented for `u16`, `u32`, `u64` and `usize`.
/// The storage type fixes the size of the range; together with `LEN_BITS` it
/// fixes the capacity of the start and length fields.
pub trait SmallRangeStorage: sealed::Sealed + Copy + Eq + Ord + Hash + 'static {
    /// Width of the storage word in bits.
    const BITS: u32;

    #[doc(hidden)]
    type NonZeroWord: Copy + Eq + Ord + Hash;
    #[doc(hidden)]
    fn to_u64(self) -> u64;
    #[doc(hidden)]
    fn from_u64_truncating(word: u64) -> Self;
    #[doc(hidden)]
    fn new_nonzero(word: Self) -> Option<Self::NonZeroWord>;
    #[doc(hidden)]
    /// # Safety
    /// `word` must be nonzero.
    unsafe fn new_nonzero_unchecked(word: Self) -> Self::NonZeroWord;
    #[doc(hidden)]
    fn get(nz: Self::NonZeroWord) -> Self;
}

macro_rules! impl_storage {
    ($($S:ty),* $(,)?) => {$(
        impl sealed::Sealed for $S {}

        impl SmallRangeStorage for $S {
            const BITS: u32 = <$S>::BITS;
            type NonZeroWord = NonZero<$S>;

            #[inline(always)]
            fn to_u64(self) -> u64 {
                self as u64
            }

            #[inline(always)]
            fn from_u64_truncating(word: u64) -> Self {
                word as $S
            }

            #[inline(always)]
            fn new_nonzero(word: Self) -> Option<NonZero<$S>> {
                NonZero::new(word)
            }

            #[inline(always)]
            unsafe fn new_nonzero_unchecked(word: Self) -> NonZero<$S> {
                // SAFETY: forwarded to the caller.
                unsafe { NonZero::new_unchecked(word) }
            }

            #[inline(always)]
            fn get(nz: NonZero<$S>) -> Self {
                nz.get()
            }
        }
    )*};
}
impl_storage!(u16, u32, u64, usize);

/// A half-open range `start..end` packed into a single storage word.
///
/// The low `LEN_BITS` bits hold `len + 1`; the remaining high bits hold
/// `start`. Because the length field is never zero, the whole word is never
/// zero, and `Option<SmallRange<..>>` is the same size as `SmallRange<..>`.
///
/// Bounds are `usize` on the API side regardless of the storage type, so the
/// range can index slices directly.
///
/// # Choosing a split
///
/// | type | bytes | max start | max len |
/// |------|-------|-----------|---------|
/// | `SmallRange<u32, 8>`  | 4 | 16,777,215 | 254 |
/// | `SmallRange<u32, 16>` | 4 | 65,535 | 65,534 |
/// | `SmallRange<u64, 16>` | 8 | 2<sup>48</sup> − 1 | 65,534 |
/// | `SmallRange<u64, 32>` | 8 | 2<sup>32</sup> − 1 | 2<sup>32</sup> − 2 |
///
/// `LEN_BITS` must be at least 1 and less than the storage width. Any other
/// split is rejected at compile time:
///
/// ```compile_fail
/// let r = small_range::SmallRange::<u32, 32>::new(0, 1);
/// ```
///
/// # Ordering
///
/// The derived `Ord` compares the packed word, which orders ranges by start
/// and then by length.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SmallRange<S: SmallRangeStorage, const LEN_BITS: u32> {
    bits: S::NonZeroWord,
}

/// Error returned when a range does not fit a [`SmallRange`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OutOfRange {
    /// Requested start.
    pub start: usize,
    /// Requested end.
    pub end: usize,
    /// Largest start the target type can hold.
    pub max_start: usize,
    /// Largest length the target type can hold.
    pub max_len: usize,
}

impl fmt::Display for OutOfRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.start > self.end {
            write!(
                f,
                "invalid range {}..{}: start exceeds end",
                self.start, self.end
            )
        } else {
            write!(
                f,
                "range {}..{} does not fit: max start is {}, max length is {}",
                self.start, self.end, self.max_start, self.max_len
            )
        }
    }
}

impl core::error::Error for OutOfRange {}

impl<S: SmallRangeStorage, const LEN_BITS: u32> SmallRange<S, LEN_BITS> {
    const VALID: () = assert!(
        LEN_BITS >= 1 && LEN_BITS < S::BITS,
        "SmallRange: LEN_BITS must be at least 1 and less than the storage width"
    );

    /// Number of bits holding the start.
    pub const START_BITS: u32 = {
        let () = Self::VALID;
        S::BITS - LEN_BITS
    };

    const LEN_MASK: u64 = {
        let () = Self::VALID;
        (1u64 << LEN_BITS) - 1
    };

    /// Largest start this type can hold.
    pub const MAX_START: usize = {
        let raw = (1u64 << Self::START_BITS) - 1;
        if raw > usize::MAX as u64 {
            usize::MAX
        } else {
            raw as usize
        }
    };

    /// Largest length this type can hold. One value of the length field is
    /// spent on the niche.
    pub const MAX_LEN: usize = {
        let raw = Self::LEN_MASK - 1;
        if raw > usize::MAX as u64 {
            usize::MAX
        } else {
            raw as usize
        }
    };

    #[inline(always)]
    fn pack(start: usize, len: usize) -> S {
        S::from_u64_truncating(((start as u64) << LEN_BITS) | (len as u64 + 1))
    }

    #[inline(always)]
    fn word(self) -> u64 {
        S::get(self.bits).to_u64()
    }

    /// Creates the range `start..end`, or `None` if it does not fit.
    ///
    /// Returns `None` when `start > end`, when `start > MAX_START`, or when
    /// `end - start > MAX_LEN`. Compiles branch-free.
    ///
    /// ```
    /// use small_range::SmallRange;
    /// type R = SmallRange<u32, 8>;
    ///
    /// assert!(R::try_new(10, 20).is_some());
    /// assert!(R::try_new(20, 10).is_none());      // start > end
    /// assert!(R::try_new(0, 255).is_none());      // len 255 > MAX_LEN 254
    /// assert!(R::try_new(1 << 24, 1 << 24).is_none()); // start > MAX_START
    /// ```
    #[inline]
    #[must_use]
    pub fn try_new(start: usize, end: usize) -> Option<Self> {
        if start > end || start > Self::MAX_START || end - start > Self::MAX_LEN {
            return None;
        }
        // The length field is in 1..=LEN_MASK, so the word is nonzero and
        // `new_nonzero` never returns `None`; the compiler folds the check.
        let bits = S::new_nonzero(Self::pack(start, end - start))?;
        Some(Self { bits })
    }

    /// Creates the range `start..end`.
    ///
    /// # Panics
    ///
    /// Panics if `start > end` or if either value exceeds the capacity of the
    /// split, in debug and release builds alike.
    ///
    /// ```
    /// use small_range::SmallRange;
    /// let r = SmallRange::<u64, 32>::new(10, 20);
    /// assert_eq!(r.to_range(), 10..20);
    /// ```
    #[inline]
    #[must_use]
    #[track_caller]
    pub fn new(start: usize, end: usize) -> Self {
        match Self::try_new(start, end) {
            Some(range) => range,
            None => Self::out_of_range(start, end),
        }
    }

    /// Creates the range `start..start + len`, or `None` if it does not fit.
    #[inline]
    #[must_use]
    pub fn try_from_start_len(start: usize, len: usize) -> Option<Self> {
        Self::try_new(start, start.checked_add(len)?)
    }

    /// Creates the range `start..start + len`.
    ///
    /// # Panics
    ///
    /// Panics if the range does not fit, or if `start + len` overflows.
    #[inline]
    #[must_use]
    #[track_caller]
    pub fn from_start_len(start: usize, len: usize) -> Self {
        match start.checked_add(len) {
            Some(end) => Self::new(start, end),
            None => panic!("SmallRange: start {start} + len {len} overflows usize"),
        }
    }

    /// Creates the range `start..end` without checking that it fits.
    ///
    /// # Safety
    ///
    /// `start <= end`, `start <= MAX_START` and `end - start <= MAX_LEN` must
    /// all hold. Violating them produces a zero word, which is undefined
    /// behavior for the `NonZero` inside. Prefer [`new`](Self::new); its
    /// checks compile to three predictable branches.
    #[inline]
    #[must_use]
    pub unsafe fn new_unchecked(start: usize, end: usize) -> Self {
        debug_assert!(
            start <= end && start <= Self::MAX_START && end - start <= Self::MAX_LEN,
            "SmallRange::new_unchecked called with an out-of-range value"
        );
        // SAFETY: the caller guarantees the bounds, so the length field is at
        // least 1 and the packed word is nonzero.
        let bits = unsafe { S::new_nonzero_unchecked(Self::pack(start, end - start)) };
        Self { bits }
    }

    #[cold]
    #[inline(never)]
    #[track_caller]
    fn out_of_range(start: usize, end: usize) -> ! {
        panic!(
            "{}: {}",
            core::any::type_name::<Self>(),
            OutOfRange {
                start,
                end,
                max_start: Self::MAX_START,
                max_len: Self::MAX_LEN,
            }
        )
    }

    /// Start of the range, inclusive.
    #[inline]
    #[must_use]
    pub fn start(self) -> usize {
        (self.word() >> LEN_BITS) as usize
    }

    /// End of the range, exclusive.
    #[inline]
    #[must_use]
    pub fn end(self) -> usize {
        self.start() + self.len()
    }

    /// Number of elements in the range.
    #[inline]
    #[must_use]
    pub fn len(self) -> usize {
        // The length field is at least 1, so subtracting before masking never
        // borrows out of the field. Doing it in this order lets loops over
        // `Option<SmallRange>` auto-vectorize.
        ((self.word() - 1) & Self::LEN_MASK) as usize
    }

    /// Whether the range contains no elements.
    #[inline]
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.word() & Self::LEN_MASK == 1
    }

    /// The equivalent standard range.
    #[inline]
    #[must_use]
    pub fn to_range(self) -> Range<usize> {
        let start = self.start();
        start..start + self.len()
    }

    /// Whether `start <= value < end`.
    ///
    /// ```
    /// use small_range::SmallRange;
    /// let r = SmallRange::<u32, 8>::new(5, 10);
    /// assert!(r.contains(5));
    /// assert!(r.contains(9));
    /// assert!(!r.contains(10));
    /// assert!(!r.contains(4));
    /// ```
    #[inline]
    #[must_use]
    pub fn contains(self, value: usize) -> bool {
        value.wrapping_sub(self.start()) < self.len()
    }

    /// Whether the two ranges share at least one element.
    ///
    /// Empty ranges never overlap anything, including themselves.
    ///
    /// ```
    /// use small_range::SmallRange;
    /// type R = SmallRange<u32, 8>;
    /// assert!(R::new(0, 10).overlaps(R::new(5, 15)));
    /// assert!(!R::new(0, 10).overlaps(R::new(10, 20)));
    /// assert!(!R::new(5, 5).overlaps(R::new(0, 10)));
    /// ```
    #[inline]
    #[must_use]
    pub fn overlaps(self, other: Self) -> bool {
        !self.is_empty()
            && !other.is_empty()
            && self.start() < other.end()
            && other.start() < self.end()
    }

    /// The packed word. The encoding is part of the public API: the low
    /// `LEN_BITS` bits hold `len + 1`, the high bits hold `start`.
    ///
    /// ```
    /// use small_range::SmallRange;
    /// assert_eq!(SmallRange::<u32, 8>::new(0, 0).to_bits(), 1);
    /// assert_eq!(SmallRange::<u32, 8>::new(2, 5).to_bits(), (2 << 8) | 4);
    /// ```
    #[inline]
    #[must_use]
    pub fn to_bits(self) -> S {
        S::get(self.bits)
    }

    /// Rebuilds a range from a packed word, or `None` if the word is not a
    /// valid encoding.
    ///
    /// ```
    /// use small_range::SmallRange;
    /// type R = SmallRange<u32, 8>;
    /// let r = R::new(7, 9);
    /// assert_eq!(R::from_bits(r.to_bits()), Some(r));
    /// assert_eq!(R::from_bits(0), None);           // the niche
    /// assert_eq!(R::from_bits(7 << 8), None);      // zero length field
    /// ```
    #[inline]
    #[must_use]
    pub fn from_bits(bits: S) -> Option<Self> {
        let word = bits.to_u64();
        let len_field = word & Self::LEN_MASK;
        if len_field == 0 {
            return None;
        }
        let start = word >> LEN_BITS;
        let len = len_field - 1;
        if start > Self::MAX_START as u64 || len > Self::MAX_LEN as u64 {
            return None;
        }
        (start as usize).checked_add(len as usize)?;
        let bits = S::new_nonzero(bits)?;
        Some(Self { bits })
    }
}

impl<S: SmallRangeStorage, const LEN_BITS: u32> Default for SmallRange<S, LEN_BITS> {
    /// The empty range `0..0`.
    #[inline]
    fn default() -> Self {
        Self::new(0, 0)
    }
}

impl<S: SmallRangeStorage, const LEN_BITS: u32> fmt::Debug for SmallRange<S, LEN_BITS> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}..{}", self.start(), self.end())
    }
}

impl<S: SmallRangeStorage, const LEN_BITS: u32> IntoIterator for SmallRange<S, LEN_BITS> {
    type Item = usize;
    type IntoIter = Range<usize>;

    #[inline]
    fn into_iter(self) -> Range<usize> {
        self.to_range()
    }
}

impl<S: SmallRangeStorage, const LEN_BITS: u32> IntoIterator for &SmallRange<S, LEN_BITS> {
    type Item = usize;
    type IntoIter = Range<usize>;

    #[inline]
    fn into_iter(self) -> Range<usize> {
        self.to_range()
    }
}

impl<S: SmallRangeStorage, const LEN_BITS: u32> From<SmallRange<S, LEN_BITS>> for Range<usize> {
    #[inline]
    fn from(range: SmallRange<S, LEN_BITS>) -> Range<usize> {
        range.to_range()
    }
}

impl<S: SmallRangeStorage, const LEN_BITS: u32> TryFrom<Range<usize>> for SmallRange<S, LEN_BITS> {
    type Error = OutOfRange;

    #[inline]
    fn try_from(range: Range<usize>) -> Result<Self, OutOfRange> {
        Self::try_new(range.start, range.end).ok_or(OutOfRange {
            start: range.start,
            end: range.end,
            max_start: Self::MAX_START,
            max_len: Self::MAX_LEN,
        })
    }
}

impl<T, S: SmallRangeStorage, const LEN_BITS: u32> Index<SmallRange<S, LEN_BITS>> for [T] {
    type Output = [T];

    #[inline]
    fn index(&self, range: SmallRange<S, LEN_BITS>) -> &[T] {
        &self[range.to_range()]
    }
}

impl<T, S: SmallRangeStorage, const LEN_BITS: u32> IndexMut<SmallRange<S, LEN_BITS>> for [T] {
    #[inline]
    fn index_mut(&mut self, range: SmallRange<S, LEN_BITS>) -> &mut [T] {
        &mut self[range.to_range()]
    }
}

impl<S: SmallRangeStorage, const LEN_BITS: u32> Index<SmallRange<S, LEN_BITS>> for str {
    type Output = str;

    #[inline]
    fn index(&self, range: SmallRange<S, LEN_BITS>) -> &str {
        &self[range.to_range()]
    }
}

impl<S: SmallRangeStorage, const LEN_BITS: u32> IndexMut<SmallRange<S, LEN_BITS>> for str {
    #[inline]
    fn index_mut(&mut self, range: SmallRange<S, LEN_BITS>) -> &mut str {
        &mut self[range.to_range()]
    }
}

#[cfg(feature = "alloc")]
mod alloc_impls {
    use super::{SmallRange, SmallRangeStorage};
    use alloc::string::String;
    use alloc::vec::Vec;
    use core::ops::{Index, IndexMut};

    impl<T, S: SmallRangeStorage, const LEN_BITS: u32> Index<SmallRange<S, LEN_BITS>> for Vec<T> {
        type Output = [T];

        #[inline]
        fn index(&self, range: SmallRange<S, LEN_BITS>) -> &[T] {
            &self[range.to_range()]
        }
    }

    impl<T, S: SmallRangeStorage, const LEN_BITS: u32> IndexMut<SmallRange<S, LEN_BITS>> for Vec<T> {
        #[inline]
        fn index_mut(&mut self, range: SmallRange<S, LEN_BITS>) -> &mut [T] {
            &mut self[range.to_range()]
        }
    }

    impl<S: SmallRangeStorage, const LEN_BITS: u32> Index<SmallRange<S, LEN_BITS>> for String {
        type Output = str;

        #[inline]
        fn index(&self, range: SmallRange<S, LEN_BITS>) -> &str {
            &self[range.to_range()]
        }
    }

    impl<S: SmallRangeStorage, const LEN_BITS: u32> IndexMut<SmallRange<S, LEN_BITS>> for String {
        #[inline]
        fn index_mut(&mut self, range: SmallRange<S, LEN_BITS>) -> &mut str {
            &mut self[range.to_range()]
        }
    }
}
