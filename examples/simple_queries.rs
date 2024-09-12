use atc_router::ast::Type;
use atc_router::ast::Value;
use atc_router::context::Context;
use atc_router::router::Router;
use atc_router::schema::Schema;
use uuid::Uuid;

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn main() {
    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    let mut schema = Schema::default();
    schema.add_field("http.queries.param0001", Type::String);
    schema.add_field("tcp.port", Type::Int);

    let mut router = Router::new(&schema);
    let uuid = Uuid::parse_str("a921a9aa-ec0e-4cf3-a6cc-1aa5583d150c").unwrap();
    router
        .add_matcher(
            0,
            uuid,
            "http.queries.param0001 == \"secret-message\" && tcp.port == 80",
        )
        .unwrap();

    let mut context = Context::new(&schema);
    context.add_value(
        "http.queries.param0001",
        Value::String("secret-message".to_string()),
    );
    context.add_value("tcp.port", Value::Int(80));

    let matched = router.execute(&mut context);
    println!("Matched? {}", matched);

    match context.result {
        Some(m) => println!(
            "Matched! uuid: {} --- prefix: {:?}",
            m.uuid,
            m.matches.get("http.queries.param0001").unwrap()
        ),
        None => println!("Nothin..."),
    };
}
