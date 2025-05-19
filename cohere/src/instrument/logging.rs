use std::io::Write;

use rustc_hash::FxHashMap;
use serde_json::Value;
use tracing::field::Visit;
use tracing_opentelemetry::OtelData;
use tracing_subscriber::Layer;

pub struct LogLayer;

impl<S> Layer<S> for LogLayer
where
    S: tracing::Subscriber + for<'lookup> tracing_subscriber::registry::LookupSpan<'lookup>,
{
    fn on_event(&self, event: &tracing::Event<'_>, ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut visitor = Visitor::default();
        event.record(&mut visitor);

        let lvl = event.metadata().level();

        if let Some(span) = ctx.event_span(event) {
            let opt_span_id = span
                .extensions()
                .get::<OtelData>()
                .and_then(|otd| otd.builder.span_id);
            let opt_trace_id = span.scope().last().and_then(|root_span| {
                root_span
                    .extensions()
                    .get::<OtelData>()
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
    attrs: FxHashMap<String, Value>,
}

impl Visit for Visitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        self.attrs.insert(
            field.name().to_string(),
            Value::String(format!("{:?}", value)),
        );
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.attrs
            .insert(field.name().to_string(), Value::Number(value.into()));
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.attrs
            .insert(field.name().to_string(), Value::Number(value.into()));
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.attrs
            .insert(field.name().to_string(), Value::String(value.into()));
    }

    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.attrs
            .insert(field.name().to_string(), Value::Bool(value));
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
    attributes: FxHashMap<String, Value>,
}

impl Visitor {
    fn print(mut self, level: &tracing::Level, trace_id: String, span_id: String) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let message: String = self
            .attrs
            .remove("message")
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| "-".into());
        let level = level.to_string().to_lowercase();
        let event = Event {
            message,
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
