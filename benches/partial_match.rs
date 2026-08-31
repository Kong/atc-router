//! Cost paid by routes that match part of an expression and then fail.
//!
//! `match_mix` leads every expression with the discriminating predicate, so a
//! candidate route fails on its first predicate and `&&` short-circuits before
//! anything is collected. Real route sets are often the other way round: many
//! routes share a leading host or method predicate and differ only later, so
//! every candidate matches the shared part before failing.
//!
//! Collecting a matched value costs a `String` clone for the field name and a
//! `Value` clone for the expression side; a matching regex additionally runs
//! `captures()` and clones every capture group. This benchmark measures what a
//! route set pays for that when the collected values are discarded anyway.

use atc_router::ast::{Type, Value};
use atc_router::context::Context;
use atc_router::router::Router;
use atc_router::schema::Schema;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use uuid::Uuid;

const HOST: &str = "shop.example.com";

fn make_uuid(a: usize) -> Uuid {
    format!("8cb2a7d0-c775-4ed9-989f-{:012}", a)
        .parse()
        .unwrap()
}

fn schema() -> Schema {
    let mut schema = Schema::default();
    schema.add_field("http.host", Type::String);
    schema.add_field("http.method", Type::String);
    schema.add_field("http.path", Type::String);
    schema
}

/// Two shared predicates match on every candidate, then the path discriminates.
/// A losing candidate collects two matched values and throws them away.
fn shared_prefix_expr(i: usize) -> String {
    format!(
        r#"http.host == "{HOST}" && http.method == "GET" && http.path ^= "/api/v{i}/""#
    )
}

/// The regex matches on every candidate, so every losing candidate runs
/// `captures()` and clones the capture groups before the host predicate fails.
fn shared_regex_expr(i: usize) -> String {
    format!(
        r##"http.path ~ r#"^/api/(v[0-9]+)/(.+)$"# && http.host == "h{i}.example.com""##
    )
}

fn bench_shape(
    c: &mut Criterion,
    group: &str,
    expr: fn(usize) -> String,
    matching_host: bool,
) {
    let schema = schema();
    let mut g = c.benchmark_group(group);

    for n in [1_000usize, 10_000] {
        let mut router = Router::new(&schema);
        for i in 0..n {
            router.add_matcher(n - i, make_uuid(i), &expr(i)).unwrap();
        }

        // The route added last has the lowest priority, so it is visited last:
        // the request that matches it walks the whole set.
        let last = n - 1;

        for (label, path, host, expected) in [
            (
                "worst (last route wins)",
                format!("/api/v{last}/items"),
                if matching_host {
                    HOST.to_string()
                } else {
                    format!("h{last}.example.com")
                },
                true,
            ),
            (
                "miss (no route wins)",
                format!("/api/v{n}/items"),
                if matching_host {
                    HOST.to_string()
                } else {
                    format!("h{n}.example.com")
                },
                false,
            ),
        ] {
            let mut ctx = Context::new(&schema);
            ctx.add_value("http.host", Value::String(host));
            ctx.add_value("http.method", Value::String("GET".to_string()));
            ctx.add_value("http.path", Value::String(path));

            g.bench_with_input(BenchmarkId::new(label, n), &n, |b, _| {
                b.iter(|| {
                    let found = router.try_match(&ctx);
                    assert_eq!(found.is_some(), expected);
                });
            });
        }
    }

    g.finish();
}

fn shared_prefix(c: &mut Criterion) {
    bench_shape(c, "shared prefix", shared_prefix_expr, true);
}

fn shared_regex(c: &mut Criterion) {
    bench_shape(c, "shared regex", shared_regex_expr, false);
}

criterion_group!(benches, shared_prefix, shared_regex);
criterion_main!(benches);
