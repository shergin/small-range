# Benchmarks

Three bench targets, all on identical deterministic data:

```bash
cargo bench --bench range_comparison      # bulk scans over Vec<Option<range>>
cargo bench --bench workloads             # the usual patterns
cargo bench --bench charts -- --write     # regenerate the charts and tables below
```

Every chart and table between `<!-- generated -->` markers in this file is
produced by the `charts` target from criterion's recorded estimates, never
typed by hand. The two criterion benches also print their charts when they
finish. Numbers below are from Apple Silicon, macOS, rustc 1.100 nightly.
Medians are shown; a dagger marks a cell whose 95% interval is wider than
±2% of the median.

Five representations are compared:

| type | holds |
|------|-------|
| `Option<Range<usize>>` | what most code stores |
| `Option<Range<u32>>` | the fair size baseline: the same value space as `SmallRange<u64, 32>` |
| `{ start: u32, len_plus_one: NonZeroU32 }` | the struct one would write by hand; same size and niche as `SmallRange<u64, 32>` |
| `Option<SmallRange<u64, 32>>` | 32 bits of start, 32 of length |
| `Option<SmallRange<u32, 8>>` | 24 bits of start, 8 of length |

<!-- generated:sizes -->
```text
                                         size in bytes
                            ██ Option<R>  ██ { u32, Option<R> } node
  30 ┤           ██████
     │           ██████
  25 ┤    ▁▁▁▁▁▁ ██████
b    │    ██████ ██████
y 20 ┤    ██████ ██████
t    │    ██████ ██████           ▁▁▁▁▁▁                            ▁▁▁▁▁▁
e 15 ┤    ██████ ██████           ██████                            ██████
s    │    ██████ ██████    ▅▅▅▅▅▅ ██████           ▅▅▅▅▅▅           ██████
  10 ┤    ██████ ██████    ██████ ██████    ▂▂▂▂▂▂ ██████    ▂▂▂▂▂▂ ██████           ▂▂▂▂▂▂
     │    ██████ ██████    ██████ ██████    ██████ ██████    ██████ ██████           ██████
   5 ┤    ██████ ██████    ██████ ██████    ██████ ██████    ██████ ██████     ▆▆▆▆▆ ██████
   0 ┤    ██████ ██████    ██████ ██████    ██████ ██████    ██████ ██████     █████ ██████
     └──────────────────────────────────────────────────────────────────────────────────────────
           Range<usize>      Range<u32>       hand-rolled      SR<u64, 32>      SR<u32, 8>
```
<!-- /generated -->

## Bulk scans

`range_comparison` scans a `Vec<Option<range>>` with one entry in ten `None`,
starts below one million and lengths below 200: summing `len()`, summing
`start()`, `contains(i + 50)`, building the vector, and a 100M-entry scan
that leaves every cache.

<!-- generated:scans_table -->
| benchmark | `Range<usize>` | `Range<u32>` | `{u32, NonZeroU32}` | `SmallRange<u64, 32>` | `SmallRange<u32, 8>` |
|---|---:|---:|---:|---:|---:|
| sum_len / 1000000 | 954.4 µs | 407.7 µs |  | 199.7 µs | 117.5 µs |
| sum_len / 10000000 | 9.37 ms | 3.93 ms |  | 1.98 ms | 1.18 ms |
| sum_start / 1000000 | 406.3 µs | 407.0 µs |  | 122.0 µs | 58.8 µs |
| sum_start / 10000000 | 4.06 ms | 3.96 ms |  | 1.33 ms | 673.8 µs |
| contains / 1000000 | 781.7 µs | 805.4 µs |  | 523.9 µs | 572.0 µs |
| contains / 10000000 | 7.90 ms | 7.79 ms |  | 5.23 ms | 5.76 ms |
| creation | 1.95 ms | 1.09 ms |  | 1.61 ms | 1.65 ms |
| large_scan / 100000000 | 94.14 ms | 40.01 ms |  | 20.02 ms | 11.99 ms |
<!-- /generated -->

