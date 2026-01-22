# small_range

A compact range type: 50% smaller than Range<T> with zero-cost Option.

> Imagine you need to store `Option<Range<usize>>` millions of times. That's 192 bits per instance. With this library you can shrink that to just 64 bits (see tradeoffs).

## Motivation

Standard `Range<T>` stores start and end as separate fields, requiring `2 * size_of::<T>()` bytes. For applications managing millions of ranges (spatial indexing, text processing, interval trees), this overhead adds up.

`SmallRange<T>` packs start and length into a single value of type `T`:

| Type | Size | vs `Range<T>` | Max Start | Max Length |
|------|------|---------------|-----------|------------|
| `SmallRange<u16>` | 2 bytes | **vs 4 bytes (50%)** | 254 | 254 |
| `SmallRange<u32>` | 4 bytes | **vs 8 bytes (50%)** | 65,534 | 65,534 |
| `SmallRange<u64>` | 8 bytes | **vs 16 bytes (50%)** | ~4.29B | ~4.29B |
| `SmallRange<usize>` | 8 bytes | **vs 16 bytes (50%)** | ~4.29B | ~4.29B |

Plus: `Option<SmallRange<T>>` is the same size as `SmallRange<T>` due to niche optimization.

```rust
use small_range::SmallRange;
use core::mem::size_of;

// 50% space savings
assert_eq!(size_of::<SmallRange<u64>>(), 8);      // vs Range<u64> = 16 bytes
assert_eq!(size_of::<SmallRange<usize>>(), 8);    // vs Range<usize> = 16 bytes

// Option adds no overhead (niche optimization)
assert_eq!(size_of::<SmallRange<u64>>(), size_of::<Option<SmallRange<u64>>>());
```

## Use Cases

Half the memory footprint means better cache locality. Ideal for:

- **HFT / Low-latency systems**: Order book ranges, tick intervals, buffer slices
- **Game engines**: Entity ID ranges, spatial partitions, collision bounds
- **Compilers & IDEs**: Source spans, token ranges, AST extents
- **Databases**: Index ranges, row bounds, interval trees

## API Overview

```rust
use small_range::SmallRange;

// Create a range (defaults to u64 storage)
let range = SmallRange::new(10u64, 20u64);

// Access bounds
assert_eq!(range.start(), 10u64);
assert_eq!(range.end(), 20u64);   // exclusive, like std Range
assert_eq!(range.len(), 10);
assert!(!range.is_empty());

// Iterate
for i in &range {
    println!("{}", i);
}

// Convert to standard Range
let std_range: core::ops::Range<u64> = range.to_range();
```

### Storage Types

```rust
use small_range::SmallRange;
use core::mem::size_of;

// SmallRange<u16>: 2 bytes, values 0-254
let r16 = SmallRange::<u16>::new(0, 100);
assert_eq!(size_of::<SmallRange<u16>>(), 2);

// SmallRange<u32>: 4 bytes, values 0-65,534
let r32 = SmallRange::<u32>::new(0, 1000);
assert_eq!(size_of::<SmallRange<u32>>(), 4);

// SmallRange<u64>: 8 bytes, values 0-4,294,967,294 (default)
let r64 = SmallRange::<u64>::new(0, 1_000_000);
assert_eq!(size_of::<SmallRange<u64>>(), 8);

// SmallRange<usize>: convenient for slice indexing
let r_usize = SmallRange::<usize>::new(0, 100);
let data = vec![0; 200];
let slice = &data[r_usize.start()..r_usize.end()];
```

## Limitations

### Value Constraints

Start and length must each fit in half the storage width minus 1:
- `SmallRange<u16>`: max start = 254, max length = 254
- `SmallRange<u32>`: max start = 65,534, max length = 65,534
- `SmallRange<u64>`: max start = 4,294,967,294, max length = 4,294,967,294

```rust,ignore
use small_range::SmallRange;

// This will panic in debug mode (start exceeds capacity)
let invalid = SmallRange::<u16>::new(255, 256);
```

### No `RangeBounds` Implementation

