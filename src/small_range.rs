use core::fmt;
use core::hash::Hash;
use core::num::NonZero;
use core::ops::Range;

use num_traits::{AsPrimitive, PrimInt, Unsigned};

/// Sealed trait module to prevent external implementations.
mod private {
    pub trait Sealed {}
    impl Sealed for u16 {}
    impl Sealed for u32 {}
    impl Sealed for u64 {}
    impl Sealed for usize {}
}

/// Trait for types that can be used as storage in a `SmallRange`.
///
/// This trait is sealed and only implemented for `u16`, `u32`, `u64`, and `usize`.
/// The storage type determines how much space the range uses and the maximum
/// values for start and length (each limited to half the storage width minus 1).
///
/// | Storage | Max Start | Max Length | Size     |
/// |---------|-----------|------------|----------|
/// | `u16`   | 254       | 254        | 2 bytes  |
/// | `u32`   | 65,534    | 65,534     | 4 bytes  |
/// | `u64`   | ~4.29B    | ~4.29B     | 8 bytes  |
/// | `usize` | ~4.29B*   | ~4.29B*    | 8 bytes* |
///
/// *On 64-bit platforms. On 32-bit, same as u32.
pub trait SmallRangeStorage:
    private::Sealed + PrimInt + Unsigned + Hash + AsPrimitive<usize> + 'static
where
    usize: AsPrimitive<Self>,
{
    /// The NonZero wrapper for this storage type.
    type NonZeroStorage: Copy + Eq + Hash;

    /// Number of bits for each element (half of storage width).
    const HALF_BITS: u32;

    /// Mask for extracting lower half (all bits set for half-width).
    const LOW_MASK: Self;

    /// Create a NonZero from storage value.
    ///
    /// # Safety
    /// The value must be non-zero.
    unsafe fn new_nonzero_unchecked(val: Self) -> Self::NonZeroStorage;

    /// Get the storage value from a NonZero.
    fn get_nonzero(nz: Self::NonZeroStorage) -> Self;
}

impl SmallRangeStorage for u16 {
    type NonZeroStorage = NonZero<u16>;
    const HALF_BITS: u32 = 8;
    const LOW_MASK: Self = 0xFF;

    #[inline]
    unsafe fn new_nonzero_unchecked(val: Self) -> Self::NonZeroStorage {
        NonZero::new_unchecked(val)
    }

    #[inline]
    fn get_nonzero(nz: Self::NonZeroStorage) -> Self {
        nz.get()
    }
}

impl SmallRangeStorage for u32 {
    type NonZeroStorage = NonZero<u32>;
    const HALF_BITS: u32 = 16;
    const LOW_MASK: Self = 0xFFFF;

    #[inline]
    unsafe fn new_nonzero_unchecked(val: Self) -> Self::NonZeroStorage {
        NonZero::new_unchecked(val)
    }

    #[inline]
    fn get_nonzero(nz: Self::NonZeroStorage) -> Self {
        nz.get()
    }
}

impl SmallRangeStorage for u64 {
    type NonZeroStorage = NonZero<u64>;
    const HALF_BITS: u32 = 32;
    const LOW_MASK: Self = 0xFFFF_FFFF;

    #[inline]
    unsafe fn new_nonzero_unchecked(val: Self) -> Self::NonZeroStorage {
        NonZero::new_unchecked(val)
    }

    #[inline]
    fn get_nonzero(nz: Self::NonZeroStorage) -> Self {
        nz.get()
    }
}

impl SmallRangeStorage for usize {
    type NonZeroStorage = NonZero<usize>;
    // On 64-bit: 32, on 32-bit: 16
    const HALF_BITS: u32 = (core::mem::size_of::<usize>() * 8 / 2) as u32;
    // On 64-bit: 0xFFFF_FFFF, on 32-bit: 0xFFFF
    const LOW_MASK: Self = (1usize << Self::HALF_BITS) - 1;

