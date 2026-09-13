//! Charts and tables built from criterion's on-disk estimates and rendered
//! with `malevich`. The criterion benches print these after their run, and
//! the `charts` bench target regenerates them into `BENCHMARKS.md`.

use super::{Asym, HandRolled, Node, Sym, ORDER, SHORT};
use malevich::{Bars, Frame, Plot, Scale};
use std::fs;
use std::mem::size_of;
use std::ops::Range;
use std::path::{Path, PathBuf};

/// Groups of the `range_comparison` bench.
pub const SCAN_GROUPS: &[&str] = &["sum_len", "sum_start", "contains", "creation", "large_scan"];
/// Groups of the `workloads` bench.
pub const WORKLOAD_GROUPS: &[&str] = &[
    "slice_sum",
    "iterate_indices",
    "graph_walk",
    "memo_lookup",
    "random_access",
    "sort_by_key",
    "sort_native_ord",
];

/// One criterion measurement: the median and its 95% interval, in ns.
#[derive(Clone, Debug)]
pub struct Estimate {
    pub group: String,
    pub function: String,
    pub param: Option<String>,
    pub elements: Option<f64>,
    pub median_ns: f64,
    pub lower_ns: f64,
    pub upper_ns: f64,
}

impl Estimate {
    /// Median per element when throughput is known, otherwise per iteration.
    fn per_unit(&self) -> f64 {
        self.median_ns / self.elements.unwrap_or(1.0)
    }
    fn param_key(&self) -> f64 {
        self.param
            .as_deref()
            .and_then(|p| p.parse().ok())
            .unwrap_or(0.0)
    }
}

/// `$CRITERION_HOME`, else `$CARGO_TARGET_DIR/criterion`, else `target/criterion`.
pub fn criterion_dir() -> PathBuf {
    if let Ok(home) = std::env::var("CRITERION_HOME") {
        return PathBuf::from(home);
    }
    let target = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".into());
    Path::new(&target).join("criterion")
}

/// Every `new/estimates.json` under the criterion directory.
pub fn load_estimates() -> Vec<Estimate> {
    let mut out = Vec::new();
    walk(&criterion_dir(), 0, &mut out);
    out.sort_by(|a, b| {
        (a.group.as_str(), rank(&a.function), a.param_key())
            .partial_cmp(&(b.group.as_str(), rank(&b.function), b.param_key()))
            .unwrap()
    });
    out
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<Estimate>) {
    if depth > 4 {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if path.file_name().is_some_and(|n| n == "new") {
            if let Some(e) = read_estimate(&path) {
                out.push(e);
            }
        } else {
            walk(&path, depth + 1, out);
        }
    }
}

fn read_estimate(new_dir: &Path) -> Option<Estimate> {
    let bench: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(new_dir.join("benchmark.json")).ok()?).ok()?;
    let est: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(new_dir.join("estimates.json")).ok()?).ok()?;
    let median = &est["median"];
    Some(Estimate {
        group: bench["group_id"].as_str()?.to_string(),
        function: bench["function_id"].as_str()?.to_string(),
        param: bench["value_str"].as_str().map(str::to_string),
        elements: bench["throughput"]["Elements"].as_f64(),
        median_ns: median["point_estimate"].as_f64()?,
        lower_ns: median["confidence_interval"]["lower_bound"].as_f64()?,
        upper_ns: median["confidence_interval"]["upper_bound"].as_f64()?,
    })
}

/// Position of a function id in `ORDER`; `Option<..>` wrappers are ignored.
fn rank(function: &str) -> usize {
    ORDER
        .iter()
        .position(|o| function.contains(o))
        .unwrap_or(ORDER.len())
}

fn short(function: &str) -> &'static str {
    SHORT.get(rank(function)).copied().unwrap_or("other")
}

fn functions<'a>(estimates: &[&'a Estimate]) -> Vec<&'a str> {
    let mut fs: Vec<&str> = Vec::new();
    for e in estimates {
        if !fs.contains(&e.function.as_str()) {
            fs.push(&e.function);
        }
    }
    fs.sort_by_key(|f| rank(f));
    fs
}

fn params(estimates: &[&Estimate]) -> Vec<Option<String>> {
    let mut ps: Vec<(f64, Option<String>)> = Vec::new();
    for e in estimates {
        if !ps.iter().any(|(_, p)| *p == e.param) {
            ps.push((e.param_key(), e.param.clone()));
        }
    }
    ps.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
    ps.into_iter().map(|(_, p)| p).collect()
}