`SmallRange` does not implement `RangeBounds<T>` because the trait requires returning references (`Bound<&T>`), but our values are computed from packed bits -- there's no stored `T` to reference.

**Workaround**: Use `.to_range()` which returns `Range<T>`, and `Range<T>` implements `RangeBounds<T>`:

```rust
use small_range::SmallRange;
use core::ops::RangeBounds;

let small = SmallRange::new(10u64, 20u64);

// to_range() gives full RangeBounds support
let range = small.to_range();
assert!(range.contains(&15));
assert!(!range.contains(&25));
```

### Sealed Trait

The `SmallRangeStorage` trait is sealed -- only `u16`, `u32`, `u64`, and `usize` are supported.

## Implementation Details

### Encoding Scheme

Values are packed as `(start+1, length+1)` where start is in the high bits and length is in the low bits:

```text
SmallRange<u32> in 4 bytes:
+----------------+----------------+
|   start + 1    |  length + 1    |  -> NonZero<u32>
|   (16 bits)    |   (16 bits)    |
+----------------+----------------+

SmallRange<u64> in 8 bytes:
+--------------------------------+--------------------------------+
|           start + 1            |          length + 1            |  -> NonZero<u64>
|           (32 bits)            |           (32 bits)            |
+--------------------------------+--------------------------------+
```

By adding 1 to both start and length:
- Both halves are always >= 1, so the packed value is never zero
- Zero is reserved for `Option::None` (niche optimization)
- `len()` is a simple mask + subtract operation

### Performance

All operations are `#[inline]` and compile to minimal assembly:
- `new()`: Subtract, two adds, shift, OR
- `start()`: Shift, mask, subtract
- `end()`: Shift, mask, subtract, add
- `len()`: Mask, subtract

No heap allocation, no branches, no function calls.

**[See full benchmark results](BENCHMARKS.md)** comparing `Option<SmallRange>` vs `Option<Range>` with 100 million entries:
- **2.4x faster** sequential scans (better cache utilization)
- **2.3x faster** creation
- **3x less memory** (800 MB vs 2.4 GB)

### Memory Layout

```rust
use small_range::SmallRange;
use core::mem::{size_of, align_of};

// Transparent wrapper around NonZero<T>
assert_eq!(size_of::<SmallRange<u64>>(), 8);
assert_eq!(align_of::<SmallRange<u64>>(), 8);

// Niche optimization works
assert_eq!(size_of::<Option<SmallRange<u64>>>(), 8);
```

## Quick Reference

### Construction

| Method | Description |
|--------|-------------|
| `SmallRange::new(start, end)` | Create from start and end values |
| `SmallRange::default()` | Empty range (0, 0) |

### Accessors

| Method | Returns | Description |
|--------|---------|-------------|
| `start()` | `T` | Start bound (inclusive) |
| `end()` | `T` | End bound (exclusive) |
| `len()` | `usize` | Number of elements |
| `is_empty()` | `bool` | True if start == end |
| `to_range()` | `Range<T>` | Convert to std Range |

### Iteration

| Method | Yields | Description |
|--------|--------|-------------|
| `for x in range` | `T` | Consuming iteration |
| `for x in &range` | `T` | Borrowing iteration |

### Traits

| Trait | Notes |
|-------|-------|
| `Clone`, `Copy` | Zero-cost copy |
| `PartialEq`, `Eq` | Bitwise comparison |
| `Hash` | Based on packed bits |
| `Default` | Empty range (0, 0) |
| `Debug` | Shows start and end |
| `IntoIterator` | For both owned and borrowed |

## When to Use SmallRange

**Good fit:**
- Storing many ranges where memory matters (50% savings)
- Need `Option<Range>` with zero discriminant overhead
- Indices that fit in half the storage width (e.g., u32 indices in u64 storage)
- `no_std` environments (only depends on `num-traits`)

**Poor fit:**
- Need `RangeBounds` trait directly (use `.to_range()` as workaround)
- Values exceed half-width capacity
- Single ranges where memory isn't a concern (just use `Range<T>`)

## License

MIT
