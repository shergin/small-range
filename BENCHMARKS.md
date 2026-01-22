# Benchmark Results

Comprehensive benchmarks comparing `Option<Range<usize>>` vs `Option<SmallRange<usize>>` with up to 100 million entries.

## Memory Layout

| Type | Size (bytes) |
|------|--------------|
| `Range<usize>` | 16 |
| `Option<Range<usize>>` | 24 |
| `SmallRange<usize>` | 8 |
| `Option<SmallRange<usize>>` | 8 |

**For 100 million entries:**

| Type | Memory Usage |
|------|--------------|
| `Option<Range<usize>>` | 2,400 MB |
| `Option<SmallRange<usize>>` | 800 MB |
| **Savings** | **3x** |

## Performance Results

All benchmarks run on Apple Silicon with `cargo bench --bench range_comparison`.

### Sequential Read: Sum Lengths

Iterating through all entries and summing `end - start` (Range) or `len()` (SmallRange).

| Dataset Size | `Option<Range<usize>>` | `Option<SmallRange<usize>>` | Speedup |
|--------------|------------------------|-----------------------------| --------|
| 1 million | 567 µs | 201 µs | **2.8x** |
| 10 million | 5.9 ms | 2.0 ms | **2.9x** |
| 100 million | 58.7 ms | 21-25 ms | **2.4x** |

### Sequential Read: Sum Start Values

Iterating through all entries and summing `start` values.

| Dataset Size | `Option<Range<usize>>` | `Option<SmallRange<usize>>` | Winner |
|--------------|------------------------|-----------------------------| -------|
| 1 million | 451 µs | 1.26 ms | Range 2.8x |
| 10 million | 4.4 ms | 12.6 ms | Range 2.9x |

> **Note:** Range is faster here because `start` is stored directly. SmallRange must decode (shift + subtract).

### Sequential Contains Check

Checking `contains(i + 50)` for each entry.

| Dataset Size | `Option<Range<usize>>` | `Option<SmallRange<usize>>` | Winner |
|--------------|------------------------|-----------------------------| -------|
| 1 million | 545 µs | 745 µs | Range 1.4x |
| 10 million | 5.9 ms | 7.5 ms | Range 1.3x |

> **Note:** Range is faster because `contains()` requires accessing both `start` and `end`, which SmallRange must decode.

### Creation Performance

Creating 1 million `Option<T>` entries.

| Type | Time | Throughput |
|------|------|------------|
| `Option<Range<usize>>` | 2.1 ms | 464 Melem/s |
| `Option<SmallRange<usize>>` | 946 µs | 1.06 Gelem/s |
| **Speedup** | **2.3x** | |

### Large Dataset Scan (100 Million Entries)

This benchmark demonstrates real-world cache effects with datasets exceeding CPU cache.

| Type | Time | Throughput | Memory |
|------|------|------------|--------|
| `Option<Range<usize>>` | 58.7 ms | 1.70 Gelem/s | 2.4 GB |
| `Option<SmallRange<usize>>` | 21-25 ms | 4.0 Gelem/s | 800 MB |
| **Speedup** | **2.4x** | | **3x less** |

## When to Use SmallRange

**SmallRange excels when:**
- Working with large collections (millions of ranges)
- Memory bandwidth is the bottleneck
- Cache efficiency matters
- Frequently computing `len()` or `is_empty()`
- Creating many ranges

**Standard Range is better when:**
- Frequently accessing `start` or `end` directly
- Using `contains()` in tight loops
- Working with small collections where cache effects don't matter

## Running Benchmarks

```bash
# Run all benchmarks
cargo bench --bench range_comparison

# Run specific benchmark
cargo bench --bench range_comparison -- "large_sequential"

# Quick test mode (verify benchmarks work)
cargo bench --bench range_comparison -- --test
```

## Hardware

These benchmarks were run on:
- Apple Silicon (ARM64)
- macOS
- Rust stable

Results may vary on different hardware. The relative performance differences should be consistent, but absolute numbers will differ.
