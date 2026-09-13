//! Compares `Option<Range<usize>>`, `Option<Range<u32>>`, and two `SmallRange`
//! splits on scans, lookups and creation.
//!
//! `Option<Range<u32>>` is the fair size baseline: it holds the same value
//! space as `SmallRange<u64, 32>`. `Option<Range<usize>>` is what most code
//! actually stores.

mod common;

use criterion::{criterion_group, BenchmarkId, Criterion, Throughput};
use small_range::SmallRange;
use std::hint::black_box;
use std::ops::Range;

/// 8 bytes, 32 bits of start and 32 bits of length.
type Sym = SmallRange<u64, 32>;
/// 4 bytes, 24 bits of start and 8 bits of length.
type Asym = SmallRange<u32, 8>;

const SMALL_SIZE: usize = 1_000_000;
const MEDIUM_SIZE: usize = 10_000_000;
const LARGE_SIZE: usize = 100_000_000;

/// Starts stay below 2^24 and lengths below 200 so that every type can hold
/// the same data. One entry in ten is `None`.
fn bounds(i: usize) -> Option<(usize, usize)> {
    (i % 10 != 0).then(|| {
        let start = i % 1_000_000;
        (start, start + i % 200)
    })
}

fn gen<T>(count: usize, make: impl Fn(usize, usize) -> T) -> Vec<Option<T>> {
    (0..count)
        .map(|i| bounds(i).map(|(s, e)| make(s, e)))
        .collect()
}

fn gen_all(count: usize) -> Data {
    Data {
        std_usize: gen(count, |s, e| s..e),
        std_u32: gen(count, |s, e| s as u32..e as u32),
        sym: gen(count, Sym::new),
        asym: gen(count, Asym::new),
    }
}

struct Data {
    std_usize: Vec<Option<Range<usize>>>,
    std_u32: Vec<Option<Range<u32>>>,
    sym: Vec<Option<Sym>>,
    asym: Vec<Option<Asym>>,
}

fn bench_sum_len(c: &mut Criterion) {
    let mut group = c.benchmark_group("sum_len");
    for size in [SMALL_SIZE, MEDIUM_SIZE] {
        group.throughput(Throughput::Elements(size as u64));
        let d = gen_all(size);
        group.bench_with_input(
            BenchmarkId::new("Option<Range<usize>>", size),
            &d.std_usize,
            |b, data| {
                b.iter(|| {
                    data.iter()
                        .flatten()
                        .map(|r| r.end - r.start)
                        .fold(0usize, |a, x| a.wrapping_add(x))
                })
            },
        );
        group.bench_with_input(
            BenchmarkId::new("Option<Range<u32>>", size),
            &d.std_u32,
            |b, data| {
                b.iter(|| {
                    data.iter()
                        .flatten()
                        .map(|r| (r.end - r.start) as usize)
                        .fold(0usize, |a, x| a.wrapping_add(x))
                })
            },
        );
        group.bench_with_input(
            BenchmarkId::new("Option<SmallRange<u64, 32>>", size),
            &d.sym,
            |b, data| {
                b.iter(|| {
                    data.iter()
                        .flatten()
                        .map(|r| r.len())
                        .fold(0usize, |a, x| a.wrapping_add(x))
                })
            },
        );
        group.bench_with_input(
            BenchmarkId::new("Option<SmallRange<u32, 8>>", size),
            &d.asym,
            |b, data| {
                b.iter(|| {
                    data.iter()
                        .flatten()
                        .map(|r| r.len())
                        .fold(0usize, |a, x| a.wrapping_add(x))
                })
            },
        );
    }
    group.finish();
}

fn bench_sum_start(c: &mut Criterion) {
    let mut group = c.benchmark_group("sum_start");
    for size in [SMALL_SIZE, MEDIUM_SIZE] {
        group.throughput(Throughput::Elements(size as u64));
        let d = gen_all(size);
        group.bench_with_input(
            BenchmarkId::new("Option<Range<usize>>", size),
            &d.std_usize,
            |b, data| {
                b.iter(|| {
                    data.iter()
                        .flatten()
                        .map(|r| r.start)
                        .fold(0usize, |a, x| a.wrapping_add(x))
                })
            },
        );
        group.bench_with_input(
            BenchmarkId::new("Option<Range<u32>>", size),
            &d.std_u32,
            |b, data| {
                b.iter(|| {
                    data.iter()
                        .flatten()
                        .map(|r| r.start as usize)
                        .fold(0usize, |a, x| a.wrapping_add(x))
                })
            },
        );
        group.bench_with_input(
            BenchmarkId::new("Option<SmallRange<u64, 32>>", size),
            &d.sym,
            |b, data| {
                b.iter(|| {
                    data.iter()
                        .flatten()
                        .map(|r| r.start())
                        .fold(0usize, |a, x| a.wrapping_add(x))
                })
            },
        );
        group.bench_with_input(
            BenchmarkId::new("Option<SmallRange<u32, 8>>", size),
            &d.asym,
            |b, data| {
                b.iter(|| {
                    data.iter()
                        .flatten()
                        .map(|r| r.start())
                        .fold(0usize, |a, x| a.wrapping_add(x))
                })
            },
        );
    }
    group.finish();
}

