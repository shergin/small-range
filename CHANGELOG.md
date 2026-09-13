# Changelog

## 2.0.0

Breaking redesign around a configurable start/length split.

### Added
- `SmallRange<S, const LEN_BITS: u32>`: the low `LEN_BITS` bits hold
  `len + 1`, the rest hold `start`. Invalid splits fail to compile.
- `MAX_START`, `MAX_LEN` and `START_BITS` associated constants.
- `from_start_len`, `try_from_start_len`, `unsafe new_unchecked`.
- `to_bits` and `from_bits`; the encoding is now documented public API.
- `Ord` and `PartialOrd`, ordering by start and then by length.
- `Index` and `IndexMut` on `[T]` and `str`, and behind the default `alloc`
  feature on `Vec<T>` and `String`, so `&v[range]` works everywhere.
- `From<SmallRange<..>> for Range<usize>` and
  `TryFrom<Range<usize>> for SmallRange<..>` with an `OutOfRange` error.
- Optional `serde` feature serializing as `{ start, end }` with validation.
- `#[must_use]` on constructors and accessors.

### Changed
- All bounds are `usize` on the API side regardless of storage type.
- `new` validates in release builds and panics with a message naming the
  type and its limits. `try_new` is branch-free.
- Only the length field is biased. `start` decodes with a single shift and
  `MAX_START` is one larger than in 1.x. Raw words are not compatible with
  1.x.
- `len` is computed as `(word - 1) & mask`, which lets loops over
  `Option<SmallRange<..>>` auto-vectorize; the 1.x `start` and `len` loops
  often did not.
- `contains` and `overlaps` take arguments by value.
- `Debug` prints `10..20`.
- Dropped the `num-traits` dependency and the `usize: AsPrimitive<T>` bound.
- Minimum supported Rust version is 1.85.

### Fixed
- **Soundness.** In 1.0.0, `new` only checked its arguments with
  `debug_assert!`. In release builds `start > end` could wrap to a zero word
  and reach `NonZero::new_unchecked`, which is undefined behavior. Miri
  confirms it with `SmallRange::<u16>::new(255, 254)`. 1.0.0 should be
  yanked.

## 1.0.0

Initial release: `SmallRange<T>` with a fixed half-and-half split.
