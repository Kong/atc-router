use atc_router::ast::{Type, Value};
use atc_router::context::Context;
use atc_router::router::Router;
use atc_router::schema::Schema;
use criterion::{criterion_group, criterion_main, Criterion};
use uuid::Uuid;

// To run this benchmark, execute the following command:
// ```shell
// cargo bench --bench build
// ```

const N: usize = 1000;

fn criterion_benchmark(c: &mut Criterion) {
    let mut schema = Schema::default();
    schema.add_field("a", Type::Int);

    let mut context = Context::new(&schema);
    context.add_value("a", Value::Int(N as i64));

    c.bench_function("Build Router", |b| {
        b.iter_with_large_drop(|| {
            let mut router = Router::new(&schema);
            for i in 0..N {
                let expr = format!("((a > 0 || a < {}) && a != 0) && a == 1", N + 1);
                router.add_matcher(N - i, Uuid::new_v4(), &expr).unwrap();
            }
            router
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