fn bench_contains(c: &mut Criterion) {
    let mut group = c.benchmark_group("contains");
    for size in [SMALL_SIZE, MEDIUM_SIZE] {
        group.throughput(Throughput::Elements(size as u64));
        let d = gen_all(size);
        group.bench_with_input(
            BenchmarkId::new("Option<Range<usize>>", size),
            &d.std_usize,
            |b, data| {
                b.iter(|| {
                    data.iter()
                        .enumerate()
                        .filter(|(i, r)| {
                            r.as_ref()
                                .is_some_and(|r| r.contains(&(i % 1_000_000 + 50)))
                        })
                        .count()
                })
            },
        );
        group.bench_with_input(
            BenchmarkId::new("Option<Range<u32>>", size),
            &d.std_u32,
            |b, data| {
                b.iter(|| {
                    data.iter()
                        .enumerate()
                        .filter(|(i, r)| {
                            r.as_ref()
                                .is_some_and(|r| r.contains(&((i % 1_000_000 + 50) as u32)))
                        })
                        .count()
                })
            },
        );
        group.bench_with_input(
            BenchmarkId::new("Option<SmallRange<u64, 32>>", size),
            &d.sym,
            |b, data| {
                b.iter(|| {
                    data.iter()
                        .enumerate()
                        .filter(|(i, r)| r.is_some_and(|r| r.contains(i % 1_000_000 + 50)))
                        .count()
                })
            },
        );
        group.bench_with_input(
            BenchmarkId::new("Option<SmallRange<u32, 8>>", size),
            &d.asym,
            |b, data| {
                b.iter(|| {
                    data.iter()
                        .enumerate()
                        .filter(|(i, r)| r.is_some_and(|r| r.contains(i % 1_000_000 + 50)))
                        .count()
                })
            },
        );
    }
    group.finish();
}

fn bench_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("creation");
    let size = SMALL_SIZE;
    group.throughput(Throughput::Elements(size as u64));
    group.bench_function("Option<Range<usize>>", |b| {
        b.iter(|| gen(black_box(size), |s, e| black_box(s)..black_box(e)))
    });
    group.bench_function("Option<Range<u32>>", |b| {
        b.iter(|| {
            gen(black_box(size), |s, e| {
                black_box(s) as u32..black_box(e) as u32
            })
        })
    });
    group.bench_function("Option<SmallRange<u64, 32>>", |b| {
        b.iter(|| gen(black_box(size), |s, e| Sym::new(black_box(s), black_box(e))))
    });
    group.bench_function("Option<SmallRange<u32, 8>>", |b| {
        b.iter(|| {
            gen(black_box(size), |s, e| {
                Asym::new(black_box(s), black_box(e))
            })
        })
    });
    group.finish();
}

/// One dataset at a time so that peak memory stays at the largest type.
fn bench_large_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_scan");
    group.sample_size(10);
    let size = LARGE_SIZE;
    group.throughput(Throughput::Elements(size as u64));

    let data = gen(size, |s, e| s..e);
    group.bench_with_input(
        BenchmarkId::new("Option<Range<usize>>", size),
        &data,
        |b, data| {
            b.iter(|| {
                data.iter()
                    .flatten()
                    .map(|r| r.end - r.start)
                    .fold(0usize, |a, x| a.wrapping_add(x))
            })
        },
    );
    drop(data);

    let data = gen(size, |s, e| s as u32..e as u32);
    group.bench_with_input(
        BenchmarkId::new("Option<Range<u32>>", size),
        &data,
        |b, data| {
            b.iter(|| {
                data.iter()
                    .flatten()
                    .map(|r| (r.end - r.start) as usize)
                    .fold(0usize, |a, x| a.wrapping_add(x))
            })
        },
    );
    drop(data);

    let data = gen(size, Sym::new);
    group.bench_with_input(
        BenchmarkId::new("Option<SmallRange<u64, 32>>", size),
        &data,
        |b, data| {
            b.iter(|| {
                data.iter()
                    .flatten()
                    .map(|r| r.len())
                    .fold(0usize, |a, x| a.wrapping_add(x))
            })
        },
    );
    drop(data);

    let data = gen(size, Asym::new);
    group.bench_with_input(
        BenchmarkId::new("Option<SmallRange<u32, 8>>", size),
        &data,
        |b, data| {
            b.iter(|| {
                data.iter()
                    .flatten()
                    .map(|r| r.len())
                    .fold(0usize, |a, x| a.wrapping_add(x))
            })
        },
    );
    group.finish();
}

fn print_layout() {
    use std::mem::size_of;
    println!(
        "\nOption<Range<usize>>          {:>2} bytes",
        size_of::<Option<Range<usize>>>()
    );
    println!(
        "Option<Range<u32>>            {:>2} bytes",
        size_of::<Option<Range<u32>>>()
    );
    println!(
        "Option<SmallRange<u64, 32>>   {:>2} bytes",
        size_of::<Option<Sym>>()
    );
    println!(
        "Option<SmallRange<u32, 8>>    {:>2} bytes\n",
        size_of::<Option<Asym>>()
    );
}

fn bench_layout(c: &mut Criterion) {
    print_layout();
    c.bench_function("layout_printed", |b| b.iter(|| black_box(1)));
}

criterion_group!(
    benches,
    bench_layout,
    bench_sum_len,
    bench_sum_start,
    bench_contains,
    bench_creation,
    bench_large_scan,
);
fn main() {
    benches();
    Criterion::default().configure_from_args().final_summary();
    if !common::charts::skip_charts() {
        print!(
            "{}",
            common::charts::report(
                common::charts::SCAN_GROUPS,
                "bulk scans: speedup over Option<Range<usize>> (higher is better)"
            )
        );
    }
}
