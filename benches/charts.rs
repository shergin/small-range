//! Renders the charts and tables for every recorded criterion result and,
//! with `--write`, splices them into `BENCHMARKS.md` between
//! `<!-- generated:NAME -->` markers. Run the other benches first:
//!
//! ```text
//! cargo bench --bench range_comparison
//! cargo bench --bench workloads
//! cargo bench --bench charts -- --write
//! ```

mod common;

use common::charts::{self, SCAN_GROUPS, WORKLOAD_GROUPS};

fn main() {
    let write = std::env::args().any(|a| a == "--write");
    let all = charts::load_estimates();
    if all.is_empty() {
        eprintln!(
            "no criterion results under {}; run `cargo bench --bench workloads` first",
            charts::criterion_dir().display()
        );
        std::process::exit(1);
    }

    let mut blocks: Vec<(&str, String)> = vec![("sizes", charts::fenced(&charts::sizes_chart()))];
    blocks.push(("scans_table", charts::table(&all, SCAN_GROUPS)));
    if let Some(c) = charts::speedup_chart(
        &all,
        SCAN_GROUPS,
        "bulk scans: speedup over Option<Range<usize>> (higher is better)",
    ) {
        blocks.push(("scans_speedup", charts::fenced(&c)));
    }
    blocks.push(("workloads_table", charts::table(&all, WORKLOAD_GROUPS)));
    if let Some(c) = charts::speedup_chart(
        &all,
        WORKLOAD_GROUPS,
        "workloads: speedup over Option<Range<usize>> (higher is better)",
    ) {
        blocks.push(("workloads_speedup", charts::fenced(&c)));
    }
    for group in SCAN_GROUPS.iter().chain(WORKLOAD_GROUPS) {
        if let Some(c) = charts::group_chart(&all, group) {
            blocks.push((group, charts::fenced(&c)));
        }
    }

    for (name, body) in &blocks {
        println!("<!-- {name} -->\n{body}\n");
    }

    if write {
        let path = "BENCHMARKS.md";
        let mut doc = std::fs::read_to_string(path).expect("read BENCHMARKS.md");
        let (mut updated, mut missing) = (Vec::new(), Vec::new());
        for (name, body) in &blocks {
            match charts::splice(&doc, name, body) {
                Some(next) => {
                    doc = next;
                    updated.push(*name);
                }
                None => missing.push(*name),
            }
        }
        std::fs::write(path, doc).expect("write BENCHMARKS.md");
        eprintln!("updated {}: {}", path, updated.join(", "));
        if !missing.is_empty() {
            eprintln!(
                "printed only (no marker in {}): {}",
                path,
                missing.join(", ")
            );
        }
    }
}
