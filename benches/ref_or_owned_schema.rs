use atc_router::ast::{Type as AtcType, Value as AtcValue};
use atc_router::{context::Context, router::Router, schema::Schema};
use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use uuid::Uuid;

fn setup_owned() -> Router<'static> {
    let fields = vec![
        ("user.name".to_string(), AtcType::String),
        ("user.age".to_string(), AtcType::Int),
        ("user.email".to_string(), AtcType::String),
        ("user.id".to_string(), AtcType::Int),
    ];

    let mut routes = vec![
        (Uuid::now_v7(), "user.name == \"john\"".to_string()),
        (
            Uuid::now_v7(),
            "user.email == \"johndoe@user.me\"".to_string(),
        ),
    ];

    for i in 0..1000 {
        routes.push((Uuid::now_v7(), format!("user.id > {}", i)));
    }

    let mut schema = Schema::default();
    for (field, typ) in fields {
        schema.add_field(&field, typ);
    }

    Router::new_owning(schema)
}

fn run_random_match(router: &mut Router<'static>) {
    let field_values = vec![
        (
            "user.name".to_string(),
            AtcValue::String("john".to_string()),
        ),
        (
            "user.email".to_string(),
            AtcValue::String("johndoe@user.me".to_string()),
        ),
        ("user.age".to_string(), AtcValue::Int(42)),
        ("user.id".to_string(), AtcValue::Int(123)),
    ];

    let schema_ref = router.schema();
    let mut context = Context::new(schema_ref);
    for (field, value) in field_values {
        context.add_value(&field, value.clone());
    }

    let _ = router.try_match(&context);
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("matching_engine_random_fields", |b| {
        b.iter_batched_ref(
            setup_owned,
            |router| {
                run_random_match(router);
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
