use std::io::Write;

use rustc_hash::FxHashMap;
use tracing::field::Visit;
use tracing_subscriber::Layer;

pub struct LogLayer;

impl<S> Layer<S> for LogLayer
where
    S: tracing::Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut visitor = Visitor::default();
        event.record(&mut visitor);

        let lvl = event.metadata().level().to_string();

        if let Some(span) = ctx.event_span(event) {
            let opt_span_id = span
                .extensions()
                .get::<tracing_opentelemetry::OtelData>()
                .and_then(|otd| otd.builder.span_id);
            let opt_trace_id = span.scope().last().and_then(|root_span| {
                root_span
                    .extensions()
                    .get::<tracing_opentelemetry::OtelData>()
                    .and_then(|otd| otd.builder.trace_id)
            });

            if let Some((trace_id, span_id)) = opt_trace_id.zip(opt_span_id) {
                visitor.print(lvl, trace_id.to_string(), span_id.to_string());
                return;
            }
        }
        visitor.print(lvl, "".into(), "".into());
    }
}

#[derive(Default)]
struct Visitor {
    msg: String,
    attrs: FxHashMap<String, String>,
}

impl Visit for Visitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.msg = format!("{:?}", value);
        } else {
            self.attrs
                .insert(field.name().to_string(), format!("{:?}", value));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.msg = value.into();
        } else {
            self.attrs
                .insert(field.name().to_string(), value.to_string());
        }
    }
}

#[derive(serde::Serialize)]
struct Event {
    #[serde(rename = "msg")]
    message: String,
    #[serde(rename = "lvl")]
    level: String,
    #[serde(rename = "ts")]
    timestamp: i64,
    #[serde(skip_serializing_if = "String::is_empty")]
    trace_id: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    span_id: String,
    #[serde(rename = "attrs")]
    #[serde(skip_serializing_if = "FxHashMap::is_empty")]
    attributes: FxHashMap<String, String>,
}

impl Visitor {
    fn print(self, level: String, trace_id: String, span_id: String) {
        let level = level.to_lowercase();
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let event = Event {
            message: self.msg,
            level,
            timestamp,
            trace_id,
            span_id,
            attributes: self.attrs,
        };
        let mut buffer = Vec::with_capacity(512);
        if serde_json::to_writer(&mut buffer, &event).is_ok() {
            let mut stdout = std::io::stdout().lock();
            let _ = stdout.write_all(&buffer);
            let _ = stdout.write_all(b"\n");
        } else {
            println!("{}", serde_json::to_string(&event).unwrap());
        }
    }
}
