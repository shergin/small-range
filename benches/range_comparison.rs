//! Benchmark comparing Option<Range<usize>> vs Option<SmallRange<usize>>
//!
//! This benchmark demonstrates the performance benefits of SmallRange
//! when working with large collections due to better cache utilization.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use small_range::SmallRange;
use std::hint::black_box;
use std::ops::Range;

// Number of elements for benchmarks
// 100 million entries: ~2.4GB for Option<Range<usize>>, ~800MB for Option<SmallRange<usize>>
const SMALL_SIZE: usize = 1_000_000; // 1M for quick iteration
const MEDIUM_SIZE: usize = 10_000_000; // 10M
const LARGE_SIZE: usize = 100_000_000; // 100M for cache pressure

/// Generate test data for Option<Range<usize>>
fn generate_std_ranges(count: usize) -> Vec<Option<Range<usize>>> {
    (0..count)
        .map(|i| {
            if i % 10 == 0 {
                None // 10% None values
            } else {
                Some(i..(i + (i % 1000)))
            }
        })
        .collect()
}

/// Generate test data for Option<SmallRange<usize>>
fn generate_small_ranges(count: usize) -> Vec<Option<SmallRange<usize>>> {
    (0..count)
        .map(|i| {
            if i % 10 == 0 {
                None // 10% None values
            } else {
                Some(SmallRange::new(i, i + (i % 1000)))
            }
        })
        .collect()
}

