use criterion::{criterion_group, criterion_main, Criterion};
use atc_router::router::Router;
use atc_router::schema::Schema;
use atc_router::ast::{Type, Value};
use atc_router::context::Context;
use uuid::Uuid;

// To run this benchmark, execute the following command:
// ```shell
// cargo bench --bench test
// ```

const N : usize = 100000;

fn criterion_benchmark(c: &mut Criterion) {
    let mut schema = Schema::default();
    schema.add_field("a", Type::Int);

    let mut router = Router::new(&schema);

    for i in 0..N {
        let expr = format!("((a > 0 || a < {}) && a != 0) && a == 1", N + 1);
        router.add_matcher(N - i, Uuid::new_v4(), &expr).unwrap();
    }

    let mut context = Context::new(&schema);
    context.add_value("a", Value::Int(N as i64));

    c.bench_function("Doesn't Match", |b| {
        b.iter(|| {
            for _ in 0..10 {
                let is_match = router.execute(&mut context);
                assert!(!is_match);
            }
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);