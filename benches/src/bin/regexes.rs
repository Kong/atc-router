use atc_router::ast::Type;
use atc_router::context::Context;
use atc_router::router::Router;
use atc_router::schema::Schema;
use rand::prelude::*;
use std::time::Instant;
use uuid::Uuid;

fn main() {
    let mut schema = Schema::default();

    schema.add_field("http.host", Type::String);
    schema.add_field("http.path", Type::String);
    schema.add_field("net.protocol", Type::String);

    let mut router = Router::new(&schema);

    for i in 0..10000 {
        router.add_matcher(0, Uuid::new_v4(), &format!(r##"
               (http.host == "service.a.api.v1.mockroute.a.mockpath"
                || http.host == "dataplane.kong.benchmark.svc.cluster.local"
                || http.host == "dataplane.kong.benchmark.svc")
               && http.path ~ r#"^/(?<all>service-{}/api/v1/mockroute-0/[^/]+/(?<part>mockpath/?))$"#
               && net.protocol == "http"
           "##, i)).unwrap();
    }

    let now = Instant::now();

    for _ in 0..10000 {
        let mut rng = SmallRng::from_entropy();
        let service_n: u32 = rng.gen_range(0..10000);
        let mut ctx = Context::new(&router);
        ctx.add_value(
            "http.host",
            "dataplane.kong.benchmark.svc.cluster.local"
                .to_string()
                .into(),
        );
        ctx.add_value(
            "http.path",
            format!("/service-{}/api/v1/mockroute-0/foobar/mockpath/", service_n).into(),
        );
        ctx.add_value("net.protocol", "http".to_string().into());
        assert!(router.execute(&mut ctx));
    }

    println!("10000 iters, {} msec", now.elapsed().as_millis());
}
