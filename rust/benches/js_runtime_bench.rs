//! Benchmarks for JS runtime performance

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rust_lib_omniread::js_engine::JsRuntime;

const HTML: &str = r#"<html><body><div id="c"><h1 class="t">Hello</h1><ul><li>A</li><li>B</li></ul></div></body></html>"#;

fn bench_runtime_creation(c: &mut Criterion) {
    c.bench_function("runtime_creation", |b| {
        b.iter(|| black_box(JsRuntime::new()))
    });
}

fn bench_simple_query(c: &mut Criterion) {
    c.bench_function("simple_query", |b| {
        b.iter(|| {
            let mut rt = JsRuntime::new();
            rt.execute(black_box(HTML), black_box(r#"text($(".t"))"#), None)
                .unwrap()
        })
    });
}

fn bench_complex_query(c: &mut Criterion) {
    c.bench_function("complex_query", |b| {
        b.iter(|| {
            let mut rt = JsRuntime::new();
            rt.execute(black_box(HTML), black_box(r#"$$("li").length"#), None)
                .unwrap()
        })
    });
}

criterion_group!(
    benches,
    bench_runtime_creation,
    bench_simple_query,
    bench_complex_query
);
criterion_main!(benches);
