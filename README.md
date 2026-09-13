# small_range

A half-open range packed into one integer, with a configurable split between
the start and length fields and a zero-cost `Option`.

```rust
use small_range::SmallRange;
use core::mem::size_of;

// 24 bits of start, 8 bits of length, in 4 bytes.
type HopRange = SmallRange<u32, 8>;

let r = HopRange::new(10, 20);
assert_eq!(r.start(), 10);
assert_eq!(r.end(), 20);
assert_eq!(r.len(), 10);

// The Option costs nothing.
assert_eq!(size_of::<Option<HopRange>>(), 4);

// It indexes slices directly.
let data = [0u8; 32];
assert_eq!(data[r].len(), 10);
```

## Why

`Range<usize>` is 16 bytes, `Option<Range<usize>>` is 24, and neither is
`Copy`. Code that stores many ranges usually stores them in `usize` because
that is what slicing wants, even when the values are small.

`SmallRange<S, LEN_BITS>` stores `start` and `len` in one word of type `S`.
The low `LEN_BITS` bits hold `len + 1`, so the word is never zero and
`Option<SmallRange<..>>` is the same size as the range itself. The API side
stays `usize`, so nothing needs casting to index a slice.

The split is yours to choose. Ranges that index a large table but are
themselves short, such as source spans or adjacency lists, can spend most of
the word on `start`:

| type | bytes | max start | max len |
|------|-------|-----------|---------|
| `SmallRange<u16, 8>`  | 2 | 255 | 254 |
| `SmallRange<u32, 8>`  | 4 | 16,777,215 | 254 |
| `SmallRange<u32, 10>` | 4 | 4,194,303 | 1,022 |
| `SmallRange<u32, 16>` | 4 | 65,535 | 65,534 |
| `SmallRange<u64, 16>` | 8 | 2<sup>48</sup> − 1 | 65,534 |
| `SmallRange<u64, 24>` | 8 | 2<sup>40</sup> − 1 | 2<sup>24</sup> − 2 |
| `SmallRange<u64, 32>` | 8 | 2<sup>32</sup> − 1 | 2<sup>32</sup> − 2 |

`LEN_BITS` must be at least 1 and less than the storage width; anything else
fails to compile. `MAX_START` and `MAX_LEN` are associated constants.

```rust
use small_range::SmallRange;

assert_eq!(SmallRange::<u32, 8>::MAX_START, 16_777_215);
assert_eq!(SmallRange::<u32, 8>::MAX_LEN, 254);
assert_eq!(SmallRange::<u64, 32>::MAX_START, u32::MAX as usize);
```

## Construction

`new` panics when the range does not fit, in release builds too. `try_new`
returns `None` instead and compiles branch-free. Both check `start <= end`,
`start <= MAX_START` and `end - start <= MAX_LEN`.

```rust
use small_range::SmallRange;
type R = SmallRange<u32, 8>;

assert_eq!(R::try_new(3, 7), Some(R::new(3, 7)));
assert_eq!(R::try_new(7, 3), None);          // start > end
assert_eq!(R::try_new(0, 255), None);        // length 255 > MAX_LEN
assert_eq!(R::from_start_len(3, 4), R::new(3, 7));
assert_eq!(R::try_from(3..7), Ok(R::new(3, 7)));
assert!(R::try_from(0..1000).is_err());
```

The panic message names the type and the limits:

```text
small_range::small_range::SmallRange<u32, 8>: range 0..300 does not fit: max start is 16777215, max length is 254
```

`unsafe fn new_unchecked` skips the checks for callers that have already
proven the bounds.

## Using a range

```rust
use small_range::SmallRange;
type R = SmallRange<u64, 32>;

let r = R::new(5, 8);

// Accessors.
assert_eq!((r.start(), r.end(), r.len(), r.is_empty()), (5, 8, 3, false));
assert!(r.contains(6));
assert!(r.overlaps(R::new(7, 20)));

// Iteration yields usize.
let items: Vec<usize> = r.into_iter().collect();
assert_eq!(items, vec![5, 6, 7]);
for i in &r {
    assert!(r.contains(i));
}

// Indexing works on slices, arrays and str, and with the default `alloc`
// feature on Vec and String.
let v = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
assert_eq!(&v[r], &[5, 6, 7]);
assert_eq!(&"abcdefghij"[r], "fgh");

// Conversions to and from std.
let std: core::ops::Range<usize> = r.into();
assert_eq!(std, 5..8);
assert_eq!(r.to_range(), 5..8);

// Debug prints like a std range.
assert_eq!(format!("{r:?}"), "5..8");

// Ord sorts by start, then by length.
let mut v = [R::new(3, 9), R::new(1, 2), R::new(3, 4)];
v.sort();
assert_eq!(v, [R::new(1, 2), R::new(3, 4), R::new(3, 9)]);
```

