use atc_router::ast::{Type, Value};
use atc_router::context::Context;
use atc_router::router::Router;
use atc_router::schema::Schema;
use criterion::{criterion_group, criterion_main, Criterion};
use uuid::Uuid;

// To run this benchmark, execute the following command:
// ```shell
// cargo bench --bench match_int
// ```

const N: usize = 100000;

fn make_uuid(a: usize) -> String {
    format!("8cb2a7d0-c775-4ed9-989f-{:012}", a)
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut schema = Schema::default();
    schema.add_field("a", Type::Int);

    let mut router = Router::new(&schema);

    let expr = format!("((a < 1 || a == {}) && a != 0) && a != 1", N);
    let variant = make_uuid(2024);
    let uuid = Uuid::try_from(variant.as_str()).unwrap();
    router.add_matcher(1, uuid, &expr).unwrap();

    let mut context = Context::new(&schema);
    context.add_value("a", Value::Int(N as i64));
    c.bench_function("Doesn't Match", |b| {
        b.iter(|| {
            for _i in 0..N {
                let is_match = router.execute(&mut context);
                assert!(is_match);
            }
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