    #[inline]
    unsafe fn new_nonzero_unchecked(val: Self) -> Self::NonZeroStorage {
        NonZero::new_unchecked(val)
    }

    #[inline]
    fn get_nonzero(nz: Self::NonZeroStorage) -> Self {
        nz.get()
    }
}

/// A compact range that packs start and length into a single storage value.
///
/// This type stores a range's start position and length in a single value,
/// achieving 50% space savings compared to `Range<T>`. It also enables niche
/// optimization so `Option<SmallRange<T>>` is the same size as `SmallRange<T>`.
///
/// # Type Parameters
/// - `T`: The storage type (`u16`, `u32`, `u64`, or `usize`). Defaults to `u64`.
///
/// # Storage Layout
/// - `SmallRange<u16>`: 2 bytes (vs 4 bytes for `Range<u16>`)
/// - `SmallRange<u32>`: 4 bytes (vs 8 bytes for `Range<u32>`)
/// - `SmallRange<u64>`: 8 bytes (vs 16 bytes for `Range<u64>`)
/// - `SmallRange<usize>`: 8 bytes on 64-bit (vs 16 bytes for `Range<usize>`)
///
/// # Encoding
/// Uses `(start+1, length+1)` encoding where start is in the high bits and
/// length is in the low bits. Since both halves are always >= 1, the packed
/// value is never zero, allowing `Option` to use 0 for `None`.
///
/// # Constraints
/// - Start must not exceed end
/// - Start and length must each fit in half the storage width minus 1
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SmallRange<T: SmallRangeStorage = u64>
where
    usize: AsPrimitive<T>,
{
    bits: T::NonZeroStorage,
}

