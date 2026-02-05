use atc_router::ast::{Type, Value};
use atc_router::context::Context;
use atc_router::router::Router;
use atc_router::schema::Schema;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use uuid::Uuid;

fn make_uuid(a: usize) -> String {
    format!("8cb2a7d0-c775-4ed9-989f-{:012}", a)
}

fn worst_case(c: &mut Criterion) {
    const MAX: usize = 2_000;
    let mut schema = Schema::default();
    schema.add_field("http.path", Type::String);

    // This is the worst case for the prefilter, we end up finding prefixes as
    // '/a', '/a/a', '/a/a/a', etc, which means a string that starts with `/a/a/a/...` also
    // starts with `/a/a/...`, and also starts with `/a/...` etc.
    let all_matchers: Vec<(usize, Uuid, String)> = (0..MAX)
        .map(|i| {
            let expression_str = format!("http.path == r#\"{}\"#", "/a".repeat(i));
            (MAX - i, make_uuid(i).parse().unwrap(), expression_str)
        })
        .collect();

    let mut g = c.benchmark_group("worst-case build");
    for n in [1, 10, 100, 500, 1000, MAX] {
        // Get the _last_ matchers, to avoid both growing the number of matchers, and the size of
        // the matchers
        let matchers = all_matchers.windows(n).last().unwrap();
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(
            BenchmarkId::new("without prefilter", n),
            matchers,
            |b, matchers| {
                b.iter_with_large_drop(|| {
                    let mut router = Router::new(&schema);
                    for &(priority, uuid, ref expression) in matchers {
                        router.add_matcher(priority, uuid, expression).unwrap();
                    }
                    router
                });
            },
        );

        g.bench_with_input(
            BenchmarkId::new("with prefilter", n),
            matchers,
            |b, matchers| {
                b.iter_with_large_drop(|| {
                    let mut router = Router::new(&schema);
                    router.enable_prefilter("http.path");
                    for &(priority, uuid, ref expression) in matchers {
                        router.add_matcher(priority, uuid, expression).unwrap();
                    }
                    router
                });
            },
        );
    }
    g.finish();

    let mut router = Router::new(&schema);
    for (priority, uuid, expression) in all_matchers {
        router.add_matcher(priority, uuid, &expression).unwrap();
    }
    let mut ctx = Context::new(&schema);
    let mut g = c.benchmark_group("worst-case match");
    for n in [1, 10, 100, 500, 1000, MAX] {
        ctx.reset();
        ctx.add_value("http.path", Value::String("/a".repeat(n - 1)));
        g.bench_with_input(
            BenchmarkId::new("without prefilter", n),
            &router,
            |b, router| {
                b.iter(|| {
                    let found = router.execute(&mut ctx);
                    assert!(found);
                    found
                })
            },
        );
    }
    router.enable_prefilter("http.path");
    for n in [1, 10, 100, 500, 1000, MAX] {
        ctx.reset();
        ctx.add_value("http.path", Value::String("/a".repeat(n - 1)));
        g.bench_with_input(
            BenchmarkId::new("with prefilter", n),
            &router,
            |b, router| {
                b.iter(|| {
                    let found = router.execute(&mut ctx);
                    assert!(found);
                    found
                })
            },
        );
    }
}

criterion_group!(benches, worst_case);
criterion_main!(benches);
