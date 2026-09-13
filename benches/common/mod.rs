//! Shared representations, deterministic data and workload kernels for the
//! `workloads` (criterion) and `showcase` (malevich) benchmarks.

#![allow(dead_code)]

pub mod charts;

use small_range::SmallRange;
use std::num::NonZeroU32;
use std::ops::Range;

/// 8 bytes, 32 bits of start and 32 bits of length.
pub type Sym = SmallRange<u64, 32>;
/// 4 bytes, 24 bits of start and 8 bits of length.
pub type Asym = SmallRange<u32, 8>;

/// Canonical order of the representations in tables and charts.
pub const ORDER: [&str; 5] = [
    "Range<usize>",
    "Range<u32>",
    "{u32, NonZeroU32}",
    "SmallRange<u64, 32>",
    "SmallRange<u32, 8>",
];

/// Labels short enough for a band axis, in `ORDER`.
pub const SHORT: [&str; 5] = [
    "Range<usize>",
    "Range<u32>",
    "hand-rolled",
    "SR<u64, 32>",
    "SR<u32, 8>",
];

// ---------------------------------------------------------------------------
// Representations under test
// ---------------------------------------------------------------------------

pub trait Repr: Clone + 'static {
    const NAME: &'static str;
    fn make(start: usize, end: usize) -> Self;
    fn start(&self) -> usize;
    fn len(&self) -> usize;
    fn indices(&self) -> Range<usize>;
    fn slice<'a, T>(&self, data: &'a [T]) -> &'a [T];
    #[inline]
    fn key(&self) -> (usize, usize) {
        (self.start(), self.len())
    }
    /// Sorts by `(start, len)` the way this type naturally can: a key
    /// extraction for `Range`, the derived `Ord` for the packed types.
    fn sort(v: &mut [Self]) {
        v.sort_unstable_by_key(Self::key);
    }
}

impl Repr for Range<usize> {
    const NAME: &'static str = "Range<usize>";
    #[inline]
    fn make(start: usize, end: usize) -> Self {
        start..end
    }
    #[inline]
    fn start(&self) -> usize {
        self.start
    }
    #[inline]
    fn len(&self) -> usize {
        self.end - self.start
    }
    #[inline]
    fn indices(&self) -> Range<usize> {
        self.clone()
    }
    #[inline]
    fn slice<'a, T>(&self, data: &'a [T]) -> &'a [T] {
        &data[self.clone()]
    }
}

impl Repr for Range<u32> {
    const NAME: &'static str = "Range<u32>";
    #[inline]
    fn make(start: usize, end: usize) -> Self {
        start as u32..end as u32
    }
    #[inline]
    fn start(&self) -> usize {
        self.start as usize
    }
    #[inline]
    fn len(&self) -> usize {
        (self.end - self.start) as usize
    }
    #[inline]
    fn indices(&self) -> Range<usize> {
        self.start as usize..self.end as usize
    }
    #[inline]
    fn slice<'a, T>(&self, data: &'a [T]) -> &'a [T] {
        &data[self.start as usize..self.end as usize]
    }
}

/// The struct one would write by hand: same size and niche as
/// `SmallRange<u64, 32>`, no bit packing.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct HandRolled {
    pub start: u32,
    pub len_plus_one: NonZeroU32,
}

impl Repr for HandRolled {
    const NAME: &'static str = "{u32, NonZeroU32}";
    #[inline]
    fn make(start: usize, end: usize) -> Self {
        Self {
            start: start as u32,
            len_plus_one: NonZeroU32::new((end - start) as u32 + 1).unwrap(),
        }
    }
    #[inline]
    fn start(&self) -> usize {
        self.start as usize
    }
    #[inline]
    fn len(&self) -> usize {
        (self.len_plus_one.get() - 1) as usize
    }
    #[inline]
    fn indices(&self) -> Range<usize> {
        self.start()..self.start() + self.len()
    }
    #[inline]
    fn slice<'a, T>(&self, data: &'a [T]) -> &'a [T] {
        &data[self.indices()]
    }
    fn sort(v: &mut [Self]) {
        v.sort_unstable();
    }
}

macro_rules! impl_small_range {
    ($S:ty, $N:literal, $name:literal) => {
        impl Repr for SmallRange<$S, $N> {
            const NAME: &'static str = $name;
            #[inline]
            fn make(start: usize, end: usize) -> Self {
                Self::new(start, end)
            }
            #[inline]
            fn start(&self) -> usize {
                SmallRange::start(*self)
            }
            #[inline]
            fn len(&self) -> usize {
                SmallRange::len(*self)
            }
            #[inline]
            fn indices(&self) -> Range<usize> {
                self.to_range()
            }
            #[inline]
            fn slice<'a, T>(&self, data: &'a [T]) -> &'a [T] {
                &data[*self]
            }
            fn sort(v: &mut [Self]) {
                v.sort_unstable();
            }
        }
    };
}
impl_small_range!(u64, 32, "SmallRange<u64, 32>");
impl_small_range!(u32, 8, "SmallRange<u32, 8>");

// ---------------------------------------------------------------------------
// Deterministic data
// ---------------------------------------------------------------------------

