use atc_router::{
    ast::Type,
    ast::Value,
    context::Context,
    router::Router,
    schema::{self, Schema},
};
use criterion::{criterion_group, criterion_main, Criterion};
use serde::{Deserialize, Serialize};
use serde_json;
use std::env;
use std::fs;
use std::net::{IpAddr, Ipv4Addr};
use std::{hint::black_box, str::FromStr};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
struct TestData {
    rules: Vec<serde_json::Value>,
    match_keys: Vec<serde_json::Value>,
    match_values: Vec<serde_json::Value>,
    not_match_values: Vec<serde_json::Value>,
}

// prepare match rules, context keys, context values from data.json file
fn prepare_data() -> TestData {
    let cwd = env::current_dir().unwrap();
    let file_str =
        fs::read_to_string(cwd.join("benches/data.json")).expect("unable to open data.json");
    serde_json::from_str(&file_str).unwrap()
}

// setup Schema
fn setup_schema() -> schema::Schema {
    let mut s = Schema::default();
    s.add_field("net.protocol", Type::String);
    s.add_field("tls.sni", Type::String);
    s.add_field("http.method", Type::String);
    s.add_field("http.host", Type::String);
    s.add_field("http.path", Type::String);
    s.add_field("http.path.segments.*", Type::String);
    s.add_field("http.path.segments.len", Type::Int);
    s.add_field("http.headers.*", Type::String);
    s.add_field("net.dst.port", Type::Int);
    s.add_field("net.src.ip", Type::IpAddr);
    s
}

// setup matchers, which be added from priority 100 with descending order
fn setup_matchers(r: &mut Router, data: &TestData) {
    let mut pri = 100;
    for v in &data.rules {
        let id = Uuid::new_v4();
        let _ = r.add_matcher(pri, id, v.as_str().unwrap());
        pri -= 1;
    }
}

// mock contexts with field values passed in from json data
fn setup_context(ctx: &mut Context, data: &TestData, test_match: bool) {
    let values = if test_match {
        &data.match_values
    } else {
        &data.not_match_values
    };
    for (i, v) in values.iter().enumerate() {
        match v {
            serde_json::Value::String(s) => {
                ctx.add_value(i, Value::String(s.to_string()));
            }
            serde_json::Value::Number(n) => {
                ctx.add_value(i, Value::Int(n.as_i64().unwrap()));
            }
            serde_json::Value::Array(l) => {
                ctx.add_value(
                    i,
                    Value::IpAddr(IpAddr::V4(
                        Ipv4Addr::from_str(l[0].as_str().unwrap()).unwrap(),
                    )),
                );
            }
            _ => panic!("incorrect data type"),
        }
    }
}

fn router_match(router: &Router, ctx: &mut Context) -> bool {
    router.execute(ctx)
}

fn criterion_benchmark(c: &mut Criterion) {
    let data = prepare_data();
    let s = setup_schema();
    let mut r = Router::new(&s);
    setup_matchers(&mut r, &data);

    let mut ctx = Context::new(r.fields.len());
    setup_context(&mut ctx, &data, true);
    c.bench_function("route match all", |b| {
        b.iter(|| router_match(black_box(&r), black_box(&mut ctx)))
    });

    let mut ctx = Context::new(r.fields.len());
    setup_context(&mut ctx, &data, false);
    c.bench_function("route mismatch all", |b| {
        b.iter(|| router_match(black_box(&r), black_box(&mut ctx)))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
