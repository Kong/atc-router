use atc_router::ast::{Type as AtcType, Value as AtcValue};
use atc_router::{context::Context, router::Router, schema::Schema};
use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion};
use uuid::Uuid;

fn setup_schema() -> Schema {
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
    schema
}

fn setup_owned() -> Router<'static> {
    let schema = setup_schema();
    Router::new_owning(schema)
}

fn setup_boxed() -> Router<'static> {
    let schema = setup_schema();
    Router::new_boxing(schema)
}

fn run_match(router: &mut Router<'static>) {
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
        ("user.id".to_string(), AtcValue::Int(567)),
    ];

    let schema_ref = router.schema();
    let mut context = Context::new(schema_ref);
    for (field, value) in field_values {
        context.add_value(&field, value.clone());
    }

    let _ = router.try_match(&context);
}

fn benchmark_multiple_setups(c: &mut Criterion) {
    let mut group = c.benchmark_group("matching_engine_setups");

    group.bench_function(BenchmarkId::new("owned", "run"), |b| {
        b.iter_batched_ref(setup_owned, run_match, BatchSize::SmallInput);
    });

    group.bench_function(BenchmarkId::new("boxed", "run"), |b| {
        b.iter_batched_ref(setup_boxed, run_match, BatchSize::SmallInput);
    });

    group.finish();
}

criterion_group!(benches, benchmark_multiple_setups);
criterion_main!(benches);