impl<T: SmallRangeStorage> SmallRange<T>
where
    usize: AsPrimitive<T>,
{
    #[inline]
    fn encode(start: T, end: T) -> T::NonZeroStorage {
        debug_assert!(start <= end, "start must not exceed end");
        let length = end - start;
        // Add 1 to both, ensuring neither half is ever 0
        let hi = start + T::one();
        let lo = length + T::one();
        debug_assert!(hi <= T::LOW_MASK, "start+1 exceeds half-width capacity");
        debug_assert!(lo <= T::LOW_MASK, "length+1 exceeds half-width capacity");
        let packed = (hi << T::HALF_BITS as usize) | lo;
        // SAFETY: packed is NEVER zero because both hi >= 1 and lo >= 1
        unsafe { T::new_nonzero_unchecked(packed) }
    }

    #[inline]
    fn decode_start_length(bits: T::NonZeroStorage) -> (T, T) {
        let packed = T::get_nonzero(bits);
        let hi = packed >> T::HALF_BITS as usize;
        let lo = packed & T::LOW_MASK;
        let start = hi - T::one();
        let length = lo - T::one();
        (start, length)
    }

    /// Creates a new `SmallRange` with the given start and end values.
    ///
    /// # Panics (debug only)
    /// - If start exceeds end
    /// - If start or length exceed the half-width capacity
    #[inline]
    pub fn new(start: T, end: T) -> Self {
        Self {
            bits: Self::encode(start, end),
        }
    }

    /// Returns the start of the range.
    #[inline]
    pub fn start(&self) -> T {
        let (start, _) = Self::decode_start_length(self.bits);
        start
    }

    /// Returns the end of the range (exclusive).
    #[inline]
    pub fn end(&self) -> T {
        let (start, length) = Self::decode_start_length(self.bits);
        start + length
    }

    /// Returns the length of the range.
    #[inline]
    pub fn len(&self) -> usize {
        let packed = T::get_nonzero(self.bits);
        let lo = packed & T::LOW_MASK;
        (lo - T::one()).as_()
    }

    /// Returns `true` if the range is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        let packed = T::get_nonzero(self.bits);
        let lo = packed & T::LOW_MASK;
        lo == T::one() // length + 1 == 1 means length == 0
    }

    /// Converts the `SmallRange` to a standard `Range<T>`.
    #[inline]
    pub fn to_range(&self) -> Range<T> {
        let (start, length) = Self::decode_start_length(self.bits);
        start..(start + length)
    }

    /// Creates a new `SmallRange` if the values are valid, returns `None` otherwise.
    ///
    /// Returns `None` if:
    /// - `start > end`
    /// - `start` or `length` exceed half-width capacity
    ///
    /// # Examples
    /// ```
    /// use small_range::SmallRange;
    ///
    /// // Valid range
    /// assert!(SmallRange::<u32>::try_new(10, 20).is_some());
    ///
    /// // Invalid: start > end
    /// assert!(SmallRange::<u32>::try_new(20, 10).is_none());
    ///
    /// // Invalid: values exceed capacity
    /// assert!(SmallRange::<u16>::try_new(255, 300).is_none());
    /// ```
    #[inline]
    pub fn try_new(start: T, end: T) -> Option<Self> {
        if start > end {
            return None;
        }
        let length = end - start;
        let hi = start + T::one();
        let lo = length + T::one();
        if hi > T::LOW_MASK || lo > T::LOW_MASK {
            return None;
        }
        let packed = (hi << T::HALF_BITS as usize) | lo;
        // SAFETY: packed is never zero because both hi >= 1 and lo >= 1
        Some(Self {
            bits: unsafe { T::new_nonzero_unchecked(packed) },
        })
    }

    /// Returns `true` if the range contains the given value.
    ///
    /// A value is contained if `start <= value < end`.
    ///
    /// # Examples
    /// ```
    /// use small_range::SmallRange;
    ///
    /// let range = SmallRange::<u32>::new(5, 10);
    /// assert!(range.contains(5));   // start is included
    /// assert!(range.contains(7));
    /// assert!(!range.contains(10)); // end is excluded
    /// assert!(!range.contains(4));
    /// ```
    #[inline]
    pub fn contains(&self, value: T) -> bool {
        value >= self.start() && value < self.end()
    }

    /// Returns `true` if this range overlaps with `other`.
    ///
    /// Two ranges overlap if they share at least one common value.
    /// Empty ranges never overlap with anything (including themselves).
    ///
    /// # Examples
    /// ```
    /// use small_range::SmallRange;
    ///
    /// let a = SmallRange::<u32>::new(0, 10);
    /// let b = SmallRange::<u32>::new(5, 15);
    /// let c = SmallRange::<u32>::new(10, 20);
    ///
    /// assert!(a.overlaps(&b));   // overlap at 5..10
    /// assert!(!a.overlaps(&c));  // a ends where c starts (no overlap)
    /// assert!(b.overlaps(&c));   // overlap at 10..15
    ///
    /// // Empty ranges never overlap
    /// let empty = SmallRange::<u32>::new(5, 5);
    /// assert!(!empty.overlaps(&a));
    /// ```
    #[inline]
    pub fn overlaps(&self, other: &Self) -> bool {
        // Empty ranges never overlap with anything
        !self.is_empty()
            && !other.is_empty()
            && self.start() < other.end()
            && other.start() < self.end()
    }
}

impl<T: SmallRangeStorage> Default for SmallRange<T>
where
    usize: AsPrimitive<T>,
{
    fn default() -> Self {
        Self::new(T::zero(), T::zero())
    }
}

impl<T: SmallRangeStorage + fmt::Debug> fmt::Debug for SmallRange<T>
where
    usize: AsPrimitive<T>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SmallRange")
            .field("start", &self.start())
            .field("end", &self.end())
            .finish()
    }
}

impl<T: SmallRangeStorage> IntoIterator for SmallRange<T>
where
    usize: AsPrimitive<T>,
    Range<T>: Iterator<Item = T>,
{
    type Item = T;
    type IntoIter = Range<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.to_range()
    }
}

impl<T: SmallRangeStorage> IntoIterator for &SmallRange<T>
where
    usize: AsPrimitive<T>,
    Range<T>: Iterator<Item = T>,
{
    type Item = T;
    type IntoIter = Range<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.to_range()
    }
}
