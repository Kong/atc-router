use atc_router::ast::{Type, Value};
use atc_router::context::Context;
use atc_router::router::Router;
use atc_router::schema::Schema;
use criterion::{criterion_group, criterion_main, Criterion};
use uuid::Uuid;

// To run this benchmark, execute the following command:
// ```shell
// cargo bench --bench not_match_mix
// ```

const N: usize = 100_000;

fn make_uuid(a: usize) -> String {
    format!("8cb2a7d0-c775-4ed9-989f-{:012}", a)
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut schema = Schema::default();
    schema.add_field("http.path", Type::String);
    schema.add_field("http.version", Type::String);
    schema.add_field("a", Type::Int);

    let mut router = Router::new(&schema);

    for i in 0..N {
        let expr = format!(
            r#"(http.path == "hello{}" && http.version == "1.1") || {} || {} || {}"#,
            i,
            "!((a == 2) && (a == 9))",
            "!(a == 1)",
            "(a == 3 && a == 4) && !(a == 5)"
        );

        let uuid = make_uuid(i);
        let uuid = Uuid::try_from(uuid.as_str()).unwrap();

        router.add_matcher(N - i, uuid, &expr).unwrap();
    }

    let mut ctx = Context::new(&schema);
    ctx.add_value(
        "http.path",
        Value::String("hello49999".to_string()),
    );
    ctx.add_value(
        "http.version",
        Value::String("1.1".to_string()),
    );
    ctx.add_value("a", Value::Int(5_i64));  // not match

    c.bench_function("Doesn't Match", |b| {
        b.iter(|| {
            let not_match = !router.execute(&mut ctx);
            assert!(not_match);
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
