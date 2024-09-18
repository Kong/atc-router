use atc_router::ast::{Type, Value};
use atc_router::context::Context;
use atc_router::router::Router;
use atc_router::schema::Schema;
use criterion::{criterion_group, criterion_main, Criterion};
use uuid::Uuid;

// To run this benchmark, execute the following command:
// ```shell
// cargo bench --bench match_mix
// ```

const N: usize = 100000;

fn make_uuid(a: usize) -> String {
    format!("8cb2a7d0-c775-4ed9-989f-{:012}", a)
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut schema = Schema::default();
    schema.add_field("http.path", atc_router::ast::Type::String);
    schema.add_field("http.version", atc_router::ast::Type::String);
    schema.add_field("a", atc_router::ast::Type::Int);

    let mut router = Router::new(&schema);

    for i in 0..N {
        let expr = format!(
            r#"(http.path == "hello{}" && http.version == "1.1") || !(( a == 2) && ( a == 9 )) || !(a == 1) || ( a == 3 && a == 4) && !(a == 5)"#,
            i
        );
        let variant = make_uuid(i);
        let uuid = Uuid::try_from(variant.as_str()).unwrap();
        router.add_matcher(N - i, uuid, &expr).unwrap();
    }

    let mut ctx_match = Context::new(&schema);
    ctx_match.add_value(
        "http.path",
        atc_router::ast::Value::String("hello49999".to_string()),
    );
    ctx_match.add_value(
        "http.version",
        atc_router::ast::Value::String("1.1".to_string()),
    );
    ctx_match.add_value("a", Value::Int(3 as i64));

    c.bench_function("Match", |b| {
        b.iter(|| {
            let is_match = router.execute(&mut ctx_match);
            assert!(is_match);
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
