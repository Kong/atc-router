use atc_router::parser::parse;
use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

fn parsing(c: &mut Criterion) {
    const MAX: usize = 1000;

    let mut g = c.benchmark_group("num alternations");
    for n in [1, 10, 100, 500, 700, 900, MAX] {
        let mut expr_str = String::new();
        for i in 0..n {
            if i != 0 {
                expr_str.push_str(" || ");
            }
            expr_str.push_str(r#"http.path == "/abc""#);
        }
        g.throughput(Throughput::ElementsAndBytes {
            elements: n as u64,
            bytes: expr_str.len() as u64,
        });
        g.bench_with_input(
            BenchmarkId::from_parameter(n),
            &expr_str[..],
            |b, expr_str| {
                b.iter_with_large_drop(|| parse(expr_str).unwrap());
            },
        );
    }
    g.finish();

    let mut g = c.benchmark_group("string length");
    for n in [1, 10, 100, 500, 700, 900, MAX] {
        let expr_str = format!("http.path == \"{}\"", "/a".repeat(n));
        g.throughput(Throughput::ElementsAndBytes {
            elements: n as u64,
            bytes: expr_str.len() as u64,
        });
        g.bench_with_input(
            BenchmarkId::from_parameter(n),
            &expr_str[..],
            |b, expr_str| {
                b.iter_with_large_drop(|| parse(expr_str).unwrap());
            },
        );
    }
}

criterion_group!(benches, parsing);
criterion_main!(benches);
