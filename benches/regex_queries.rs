use std::time::Duration;

use atc_router::ast::Type;
use atc_router::ast::Value;
use atc_router::context::Context;
use atc_router::router::Router;
use atc_router::schema::Schema;
use criterion::{criterion_group, criterion_main, Criterion};
use uuid::Uuid;

struct QueryFieldWithUuid {
    uuid: Uuid,
    field: String,
}

fn generate_fields(field_count: i32) -> Vec<QueryFieldWithUuid> {
    (0..field_count)
        .map(|i| QueryFieldWithUuid {
            uuid: Uuid::new_v4(),
            field: format!("http.queries.param{:04}", i),
        })
        .collect()
}

fn run() {
    let regex = "r#\"^\\d+---secret-messagesecret-messagesecret-messagesecret-messagesecret-messagesecret-messagesecret-message$\"#";
    let fields = generate_fields(10000);

    let mut schema = Schema::default();
    fields
        .iter()
        .for_each(|field| schema.add_field(&field.field, Type::String));
    schema.add_field("tcp.port", Type::Int);

    let mut router: Router<'_> = Router::new(&schema);

    let _: Result<Vec<()>, String> = fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            router.add_matcher(i, field.uuid, &format!("{} ~ {}", field.field, regex))
        })
        .collect();

    let mut context = Context::new(&schema);
    context.add_value("http.queries.param0001", Value::String("12345---secret-messagesecret-messagesecret-messagesecret-messagesecret-messagesecret-messagesecret-messagex".to_string()));
    context.add_value("http.queries.param0002", Value::String("12345---secret-messagesecret-messagesecret-messagesecret-messagesecret-messagesecret-messagesecret-message".to_string()));
    context.add_value("http.queries.param0003", Value::String("12345---secret-messagesecret-messagesecret-messagesecret-messagesecret-messagesecret-messagesecret-messagex".to_string()));
    context.add_value("http.queries.param0004", Value::String("12345---secret-messagesecret-messagesecret-messagesecret-messagesecret-messagesecret-messagesecret-messagex".to_string()));
    context.add_value("http.queries.param0005", Value::String("12345---secret-messagesecret-messagesecret-messagesecret-messagesecret-messagesecret-messagesecret-messagex".to_string()));
    context.add_value("tcp.port", Value::Int(80));

    router.execute(&mut context);
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("regex_queries");
    group.bench_function("regex_queries", |b| b.iter(|| run()));
    group.finish();
}

criterion_group! {
    name = benches;
    // This can be any expression that returns a `Criterion` object.
    config = Criterion::default().measurement_time(Duration::from_secs(10));
    targets = bench
}
criterion_main!(benches);
