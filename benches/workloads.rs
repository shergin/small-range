//! Benchmarks of the ways ranges are usually used, rather than bulk scans:
//! slicing a buffer, iterating indices, walking an adjacency list stored in
//! graph nodes, three-state memo lookups, random access and sorting.
//!
//! Five representations run on identical data:
//! `Option<Range<usize>>`, `Option<Range<u32>>`, a hand-rolled
//! `{ start: u32, len_plus_one: NonZeroU32 }` struct, `SmallRange<u64, 32>`
//! and `SmallRange<u32, 8>`. See `common/mod.rs` for the kernels.

mod common;

use common::*;
use criterion::measurement::WallTime;
use criterion::{criterion_group, BatchSize, BenchmarkGroup, BenchmarkId, Criterion, Throughput};
use small_range::SmallRange;
use std::hint::black_box;
use std::mem::size_of;
use std::ops::Range;

fn bench_slice_sum(c: &mut Criterion) {
    const COUNT: usize = 1_000_000;
    const BUFFER: usize = 1 << 20;
    let specs = range_specs(COUNT, BUFFER, 64);
    let data: Vec<u8> = (0..BUFFER).map(|i| (i * 31 % 251) as u8).collect();
    let mut group = c.benchmark_group("slice_sum");
    group.throughput(Throughput::Elements(COUNT as u64));

    fn run<R: Repr>(group: &mut BenchmarkGroup<WallTime>, specs: &[(usize, usize)], data: &[u8]) {
        let rs = ranges::<R>(specs);
        group.bench_function(R::NAME, |b| {
            b.iter(|| slice_sum(black_box(&rs), black_box(data)))
        });
    }
    run::<Range<usize>>(&mut group, &specs, &data);
    run::<Range<u32>>(&mut group, &specs, &data);
    run::<HandRolled>(&mut group, &specs, &data);
    run::<SmallRange<u64, 32>>(&mut group, &specs, &data);
    run::<SmallRange<u32, 8>>(&mut group, &specs, &data);
    group.finish();
}

// ---------------------------------------------------------------------------
// Workload: iterate the indices of each range
// ---------------------------------------------------------------------------

fn bench_iterate(c: &mut Criterion) {
    const COUNT: usize = 1_000_000;
    let specs = range_specs(COUNT, 1 << 20, 64);
    let mut group = c.benchmark_group("iterate_indices");
    group.throughput(Throughput::Elements(COUNT as u64));

    fn run<R: Repr>(group: &mut BenchmarkGroup<WallTime>, specs: &[(usize, usize)]) {
        let rs = ranges::<R>(specs);
        group.bench_function(R::NAME, |b| b.iter(|| iterate_indices(black_box(&rs))));
    }
    run::<Range<usize>>(&mut group, &specs);
    run::<Range<u32>>(&mut group, &specs);
    run::<HandRolled>(&mut group, &specs);
    run::<SmallRange<u64, 32>>(&mut group, &specs);
    run::<SmallRange<u32, 8>>(&mut group, &specs);
    group.finish();
}

// ---------------------------------------------------------------------------
// Workload: adjacency list stored in graph nodes, random visits
// ---------------------------------------------------------------------------

fn bench_graph_walk(c: &mut Criterion) {
    const STEPS: usize = 1_000_000;
    let mut group = c.benchmark_group("graph_walk");
    group.throughput(Throughput::Elements(STEPS as u64));
    group.sample_size(20);

    fn run<R: Repr>(group: &mut BenchmarkGroup<WallTime>, spec: &GraphSpec, node_count: usize) {
        let ns = nodes::<R>(spec);
        group.bench_with_input(BenchmarkId::new(R::NAME, node_count), &ns, |b, ns| {
            b.iter(|| walk(black_box(ns), black_box(&spec.edges), STEPS))
        });
    }
    for node_count in [1 << 17, 1 << 21] {
        let spec = graph_spec(node_count, 8);
        run::<Range<usize>>(&mut group, &spec, node_count);
        run::<Range<u32>>(&mut group, &spec, node_count);
        run::<HandRolled>(&mut group, &spec, node_count);
        run::<SmallRange<u64, 32>>(&mut group, &spec, node_count);
        run::<SmallRange<u32, 8>>(&mut group, &spec, node_count);
    }
    group.finish();
}

// ---------------------------------------------------------------------------
// Workload: three-state memo table, [Option<R>; DEPTH] per entry
// ---------------------------------------------------------------------------

fn bench_memo_lookup(c: &mut Criterion) {
    const COUNT: usize = 1_000_000;
    let specs = memo_specs(COUNT);
    let mut group = c.benchmark_group("memo_lookup");
    group.throughput(Throughput::Elements((COUNT * DEPTH) as u64));

    fn run<R: Repr>(
        group: &mut BenchmarkGroup<WallTime>,
        specs: &[[Option<(usize, usize)>; DEPTH]],
    ) {
        let ms = memos::<R>(specs);
        group.bench_function(R::NAME, |b| b.iter(|| memo_lookup(black_box(&ms))));
    }
    run::<Range<usize>>(&mut group, &specs);
    run::<Range<u32>>(&mut group, &specs);
    run::<HandRolled>(&mut group, &specs);
    run::<SmallRange<u64, 32>>(&mut group, &specs);
    run::<SmallRange<u32, 8>>(&mut group, &specs);
    group.finish();
}

