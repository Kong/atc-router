use atc_router::ast::{Type, Value};
use atc_router::context::Context;
use atc_router::router::Router;
use atc_router::schema::Schema;
use criterion::{criterion_group, criterion_main, Criterion};
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use uuid::Uuid;

// To run this benchmark, execute the following command:
// ```shell
// cargo bench --bench change-route
// ```

fn make_uuid(a: usize) -> String {
    format!("8cb2a7d0-c775-4ed9-989f-{:012}", a)
}

fn generate_random_string(len: usize) -> String {
    let bytes: Vec<u8> = thread_rng().sample_iter(&Alphanumeric).take(len).collect();
    String::from_utf8(bytes).unwrap()
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut schema = Schema::default();
    schema.add_field("http.path", Type::String);

    let mut router = Router::new(&schema);
    let mut context = Context::new(&schema);
    for i in 0..100000 {
        let uuid = Uuid::try_from(make_uuid(i).as_str()).unwrap();
        router
            .add_matcher(
                i + 1,
                uuid,
                format!(r##"http.path ^= r#"/route-{}"# "##, i).as_str(),
            )
            .unwrap();
    }
    let uuid = Uuid::try_from(make_uuid(0).as_str()).unwrap();

    c.bench_function("Route match", |b| {
        b.iter(|| {
            for _ in 0..10 {
                let random_id = generate_random_string(16);
                router.remove_matcher(0, uuid);
                router
                    .add_matcher(
                        0,
                        uuid,
                        format!(r##"http.path ^= r#"/test/{}"# "##, random_id).as_str(),
                    )
                    .unwrap();
                for _ in 0..1000 {
                  context.reset();
                  context.add_value("http.path", Value::String("/route-1".to_string()));
                  assert!(router.execute(&mut context));
                  assert!(context.value_of("http.path").is_some());
                }
            }
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