/// Benchmark: Sequential read - sum all lengths
fn bench_sequential_read_sum(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequential_read_sum");

    for size in [SMALL_SIZE, MEDIUM_SIZE] {
        group.throughput(Throughput::Elements(size as u64));

        // Benchmark Option<Range<usize>>
        let std_data = generate_std_ranges(size);
        group.bench_with_input(
            BenchmarkId::new("Option<Range<usize>>", size),
            &std_data,
            |b, data| {
                b.iter(|| {
                    let mut sum: usize = 0;
                    for range in data.iter() {
                        if let Some(r) = range {
                            sum += r.end - r.start;
                        }
                    }
                    black_box(sum)
                })
            },
        );
        drop(std_data);

        // Benchmark Option<SmallRange<usize>>
        let small_data = generate_small_ranges(size);
        group.bench_with_input(
            BenchmarkId::new("Option<SmallRange<usize>>", size),
            &small_data,
            |b, data| {
                b.iter(|| {
                    let mut sum: usize = 0;
                    for range in data.iter() {
                        if let Some(r) = range {
                            sum += r.len();
                        }
                    }
                    black_box(sum)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark: Sequential read - sum all start values
fn bench_sequential_read_starts(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequential_read_starts");

    for size in [SMALL_SIZE, MEDIUM_SIZE] {
        group.throughput(Throughput::Elements(size as u64));

        let std_data = generate_std_ranges(size);
        group.bench_with_input(
            BenchmarkId::new("Option<Range<usize>>", size),
            &std_data,
            |b, data| {
                b.iter(|| {
                    let mut sum: usize = 0;
                    for range in data.iter() {
                        if let Some(r) = range {
                            sum = sum.wrapping_add(r.start);
                        }
                    }
                    black_box(sum)
                })
            },
        );
        drop(std_data);

        let small_data = generate_small_ranges(size);
        group.bench_with_input(
            BenchmarkId::new("Option<SmallRange<usize>>", size),
            &small_data,
            |b, data| {
                b.iter(|| {
                    let mut sum: usize = 0;
                    for range in data.iter() {
                        if let Some(r) = range {
                            sum = sum.wrapping_add(r.start());
                        }
                    }
                    black_box(sum)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark: Sequential read - check contains
fn bench_sequential_contains(c: &mut Criterion) {
    let mut group = c.benchmark_group("sequential_contains");

    for size in [SMALL_SIZE, MEDIUM_SIZE] {
        group.throughput(Throughput::Elements(size as u64));

        let std_data = generate_std_ranges(size);
        group.bench_with_input(
            BenchmarkId::new("Option<Range<usize>>", size),
            &std_data,
            |b, data| {
                b.iter(|| {
                    let mut count: usize = 0;
                    for (i, range) in data.iter().enumerate() {
                        if let Some(r) = range {
                            if r.contains(&(i + 50)) {
                                count += 1;
                            }
                        }
                    }
                    black_box(count)
                })
            },
        );
        drop(std_data);

        let small_data = generate_small_ranges(size);
        group.bench_with_input(
            BenchmarkId::new("Option<SmallRange<usize>>", size),
            &small_data,
            |b, data| {
                b.iter(|| {
                    let mut count: usize = 0;
                    for (i, range) in data.iter().enumerate() {
                        if let Some(r) = range {
                            if r.contains(i + 50) {
                                count += 1;
                            }
                        }
                    }
                    black_box(count)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark: Memory bandwidth test with large dataset
/// This really shows cache effects with 100M+ entries
fn bench_large_sequential_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_sequential_scan");
    group.sample_size(10); // Fewer samples for large data

    let size = LARGE_SIZE;
    group.throughput(Throughput::Elements(size as u64));

    // Option<Range<usize>>: 24 bytes each = 2.4 GB
    println!(
        "Generating {} Option<Range<usize>> entries (~{} GB)...",
        size,
        (size * 24) / 1_000_000_000
    );
    let std_data = generate_std_ranges(size);

    group.bench_with_input(
        BenchmarkId::new("Option<Range<usize>>", size),
        &std_data,
        |b, data| {
            b.iter(|| {
                let mut sum: usize = 0;
                for range in data.iter() {
                    if let Some(r) = range {
                        sum = sum.wrapping_add(r.end - r.start);
                    }
                }
                black_box(sum)
            })
        },
    );

    // Free memory before allocating next dataset
    drop(std_data);

    // Option<SmallRange<usize>>: 8 bytes each = 800 MB
    println!(
        "Generating {} Option<SmallRange<usize>> entries (~{} MB)...",
        size,
        (size * 8) / 1_000_000
    );
    let small_data = generate_small_ranges(size);

    group.bench_with_input(
        BenchmarkId::new("Option<SmallRange<usize>>", size),
        &small_data,
        |b, data| {
            b.iter(|| {
                let mut sum: usize = 0;
                for range in data.iter() {
                    if let Some(r) = range {
                        sum = sum.wrapping_add(r.len());
                    }
                }
                black_box(sum)
            })
        },
    );

    group.finish();
}

/// Benchmark: Creation performance
fn bench_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("creation");

    let size = SMALL_SIZE;
    group.throughput(Throughput::Elements(size as u64));

    group.bench_function("Option<Range<usize>>", |b| {
        b.iter(|| {
            let data: Vec<Option<Range<usize>>> = (0..size)
                .map(|i| {
                    if i % 10 == 0 {
                        None
                    } else {
                        Some(black_box(i)..black_box(i + 100))
                    }
                })
                .collect();
            black_box(data)
        })
    });

    group.bench_function("Option<SmallRange<usize>>", |b| {
        b.iter(|| {
            let data: Vec<Option<SmallRange<usize>>> = (0..size)
                .map(|i| {
                    if i % 10 == 0 {
                        None
                    } else {
                        Some(SmallRange::new(black_box(i), black_box(i + 100)))
                    }
                })
                .collect();
            black_box(data)
        })
    });

    group.finish();
}

/// Print memory usage comparison
fn print_memory_comparison() {
    use std::mem::size_of;

    println!("\n=== Memory Layout Comparison ===\n");
    println!("Type                          | Size (bytes)");
    println!("------------------------------|-------------");
    println!(
        "Range<usize>                  | {:>12}",
        size_of::<Range<usize>>()
    );
    println!(
        "Option<Range<usize>>          | {:>12}",
        size_of::<Option<Range<usize>>>()
    );
    println!(
        "SmallRange<usize>             | {:>12}",
        size_of::<SmallRange<usize>>()
    );
    println!(
        "Option<SmallRange<usize>>     | {:>12}",
        size_of::<Option<SmallRange<usize>>>()
    );
    println!();
    println!("For 100 million entries:");
    println!(
        "  Option<Range<usize>>:       {:>6} MB",
        (100_000_000 * size_of::<Option<Range<usize>>>()) / 1_000_000
    );
    println!(
        "  Option<SmallRange<usize>>:  {:>6} MB",
        (100_000_000 * size_of::<Option<SmallRange<usize>>>()) / 1_000_000
    );
    println!(
        "  Memory savings:             {:>6}x",
        size_of::<Option<Range<usize>>>() / size_of::<Option<SmallRange<usize>>>()
    );
    println!();
}

fn bench_print_memory_info(c: &mut Criterion) {
    // Print memory comparison once at the start
    print_memory_comparison();

    // Dummy benchmark just to have the function
    c.bench_function("memory_info_printed", |b| b.iter(|| black_box(1)));
}

criterion_group!(
    benches,
    bench_print_memory_info,
    bench_sequential_read_sum,
    bench_sequential_read_starts,
    bench_sequential_contains,
    bench_creation,
    bench_large_sequential_scan,
);

criterion_main!(benches);

// Run this to see memory comparison before benchmarks
#[test]
fn show_memory_comparison() {
    print_memory_comparison();
}