// ---------------------------------------------------------------------------
// Workload: random access into a large table of optional ranges
// ---------------------------------------------------------------------------

fn bench_random_access(c: &mut Criterion) {
    const COUNT: usize = 10_000_000;
    const LOOKUPS: usize = 2_000_000;
    let specs = range_specs(COUNT, 1 << 20, 64);
    let mut group = c.benchmark_group("random_access");
    group.throughput(Throughput::Elements(LOOKUPS as u64));
    group.sample_size(30);

    fn run<R: Repr>(group: &mut BenchmarkGroup<WallTime>, specs: &[(usize, usize)]) {
        let table: Vec<Option<R>> = specs
            .iter()
            .enumerate()
            .map(|(i, &(s, e))| (i % 10 != 0).then(|| R::make(s, e)))
            .collect();
        group.bench_function(R::NAME, |b| {
            b.iter(|| random_access(black_box(&table), LOOKUPS))
        });
    }
    run::<Range<usize>>(&mut group, &specs);
    run::<Range<u32>>(&mut group, &specs);
    run::<HandRolled>(&mut group, &specs);
    run::<SmallRange<u64, 32>>(&mut group, &specs);
    run::<SmallRange<u32, 8>>(&mut group, &specs);
    group.finish();
}

// ---------------------------------------------------------------------------
// Workload: sort by (start, len)
// ---------------------------------------------------------------------------

fn bench_sort(c: &mut Criterion) {
    const COUNT: usize = 1_000_000;
    let specs = range_specs(COUNT, 1 << 20, 64);

    let mut group = c.benchmark_group("sort_by_key");
    group.throughput(Throughput::Elements(COUNT as u64));
    group.sample_size(20);

    fn by_key<R: Repr>(group: &mut BenchmarkGroup<WallTime>, specs: &[(usize, usize)]) {
        let rs = ranges::<R>(specs);
        group.bench_function(R::NAME, |b| {
            b.iter_batched(
                || rs.clone(),
                |mut v| {
                    v.sort_unstable_by_key(Repr::key);
                    v
                },
                BatchSize::LargeInput,
            )
        });
    }
    by_key::<Range<usize>>(&mut group, &specs);
    by_key::<Range<u32>>(&mut group, &specs);
    by_key::<HandRolled>(&mut group, &specs);
    by_key::<SmallRange<u64, 32>>(&mut group, &specs);
    by_key::<SmallRange<u32, 8>>(&mut group, &specs);
    group.finish();

    // Types with a derived `Ord` that already means (start, len).
    let mut group = c.benchmark_group("sort_native_ord");
    group.throughput(Throughput::Elements(COUNT as u64));
    group.sample_size(20);

    fn native<R: Repr + Ord>(group: &mut BenchmarkGroup<WallTime>, specs: &[(usize, usize)]) {
        let rs = ranges::<R>(specs);
        group.bench_function(R::NAME, |b| {
            b.iter_batched(
                || rs.clone(),
                |mut v| {
                    v.sort_unstable();
                    v
                },
                BatchSize::LargeInput,
            )
        });
    }
    native::<HandRolled>(&mut group, &specs);
    native::<SmallRange<u64, 32>>(&mut group, &specs);
    native::<SmallRange<u32, 8>>(&mut group, &specs);
    group.finish();
}

// ---------------------------------------------------------------------------

fn print_layout() {
    fn row<R: Repr>() {
        println!(
            "{:<22} Option {:>2} B   Node {:>2} B   [Option; 5] {:>3} B",
            R::NAME,
            size_of::<Option<R>>(),
            size_of::<Node<R>>(),
            size_of::<[Option<R>; DEPTH]>()
        );
    }
    println!();
    row::<Range<usize>>();
    row::<Range<u32>>();
    row::<HandRolled>();
    row::<SmallRange<u64, 32>>();
    row::<SmallRange<u32, 8>>();
    println!();
}

fn bench_layout(c: &mut Criterion) {
    print_layout();
    c.bench_function("workloads_layout_printed", |b| b.iter(|| black_box(1)));
}

criterion_group!(
    benches,
    bench_layout,
    bench_slice_sum,
    bench_iterate,
    bench_graph_walk,
    bench_memo_lookup,
    bench_random_access,
    bench_sort,
);
fn main() {
    benches();
    Criterion::default().configure_from_args().final_summary();
    if !common::charts::skip_charts() {
        print!(
            "{}",
            common::charts::report(
                common::charts::WORKLOAD_GROUPS,
                "workloads: speedup over Option<Range<usize>> (higher is better)"
            )
        );
    }
}
