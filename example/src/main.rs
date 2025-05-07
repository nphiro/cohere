use opentelemetry::{
    Context, KeyValue, global,
    trace::{SpanKind, TraceContextExt, Tracer},
};

use cohere::{cmlt, env};

#[derive(serde::Deserialize, Debug, Default)]
struct Config {
    url: String,
}

fn main() {
    let _shutdown = cmlt::init();

    let mut config = Config::default();

    env::parse(&mut config);

    println!("URL: {}", config.url);

    let tracer = global::tracer("cohere");

    let span = tracer
        .span_builder("my_span")
        .with_kind(SpanKind::Client)
        .with_attributes([
            KeyValue::new("http.method", "GET"),
            KeyValue::new("http.url", "https://example.com"),
        ])
        .start(&tracer);

    let cx = Context::current_with_span(span);
    let span = cx.span();
    span.set_attribute(KeyValue::new("http.status_code", 200));
    span.end();
}
