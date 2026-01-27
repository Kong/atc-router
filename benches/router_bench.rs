use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use rand::prelude::*;
use regex::Regex;
use router_prefilter::{Matcher, MatcherVisitor, RouterPrefilter};
use std::fs;
use std::hint::black_box;
use std::sync::LazyLock;

/// Load GitHub API routes from the paths file
/// Returns a vector of (method, path) tuples
fn load_github_paths() -> Vec<(String, String)> {
    let content =
        fs::read_to_string("benches/github_paths.txt").expect("Failed to read github_paths.txt");

    content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let (method, path) = line
                .split_once(':')
                .expect("every line should have a colon");
            (method.to_string(), path.to_string())
        })
        .collect()
}

fn path_to_regex(path: &str) -> String {
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\{[^}]*}").unwrap());
    // inaccurate, most segments don't allow `/`
    format!("^{}$", RE.replace_all(path, r"(.*)"))
}

/// Generate paths with version prefixes and optional non-path entries
///
/// Takes the base GitHub paths and adds `/v{i}/` prefixes to reach at least
/// `expected_paths` total paths. Then converts `non_path_fraction` of the
/// paths to `None` to simulate matches that don't depend on the path at all.
fn generate_path_set(
    mut rng: impl Rng,
    expected_paths: usize,
    non_path_fraction: f64,
) -> Vec<PathMatch> {
    let paths = load_github_paths();
    let base_paths: Vec<String> = paths.into_iter().map(|(_, path)| path).collect();

    let base_count = base_paths.len();
    let versions_needed = expected_paths.div_ceil(base_count);

    let mut paths: Vec<PathMatch> = Vec::new();
    for version in 0..versions_needed {
        for path in &base_paths {
            let full_path = format!("/v{}{}", version, path);
            paths.push(PathMatch(Some(path_to_regex(&full_path))));
            if paths.len() >= expected_paths {
                break;
            }
        }
    }

    // Convert to Option<String> and turn some into None
    let non_path_count = (paths.len() as f64 * non_path_fraction).round() as usize;

    // Turn the first `non_path_count` entries into None
    for path in paths[..non_path_count].iter_mut() {
        path.0 = None;
    }

    paths.shuffle(&mut rng);

    paths
}

#[derive(Debug, Clone)]
struct PathMatch(Option<String>);

impl Matcher for PathMatch {
    fn visit(&self, visitor: &mut MatcherVisitor) {
        if let Some(path_regex) = &self.0 {
            visitor.visit_match_regex(path_regex);
        }
    }
}

fn benchmarks(c: &mut Criterion) {
    let mut group = c.benchmark_group("build prefilter");
    let frequency = [
        ("all paths", 0.0),
        ("mostly paths", 0.02),
        ("half paths", 0.5),
        ("rarely paths", 0.99),
        ("no paths", 1.0),
    ];
    for size in [1, 100, 1_000, 10_000] {
        group.throughput(Throughput::Elements(size as u64));
        for (name, frequency) in frequency {
            let paths = generate_path_set(StdRng::seed_from_u64(1234), size, frequency);
            group.bench_with_input(BenchmarkId::new(name, size), &paths[..], |b, paths| {
                b.iter(|| RouterPrefilter::with_max_unfiltered(paths, 3 * size / 4))
            });
        }
    }
    group.finish();
    let mut group = c.benchmark_group("run prefilter");
    for size in [1, 100, 1_000, 10_000] {
        for (name, frequency) in frequency {
            let paths = generate_path_set(StdRng::seed_from_u64(1234), size, frequency);
            if let Some(prefilter) = RouterPrefilter::new(paths) {
                group.bench_with_input(BenchmarkId::new(name, size), &prefilter, |b, prefilter| {
                    b.iter(|| {
                        black_box(
                            prefilter
                                .possible_matches("/v1/orgs/MyOrg/attestations/MyAttestation")
                                .count(),
                        );
                    })
                });
            }
        }
    }
    group.finish();
    let mut group = c.benchmark_group("overlapping matches");
    let paths = vec![PathMatch(Some("^/all/overlapping".to_string())); 10_000];
    let prefilter = RouterPrefilter::new(&paths).unwrap();
    group.throughput(Throughput::Elements(paths.len() as u64));
    group.bench_with_input(
        BenchmarkId::from_parameter(paths.len()),
        &prefilter,
        |b, prefilter| {
            b.iter(|| {
                prefilter
                    .possible_matches("/all/overlapping/mypath")
                    .sum::<usize>()
            })
        },
    );
    group.finish();
}

criterion_group!(benches, benchmarks);
criterion_main!(benches);
