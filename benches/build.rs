use atc_router::ast::Type;
use atc_router::router::Router;
use atc_router::schema::Schema;
use criterion::{criterion_group, criterion_main, Criterion};
use uuid::Uuid;

// To run this benchmark, execute the following command:
// ```shell
// cargo bench --bench build
// ```

const N: usize = 1000;

fn make_uuid(a: usize) -> String {
    format!("8cb2a7d0-c775-4ed9-989f-{:012}", a)
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut schema = Schema::default();
    schema.add_field("a", Type::Int);

    // prepare test data
    let mut data = Vec::new();
    for i in 0..N {
        let expr = format!("((a > 0 || a < {}) && a != 0) && a == 1", N + 1);
        let variant = make_uuid(i);
        let uuid = Uuid::try_from(variant.as_str()).unwrap();

        data.push((uuid, expr))
    }

    c.bench_function("Build Router", |b| {
        b.iter_with_large_drop(|| {
            let mut router = Router::new(&schema);
            for i in 0..N {
                let v = &data[i];
                router.add_matcher(N - i, v.0, &v.1).unwrap();
            }
            router
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