fn find<'a>(
    estimates: &[&'a Estimate],
    function: &str,
    param: &Option<String>,
) -> Option<&'a Estimate> {
    estimates
        .iter()
        .copied()
        .find(|e| e.function == function && e.param == *param)
}

fn unit_label(estimates: &[&Estimate]) -> &'static str {
    if estimates.iter().all(|e| e.elements.is_some()) {
        "ns per element"
    } else {
        "ns per iteration"
    }
}

/// One bar series positioned on a band axis.
struct Series {
    x: Vec<f64>,
    value: Vec<f64>,
}

/// Bars of the median per element for one group.
/// Groups with several parameter values become grouped bars, one series per
/// function.
pub fn group_chart(all: &[Estimate], group: &str) -> Option<String> {
    let es: Vec<&Estimate> = all.iter().filter(|e| e.group == group).collect();
    if es.len() < 2 {
        return None;
    }
    let fns = functions(&es);
    let ps = params(&es);
    let title = format!("{group}: {} (lower is better)", unit_label(&es));

    if ps.len() <= 1 {
        let values: Vec<f64> = fns
            .iter()
            .map(|f| find(&es, f, &ps[0]).map_or(f64::NAN, Estimate::per_unit))
            .collect();
        let labels: Vec<&str> = fns.iter().map(|f| short(f)).collect();
        return Some(
            Plot::new()
                .layer(Bars::new(labels, &values[..]))
                .title(title)
                .y_label("ns")
                .render_best(&Frame::plain(96, 14)),
        );
    }

    let bands: Vec<String> = ps.iter().map(|p| p.clone().unwrap_or_default()).collect();
    let width = 0.8 / fns.len() as f64;
    let series: Vec<Series> = fns
        .iter()
        .enumerate()
        .map(|(k, f)| {
            let offset = (k as f64 - (fns.len() as f64 - 1.0) / 2.0) * width;
            Series {
                x: (0..ps.len()).map(|i| i as f64 + offset).collect(),
                value: ps
                    .iter()
                    .map(|p| find(&es, f, p).map_or(f64::NAN, Estimate::per_unit))
                    .collect(),
            }
        })
        .collect();
    let mut plot = Plot::new()
        .x_scale(Scale::bands(bands.iter().map(String::as_str)))
        .title(title)
        .y_label("ns");
    for (f, s) in fns.iter().zip(&series) {
        plot = plot.layer(Bars::at(&s.x[..], width * 0.9, &s.value[..]).label(*f));
    }
    Some(plot.render_best(&Frame::plain(100, 18)))
}

/// Speedup of every function over the `Range<usize>` baseline, one band per
/// group. Groups with several parameters use the largest one; groups without
/// the baseline are skipped.
pub fn speedup_chart(all: &[Estimate], groups: &[&str], title: &str) -> Option<String> {
    let mut bands: Vec<&str> = Vec::new();
    let mut rows: Vec<Vec<f64>> = Vec::new(); // per band, indexed by ORDER
    for group in groups {
        let es: Vec<&Estimate> = all.iter().filter(|e| e.group == *group).collect();
        let Some(param) = params(&es).into_iter().last() else {
            continue;
        };
        let fns = functions(&es);
        let Some(base) = fns
            .iter()
            .find(|f| rank(f) == 0)
            .and_then(|f| find(&es, f, &param))
        else {
            continue;
        };
        let mut row = vec![f64::NAN; ORDER.len()];
        for f in &fns {
            if let Some(e) = find(&es, f, &param) {
                row[rank(f)] = base.per_unit() / e.per_unit();
            }
        }
        bands.push(group);
        rows.push(row);
    }
    if bands.is_empty() {
        return None;
    }
    let width = 0.8 / ORDER.len() as f64;
    let series: Vec<(Vec<f64>, Vec<f64>)> = (0..ORDER.len())
        .map(|k| {
            let offset = (k as f64 - (ORDER.len() as f64 - 1.0) / 2.0) * width;
            let x = (0..bands.len()).map(|i| i as f64 + offset).collect();
            let v = rows.iter().map(|r| r[k]).collect();
            (x, v)
        })
        .collect();
    let mut plot = Plot::new()
        .x_scale(Scale::bands(bands.iter().copied()))
        .title(title)
        .y_label("x");
    for (name, (x, v)) in ORDER.iter().zip(&series) {
        plot = plot.layer(Bars::at(&x[..], width * 0.9, &v[..]).label(*name));
    }
    Some(plot.render_best(&Frame::plain(110, 22)))
}

