use atc_router::ast::Type;
use atc_router::ast::Value;
use atc_router::context::Context;
use atc_router::router::Router;
use atc_router::schema::Schema;
use criterion::{criterion_group, criterion_main, Criterion};
use uuid::Uuid;

fn run() -> bool {
    let mut schema = Schema::default();
    schema.add_field("http.path", Type::String);
    schema.add_field("tcp.port", Type::Int);

    let mut router = Router::new(&schema);
    let uuid = Uuid::parse_str("a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c").unwrap();
    router.add_matcher(0, uuid, "http.path ^= \"/foo\" && tcp.port == 80");

    let mut context = Context::new(&schema);
    context.add_value("http.path", Value::String("/foo/bar".to_string()));
    context.add_value("tcp.port", Value::Int(80));

    router.execute(&mut context)
}

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("simple", |b| b.iter(|| run()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