`Option<SmallRange<..>>` gives three distinguishable states in one word:
`None`, an empty `Some`, and a populated `Some`. Memo tables use this to tell
"not computed" from "computed, nothing there".

## Encoding

The packed word is public API, reachable through `to_bits` and `from_bits`:

```text
SmallRange<u32, 8>
+--------------------------+----------+
|          start           |  len + 1 |   NonZero<u32>
|         24 bits          |  8 bits  |
+--------------------------+----------+
```

```rust
use small_range::SmallRange;
type R = SmallRange<u32, 8>;

assert_eq!(R::new(2, 5).to_bits(), (2 << 8) | 4);
assert_eq!(R::from_bits((2 << 8) | 4), Some(R::new(2, 5)));
assert_eq!(R::from_bits(0), None);        // the niche
assert_eq!(R::from_bits(2 << 8), None);   // zero length field
```

Because only the length field is biased, `start` decodes with a single shift,
and for byte-aligned splits the compiler reads it with a narrow load.

## Limitations

- **No `RangeBounds`.** The trait returns references to stored bounds, and
  this type stores none. Use `to_range()` or `.into()` where a `RangeBounds`
  is required, such as `Vec::drain` or `BTreeMap::range`.
- **Capacity is real.** A `SmallRange<u32, 8>` cannot hold a length of 255.
  `new` panics and `try_new` returns `None`; pick the split from your data.
- **`usize` storage is platform-sized.** `SmallRange<usize, N>` has 32-bit
  halves on 32-bit targets. Prefer `u32` or `u64` for a portable layout.
- **The byte order of the word is the platform's.** Serialize through the
  `serde` feature, which writes `start` and `end`, rather than through the
  raw bits.

## Features

- `alloc` (default): `Index` impls for `Vec<T>` and `String`. Slices, arrays
  and `str` work without it.
- `serde`: `Serialize` as `{ "start": .., "end": .. }` and `Deserialize` with
  the same validation as `try_new`.

The crate is `no_std` and has no required dependencies. Minimum supported
Rust version is 1.85.

## Migrating from 1.x

- `SmallRange<T>` is now `SmallRange<S, LEN_BITS>`. The 1.x layouts are
  `SmallRange<u16, 8>`, `SmallRange<u32, 16>`, `SmallRange<u64, 32>` and
  `SmallRange<usize, 32>` on 64-bit targets.
- All bounds are `usize` on the API side, whatever the storage type.
- `new` now checks its arguments in release builds. In 1.x an invalid range
  could produce a zero word and undefined behavior; 1.x should not be used.
- `start` is no longer biased, so `MAX_START` grew by one and the packed word
  of a given range changed. Nothing that persisted raw bits is compatible.
- `contains` and `overlaps` take their arguments by value.
- `Debug` prints `10..20` instead of a struct.
- `Ord`, `From<SmallRange> for Range<usize>`, `TryFrom<Range<usize>>`,
  `from_start_len`, `to_bits`, `from_bits` and slice indexing are new.

## Benchmarks

See [BENCHMARKS.md](BENCHMARKS.md) for bulk scans and for the usual
patterns: slicing, iterating, walking an adjacency list, memo lookups, random
access and sorting, each against `Range<usize>`, `Range<u32>` and a
hand-rolled `{u32, NonZeroU32}` struct. The short version: a graph walk over
2M nodes runs 1.7x faster with 8-byte nodes than with 32-byte ones,
three-state memo lookups gain 2x to 3x, sorting by the derived `Ord` is 1.8x
faster than a key extraction, and slicing or iterating through the range costs
the same as through `Range`. The benches draw their results as terminal
charts with `malevich`, and `cargo bench --bench charts -- --write`
regenerates the charts and tables in BENCHMARKS.md from the recorded runs.

## License

MIT
