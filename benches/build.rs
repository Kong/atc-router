use atc_router::ast::Type;
use atc_router::router::Router;
use atc_router::schema::Schema;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use uuid::Uuid;

// To run this benchmark, execute the following command:
// ```shell
// cargo bench --bench build
// ```

const N: usize = 5000;

fn make_uuid(a: usize) -> String {
    format!("8cb2a7d0-c775-4ed9-989f-{:012}", a)
}

fn criterion_benchmark(c: &mut Criterion) {
    // prepare test data
    let mut data = Vec::new();
    for i in 0..N {
        let priority = N - i;

        let uuid = make_uuid(i);
        let uuid = Uuid::try_from(uuid.as_str()).unwrap();

        let expr = format!(
            "((a > 0 || a < {0}) && a != 0) && a == 1 && b == \"{0}\"",
            N + 1
        );

        data.push((priority, uuid, expr))
    }

    let mut schema = Schema::default();
    schema.add_field("a", Type::Int);
    schema.add_field("b", Type::String);

    let mut g = c.benchmark_group("build router");
    for n in [1, 10, 100, 500, 1000, 3000, N] {
        g.throughput(Throughput::Elements(n as u64));
        g.bench_with_input(
            BenchmarkId::new("without prefilter", n),
            &data[..n],
            |b, data| {
                b.iter_with_large_drop(|| {
                    let mut router = Router::new(&schema);
                    for v in data {
                        router.add_matcher(v.0, v.1, &v.2).unwrap();
                    }
                    router
                });
            },
        );

        g.bench_with_input(
            BenchmarkId::new("with prefilter", n),
            &data[..n],
            |b, data| {
                b.iter_with_large_drop(|| {
                    let mut router = Router::new(&schema);
                    router.enable_prefilter("b");
                    for v in data {
                        router.add_matcher(v.0, v.1, &v.2).unwrap();
                    }
                    router
                });
            },
        );
    }
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