/// Bytes per `Option<R>` and per `{ u32, Option<R> }` node.
pub fn sizes_chart() -> String {
    let option: Vec<f64> = vec![
        size_of::<Option<Range<usize>>>() as f64,
        size_of::<Option<Range<u32>>>() as f64,
        size_of::<Option<HandRolled>>() as f64,
        size_of::<Option<Sym>>() as f64,
        size_of::<Option<Asym>>() as f64,
    ];
    let node: Vec<f64> = vec![
        size_of::<Node<Range<usize>>>() as f64,
        size_of::<Node<Range<u32>>>() as f64,
        size_of::<Node<HandRolled>>() as f64,
        size_of::<Node<Sym>>() as f64,
        size_of::<Node<Asym>>() as f64,
    ];
    let left: Vec<f64> = (0..SHORT.len()).map(|i| i as f64 - 0.2).collect();
    let right: Vec<f64> = (0..SHORT.len()).map(|i| i as f64 + 0.2).collect();
    Plot::new()
        .x_scale(Scale::bands(SHORT))
        .layer(Bars::at(&left[..], 0.34, &option[..]).label("Option<R>"))
        .layer(Bars::at(&right[..], 0.34, &node[..]).label("{ u32, Option<R> } node"))
        .title("size in bytes")
        .y_label("bytes")
        .render_best(&Frame::plain(96, 16))
}

fn fmt_time(ns: f64) -> String {
    if ns >= 1e6 {
        format!("{:.2} ms", ns / 1e6)
    } else if ns >= 1e3 {
        format!("{:.1} µs", ns / 1e3)
    } else {
        format!("{ns:.1} ns")
    }
}

/// A markdown table of medians per iteration, one row per group and
/// parameter, one column per function in `ORDER`. Cells whose 95% interval
/// is wider than ±2% of the median carry a dagger.
pub fn table(all: &[Estimate], groups: &[&str]) -> String {
    let mut s = String::from("| benchmark |");
    for name in ORDER {
        s.push_str(&format!(" `{name}` |"));
    }
    s.push_str("\n|---|");
    s.push_str(&"---:|".repeat(ORDER.len()));
    s.push('\n');
    for group in groups {
        let es: Vec<&Estimate> = all.iter().filter(|e| e.group == *group).collect();
        for param in params(&es) {
            let label = match &param {
                Some(p) => format!("{group} / {p}"),
                None => group.to_string(),
            };
            s.push_str(&format!("| {label} |"));
            for name in ORDER {
                let cell = es
                    .iter()
                    .find(|e| e.function.contains(name) && e.param == param)
                    .map(|e| {
                        let half = (e.upper_ns - e.lower_ns) / 2.0;
                        let wide = half / e.median_ns > 0.02;
                        format!("{}{}", fmt_time(e.median_ns), if wide { "†" } else { "" })
                    })
                    .unwrap_or_default();
                s.push_str(&format!(" {cell} |"));
            }
            s.push('\n');
        }
    }
    s
}

/// Everything a bench prints after its run: one chart per group, the speedup
/// summary, and the table.
pub fn report(groups: &[&str], title: &str) -> String {
    let all = load_estimates();
    let mut out = String::new();
    for group in groups {
        if let Some(chart) = group_chart(&all, group) {
            out.push_str(&chart);
            out.push('\n');
        }
    }
    if let Some(chart) = speedup_chart(&all, groups, title) {
        out.push_str(&chart);
        out.push('\n');
    }
    out.push_str(&table(&all, groups));
    out
}

/// Criterion's `--test` and `--list` modes produce no estimates.
pub fn skip_charts() -> bool {
    std::env::args().any(|a| a == "--test" || a == "--list")
}

/// Replaces the body between `<!-- generated:NAME -->` and
/// `<!-- /generated -->` in `markdown`. Returns `None` if the markers are
/// missing.
pub fn splice(markdown: &str, name: &str, body: &str) -> Option<String> {
    let open = format!("<!-- generated:{name} -->");
    let close = "<!-- /generated -->";
    let start = markdown.find(&open)? + open.len();
    let end = start + markdown[start..].find(close)?;
    Some(format!(
        "{}\n{}\n{}",
        &markdown[..start],
        body.trim_end(),
        &markdown[end..]
    ))
}

/// Wraps a chart in a fenced text block for markdown.
pub fn fenced(chart: &str) -> String {
    format!("```text\n{}\n```", chart.trim_end())
}
