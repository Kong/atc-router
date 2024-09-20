use atc_router::ast::{Type, Value};
use atc_router::context::Context;
use atc_router::router::Router;
use atc_router::schema::Schema;
use criterion::{criterion_group, criterion_main, Criterion};
use uuid::Uuid;

// To run this benchmark, execute the following command:
// ```shell
// cargo bench --bench string
// ```

const N: usize = 100000;

fn make_uuid(a: usize) -> String {
    format!("8cb2a7d0-c775-4ed9-989f-{:012}", a)
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut schema = Schema::default();
    schema.add_field("http.path.segments.*", Type::String);
    schema.add_field("http.path.segments.len", Type::Int);

    let mut router = Router::new(&schema);

    for i in 0..N {
        let expr = format!(
            r#"http.path.segments.0_1 == "test/run" && http.path.segments.3 == "address{}" && http.path.segments.len == 3"#,
            i
        );
        let variant = make_uuid(i);
        let uuid = Uuid::try_from(variant.as_str()).unwrap();
        router.add_matcher(N - i, uuid, &expr).unwrap();
    }

    let mut context = Context::new(&schema);
    context.add_value("http.path.segments.0_1", "test/run".to_string().into());
    context.add_value("http.path.segments.3", "bar".to_string().into());
    context.add_value("http.path.segments.len", Value::Int(3 as i64));

    c.bench_function("Doesn't Match", |b| {
        b.iter(|| {
            let is_match = router.execute(&mut context);
            assert!(!is_match);
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