<!-- generated:scans_speedup -->
```text
                       bulk scans: speedup over Option<Range<usize>> (higher is better)
     ██ Range<usize>  ██ Range<u32>  ██ {u32, NonZeroU32}  ██ SmallRange<u64, 32>  ██ SmallRange<u32, 8>
  8 ┤                 ▇▇▇                                                                             ▅▅▅
    │                 ███                                                                             ███
  7 ┤                 ███                                                                             ███
    │                 ███                                                                             ███
  6 ┤                 ███                 ▅▅▅                                                         ███
    │                 ███                 ███                                                         ███
  5 ┤                 ███                 ███                                                         ███
    │              ▆▆▆███                 ███                                                      ▅▅▅███
x   │              ██████                 ███                                                      ██████
  4 ┤              ██████                 ███                                                      ██████
    │              ██████                 ███                                                      ██████
  3 ┤              ██████              ██████                                                      ██████
    │        ▄▄▄   ██████              ██████                                               ▄▄▄    ██████
  2 ┤        ███   ██████              ██████                        ▇▇▇ ▂▂                 ███    ██████
    │        ███   ██████              ██████              ▅▅▅▂▂▂    ███ ██                 ███    ██████
  1 ┤     ▄▄▄███   ██████     ▄▄ ▄▄▄   ██████    ▄▄▄ ▄▄    ██████    ███ ██    ▇▇▇▇▇▇    ▄▄▄███    ██████
    │     ██████   ██████     ██ ███   ██████    ███ ██    ██████    ███ ██    ██████    ██████    ██████
  0 ┤     ██████   ██████     ██ ███   ██████    ███ ██    ██████    ███ ██    ██████    ██████    ██████
    └─────────────────────────────────────────────────────────────────────────────────────────────────────────
              sum_len            sum_start           contains            creation           large_scan
```
<!-- /generated -->

- Scan time tracks bytes moved once the loop vectorizes. The 100M-entry scan
  reads 2.4 GB for `Range<usize>`, 1.2 GB for `Range<u32>`, 800 MB for
  `SmallRange<u64, 32>` and 400 MB for `SmallRange<u32, 8>`.
- `contains` is a single unsigned compare of `value - start` against `len`,
  cheaper than the two compares `Range::contains` performs.
- `SmallRange::new` validates in release builds, three predictable branches
  per element, so creation trails the unchecked `Range<u32>`.
- Whether a scan vectorizes matters more than the encoding. `len()` is
  computed as `(word - 1) & mask` because that form lets LLVM vectorize
  loops over `Option<SmallRange<..>>`; `(word & mask) - 1` did not in this
  loop shape and ran six times slower. Read these rows as true for these
  loops, not for ranges in general.

## Workloads

`workloads` measures the ways ranges are usually used: slicing a buffer
through the range, iterating its indices, walking an adjacency list stored
in graph nodes at two sizes, three-state memo lookups, random access into a
10M table, and sorting by key or by derived `Ord`.

<!-- generated:workloads_table -->
| benchmark | `Range<usize>` | `Range<u32>` | `{u32, NonZeroU32}` | `SmallRange<u64, 32>` | `SmallRange<u32, 8>` |
|---|---:|---:|---:|---:|---:|
| slice_sum | 15.02 ms | 14.89 ms | 14.66 ms | 14.66 ms | 14.62 ms |
| iterate_indices | 1.10 ms | 1.09 ms | 1.06 ms | 1.05 ms | 1.07 ms |
| graph_walk / 131072 | 14.64 ms | 14.20 ms | 14.51 ms | 14.35 ms | 14.01 ms |
| graph_walk / 2097152 | 59.32 ms | 47.28 ms | 40.95 ms | 46.58 ms | 33.92 ms |
| memo_lookup | 13.73 ms | 13.71 ms | 6.22 ms | 4.67 ms | 6.21 ms |
| random_access | 14.36 ms | 14.53 ms | 12.50 ms | 11.15 ms | 9.97 ms |
| sort_by_key | 20.78 ms | 19.41 ms | 18.55 ms | 18.41 ms | 20.46 ms |
| sort_native_ord |  |  | 18.40 ms | 11.82 ms | 11.30 ms |
<!-- /generated -->