pub struct Lcg(u64);

impl Lcg {
    pub fn new(seed: u64) -> Self {
        Self(seed)
    }
    #[inline]
    pub fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    #[inline]
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// `count` ranges into a buffer of `buffer_len`, lengths below `max_len`.
pub fn range_specs(count: usize, buffer_len: usize, max_len: usize) -> Vec<(usize, usize)> {
    let mut rng = Lcg::new(7);
    (0..count)
        .map(|_| {
            let len = rng.below(max_len);
            let start = rng.below(buffer_len - max_len);
            (start, start + len)
        })
        .collect()
}

pub fn ranges<R: Repr>(specs: &[(usize, usize)]) -> Vec<R> {
    specs.iter().map(|&(s, e)| R::make(s, e)).collect()
}

// ---------------------------------------------------------------------------
// Workload: slice a buffer through the range and consume the slice
// ---------------------------------------------------------------------------

pub fn slice_sum<R: Repr>(ranges: &[R], data: &[u8]) -> u64 {
    ranges
        .iter()
        .map(|r| r.slice(data).iter().map(|&b| b as u64).sum::<u64>())
        .fold(0u64, |a, b| a.wrapping_add(b))
}

pub fn iterate_indices<R: Repr>(ranges: &[R]) -> u64 {
    let mut acc = 0u64;
    for r in ranges {
        for i in r.indices() {
            acc = acc.wrapping_add(i as u64);
        }
    }
    acc
}

#[derive(Clone)]
pub struct Node<R> {
    pub payload: u32,
    pub next: Option<R>,
}

pub struct GraphSpec {
    pub payloads: Vec<u32>,
    /// `None` for terminal nodes, otherwise the edge range.
    pub next: Vec<Option<(usize, usize)>>,
    pub edges: Vec<u32>,
}

pub fn graph_spec(node_count: usize, max_degree: usize) -> GraphSpec {
    let mut rng = Lcg::new(11);
    let mut edges = Vec::with_capacity(node_count * max_degree / 2);
    let mut next = Vec::with_capacity(node_count);
    let mut payloads = Vec::with_capacity(node_count);
    for _ in 0..node_count {
        payloads.push(rng.next() as u32);
        let degree = rng.below(max_degree + 1);
        if degree == 0 {
            next.push(None);
        } else {
            let start = edges.len();
            for _ in 0..degree {
                edges.push(rng.below(node_count) as u32);
            }
            next.push(Some((start, edges.len())));
        }
    }
    GraphSpec {
        payloads,
        next,
        edges,
    }
}

pub fn nodes<R: Repr>(spec: &GraphSpec) -> Vec<Node<R>> {
    spec.payloads
        .iter()
        .zip(&spec.next)
        .map(|(&payload, next)| Node {
            payload,
            next: next.map(|(s, e)| R::make(s, e)),
        })
        .collect()
}

/// Visit `steps` nodes in a pseudo-random order; at each, read every
/// neighbour's payload through the node's range.
pub fn walk<R: Repr>(nodes: &[Node<R>], edges: &[u32], steps: usize) -> u64 {
    let mut rng = Lcg::new(3);
    let mut sum = 0u64;
    for _ in 0..steps {
        let node = &nodes[rng.below(nodes.len())];
        if let Some(r) = &node.next {
            for e in r.indices() {
                sum = sum.wrapping_add(nodes[edges[e] as usize].payload as u64);
            }
        }
    }
    sum
}

pub const DEPTH: usize = 5;

pub fn memo_specs(count: usize) -> Vec<[Option<(usize, usize)>; DEPTH]> {
    let mut rng = Lcg::new(5);
    (0..count)
        .map(|_| {
            std::array::from_fn(|_| match rng.below(10) {
                0..=3 => None,
                4..=5 => Some((0, 0)),
                _ => {
                    let start = rng.below(1 << 20);
                    Some((start, start + rng.below(64)))
                }
            })
        })
        .collect()
}

pub fn memos<R: Repr>(specs: &[[Option<(usize, usize)>; DEPTH]]) -> Vec<[Option<R>; DEPTH]> {
    specs
        .iter()
        .map(|row| std::array::from_fn(|d| row[d].map(|(s, e)| R::make(s, e))))
        .collect()
}

/// Classify every slot: not computed, computed and empty, or populated.
pub fn memo_lookup<R: Repr>(memos: &[[Option<R>; DEPTH]]) -> (u64, u64, u64) {
    let (mut misses, mut dead, mut live) = (0u64, 0u64, 0u64);
    for row in memos {
        for slot in row {
            match slot {
                None => misses += 1,
                Some(r) if r.len() == 0 => dead += 1,
                Some(r) => live = live.wrapping_add(r.len() as u64),
            }
        }
    }
    (misses, dead, live)
}

pub fn random_access<R: Repr>(table: &[Option<R>], lookups: usize) -> u64 {
    let mut rng = Lcg::new(9);
    let mut sum = 0u64;
    for _ in 0..lookups {
        if let Some(r) = &table[rng.below(table.len())] {
            sum = sum.wrapping_add((r.start() + r.len()) as u64);
        }
    }
    sum
}