<!-- generated:workloads_speedup -->
```text
                       workloads: speedup over Option<Range<usize>> (higher is better)
     ██ Range<usize>  ██ Range<u32>  ██ {u32, NonZeroU32}  ██ SmallRange<u64, 32>  ██ SmallRange<u32, 8>
  3.0 ┤                                                             ▅▅
      │                                                             ██
      │                                                             ██
  2.5 ┤                                                             ██
      │                                                          ▃▃▃██ ▃▃
      │                                                          █████ ██
  2.0 ┤                                                          █████ ██
      │                                               ▅▅         █████ ██
x     │                                               ██         █████ ██
  1.5 ┤                                          ▇▇   ██         █████ ██              ▆▆
      │                                       ▅▅▅██▆▆▆██         █████ ██           ▇▇▇██
  1.0 ┤    ▁▁▂▂▂▂▂ ▂▂▃▃▃   ▁▁▁▂▂▃▃▃▄▄ ▃▃    ▁▁██████████    ▁▁ ▁▁█████ ██   ▁▁▁▁▁ ███████    ▁▁▅▅▅▇▇ ▇▇▂▂▂
      │    ███████ █████   ██████████ ██    ████████████    ██ ███████ ██   █████ ███████    ███████ █████
      │    ███████ █████   ██████████ ██    ████████████    ██ ███████ ██   █████ ███████    ███████ █████
  0.5 ┤    ███████ █████   ██████████ ██    ████████████    ██ ███████ ██   █████ ███████    ███████ █████
      │    ███████ █████   ██████████ ██    ████████████    ██ ███████ ██   █████ ███████    ███████ █████
      │    ███████ █████   ██████████ ██    ████████████    ██ ███████ ██   █████ ███████    ███████ █████
  0.0 ┤    ███████ █████   ██████████ ██    ████████████    ██ ███████ ██   █████ ███████    ███████ █████
      └───────────────────────────────────────────────────────────────────────────────────────────────────────
             slice_sum     iterate_indices   graph_walk      memo_lookup     random_access    sort_by_key
```
<!-- /generated -->

- **Slicing and iterating are a wash.** Once the range is decoded the work
  is in the slice or the loop body, and every type decodes in one or two
  instructions.
- **Node size matters once the graph leaves cache.** With 128K nodes every
  type fits and the walk is latency-bound at the same speed. With 2M nodes
  the 8-byte `SmallRange<u32, 8>` node walks 1.7x faster than the 32-byte
  `Range<usize>` node, and the 12-byte hand-rolled node beats the 16-byte
  `SmallRange<u64, 32>` node, as the sizes predict.

<!-- generated:graph_walk -->
```text
                            graph_walk: ns per element (lower is better)
██ Range<usize>  ██ Range<u32>  ██ {u32, NonZeroU32}  ██ SmallRange<u64, 32>  ██ SmallRange<u32, 8>
  60 ┤                                                   ▇▇▇▇▇▇
     │                                                   ██████
  50 ┤                                                   ██████ ▁▁▁▁▁▁
     │                                                   ██████ ██████       ▇▇▇▇▇▇
  40 ┤                                                   ██████ ██████ ▅▅▅▅▅▅██████
     │                                                   ██████ ██████ ████████████
n    │                                                   ██████ ██████ ████████████ ██████
s 30 ┤                                                   ██████ ██████ ████████████ ██████
     │                                                   ██████ ██████ ████████████ ██████
  20 ┤                                                   ██████ ██████ ████████████ ██████
     │          ▅▅▅▅▅▅▄▄▄▄▄▄ ▅▅▅▅▅▅ ▄▄▄▄▄▄▄▄▄▄▄▄         ██████ ██████ ████████████ ██████
  10 ┤          ████████████ ██████ ████████████         ██████ ██████ ████████████ ██████
     │          ████████████ ██████ ████████████         ██████ ██████ ████████████ ██████
   0 ┤          ████████████ ██████ ████████████         ██████ ██████ ████████████ ██████
     └──────────────────────────────────────────────────────────────────────────────────────────────
                             131072                                    2097152
```
<!-- /generated -->

- **Three-state memo tables gain 2x to 3x.** Checking `None`, then empty,
  then length is a word compare and a masked compare on the packed types,
  against a tag byte plus two loads and a subtract on `Range`.
- **Random access follows bytes per entry**, 4 through 24.
- **A derived `Ord` sorts 1.8x faster** than a `(start, len)` key
  extraction, because the comparison is one integer compare. The
  hand-rolled struct's derived `Ord` compares two fields and gains nothing.

## Reading the numbers

- The gains come from bytes, not from per-element compute. A `SmallRange`
  accessor is one or two instructions, and so is a field load from `Range`.
- Workloads that touch one range at a time see none of the scan speedups.
  For them the benefit is the smaller node and the free `Option`.
- One machine, one core, one allocator, one length distribution. Rerun
  before quoting numbers on other hardware.
